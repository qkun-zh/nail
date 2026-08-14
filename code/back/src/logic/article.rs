use nail_common::request::ArticleSearchParams;
use nail_common::request::DeleteMode;
use nail_common::response::article::{ArticleIdView, ArticleListItem, ArticleListPage, ArticleView};
use nail_common::response::search::SearchPage;
use uuid::Uuid;

use crate::infrastructure::pdf::PdfUpload;
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize_create, authorize_or, is_author};
use crate::repository::authorization::Resource;
use crate::logic::error::LogicError;
use crate::logic::search::{search_articles, sync_article_best_effort};
use crate::logic::version::{
    place_uploaded_pdf, remove_orphaned_pdfs, validate_note, validate_version,
};
use crate::repository::article::{
    ArticleDraft, ArticleUpdate, CreateArticleError, UpdateArticleError, create_article as create_article_node,
    read_article as read_article_node, read_articles as read_article_nodes, update_article as update_article_node,
};
use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_DELETE, PERMISSION_ARTICLE_UPDATE,
};
use crate::repository::transfer::{TransferTargetError, transfer_article};
use crate::repository::version::{VersionDraft, content_hash_owner, read_version};

const MAX_PAGE_SIZE: u64 = 200;
const MAX_PAGE: u64 = 10_000;

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum ArticleReadPage {
    Search(SearchPage),
    List(ArticleListPage),
}

pub async fn create_article(
    state: &AppState,
    actor_id: &str,
    raw_title: &str,
    raw_summary: &str,
    raw_tags: &str,
    raw_version: &str,
    raw_note: &str,
    upload: PdfUpload,
) -> Result<(String, String), LogicError> {
    authorize_create(state, actor_id, PERMISSION_ARTICLE_CREATE).await?;

    let title = validate_title(raw_title, state.config.server.max_title_chars)?;
    let summary = validate_summary(raw_summary, state.config.server.max_summary_chars)?;
    let tags = validate_tags(raw_tags, state.config.server.max_tags_per_article)?;
    let version_number = validate_version(raw_version)?;
    let note = validate_note(raw_note, state.config.server.max_version_note_chars)?;

    let hash = upload.hash.clone();
    reject_duplicate_content_hash(state, &hash).await?;

    let upload = place_uploaded_pdf(state, upload).await?;
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    let draft = ArticleDraft {
        article_id: article_id.clone(),
        author_id: actor_id.to_string(),
        title,
        summary,
        tags,
        first_version: VersionDraft {
            version_id: version_id.clone(),
            version_number,
            content_hash: hash,
            note,
        },
    };

    match create_article_node(&state.graph, &draft).await {
        Ok(()) => {
            upload.keep_final();
            sync_article_best_effort(state, &article_id).await;
            Ok((article_id, version_id))
        }
        Err(error) => {
            drop(upload);
            Err(map_create_article_error(error))
        }
    }
}

pub async fn read_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    check_if_is_author: bool,
) -> Result<ArticleView, LogicError> {
    let article = read_article_node(&state.graph, article_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::not_found("article not found"))?;

    let created_at = nail_common::time::uuidv7_timestamp_secs(&article.id).unwrap_or(0);
    let mut view = ArticleView {
        id: article.id,
        author_id: article.author_id,
        author_name: article.author_name,
        title: article.title,
        summary: article.summary,
        created_at,
        tags: article.tags,
        is_author: None,
    };
    if check_if_is_author {
        view.is_author = Some(is_author(state, actor_id, Some(article_id), None, None).await?);
    }
    Ok(view)
}

pub async fn read_articles(
    state: &AppState,
    params: &ArticleSearchParams,
) -> Result<ArticleReadPage, LogicError> {
    if is_search_request(params) {
        let page = search_articles(state, params).await?;
        return Ok(ArticleReadPage::Search(page));
    }

    let limit = params
        .limit
        .unwrap_or(state.config.server.search_page_size)
        .clamp(1, MAX_PAGE_SIZE);
    let page = params.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, total) = read_article_nodes(&state.graph, limit, offset)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;

    let article_list: Vec<ArticleListItem> = items
        .into_iter()
        .map(|item| ArticleListItem {
            id: item.id,
            title: item.title,
            summary: item.summary,
            author_id: item.author_id,
            author_name: item.author_name,
            tags: item.tags,
            latest_version: item.latest_version,
            latest_version_id: item.latest_version_id,
        })
        .collect();

    let total_pages = total.div_ceil(limit);
    let truncated = total_pages > state.config.server.max_search_pages;
    Ok(ArticleReadPage::List(ArticleListPage {
        article_list,
        page,
        total,
        total_pages,
        has_next: page < total_pages,
        has_prev: page > 1,
        truncated,
    }))
}

pub async fn update_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    raw_title: &str,
    raw_summary: &str,
    raw_tags: &str,
) -> Result<ArticleIdView, LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_ARTICLE_UPDATE,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;
    let title = validate_title(raw_title, state.config.server.max_title_chars)?;
    let summary = validate_summary(raw_summary, state.config.server.max_summary_chars)?;
    let tags = validate_tags(raw_tags, state.config.server.max_tags_per_article)?;
    update_article_node(
        &state.graph,
        article_id,
        &ArticleUpdate {
            title,
            summary,
            tags,
        },
    )
    .await
    .map_err(map_update_article_error)?;
    sync_article_best_effort(state, article_id).await;
    Ok(ArticleIdView {
        article_id: article_id.to_string(),
    })
}

pub async fn delete_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    mode: Option<DeleteMode>,
) -> Result<ArticleIdView, LogicError> {
    match mode {
        Some(DeleteMode::Transfer) => {
            authorize_or(
                state,
                actor_id,
                PERMISSION_ARTICLE_DELETE,
                &Resource::Article(article_id.to_string()),
                "article not found",
            )
            .await?;
            transfer_article(&state.graph, article_id)
                .await
                .map_err(|error| match error {
                    TransferTargetError::TargetMissing => LogicError::not_found("article not found"),
                    TransferTargetError::NoRecycler => {
                        LogicError::internal("no recycler available")
                    }
                    TransferTargetError::Db(error) => {
                        LogicError::internal(format!("database query failed: {error}"))
                    }
                })?;
            sync_article_best_effort(state, article_id).await;
            Ok(ArticleIdView {
                article_id: article_id.to_string(),
            })
        }
        Some(DeleteMode::Hard) => {
            authorize_or(
                state,
                actor_id,
                PERMISSION_ARTICLE_DELETE,
                &Resource::Article(article_id.to_string()),
                "article not found",
            )
            .await?;
            let outcome = crate::repository::delete::delete_article(&state.graph, article_id)
                .await
                .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
            remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
            sync_article_best_effort(state, article_id).await;
            Ok(ArticleIdView {
                article_id: article_id.to_string(),
            })
        }
        None => Err(LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
        )),
    }
}

async fn reject_duplicate_content_hash(state: &AppState, hash: &str) -> Result<(), LogicError> {
    let Some(owner) = content_hash_owner(&state.graph, hash)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
    else {
        return Ok(());
    };
    let owned_version = read_version(&state.graph, &owner.version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .map(|entry| entry.version_number)
        .unwrap_or_default();
    Err(LogicError::bad_request(format!(
        "identical PDF already exists (version {owned_version} of \"{}\")",
        owner.article_title
    )))
}

fn is_search_request(params: &ArticleSearchParams) -> bool {
    params.q.is_some()
        || params.ranges.is_some()
        || params.sort.is_some()
        || params.from.is_some()
        || params.to.is_some()
}

fn validate_title(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(raw, max_chars as usize, false)
        .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn validate_summary(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(raw, max_chars as usize, true)
        .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn validate_tags(raw: &str, max_tags: usize) -> Result<Vec<String>, LogicError> {
    let tags = nail_common::tag::parse_hashtag_tags(raw, max_tags)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if tags.is_empty() {
        return Err(LogicError::bad_request("at least one tag is required"));
    }
    Ok(tags)
}

fn map_create_article_error(error: CreateArticleError) -> LogicError {
    match error {
        CreateArticleError::AuthorMissing => LogicError::internal("author not found"),
        CreateArticleError::TitleTaken => LogicError::bad_request("title already exists"),
        CreateArticleError::ContentHashTaken => {
            LogicError::bad_request("identical PDF already exists")
        }
        CreateArticleError::Db(error) => {
            LogicError::internal(format!("database query failed: {error}"))
        }
    }
}

fn map_update_article_error(error: UpdateArticleError) -> LogicError {
    match error {
        UpdateArticleError::Missing => LogicError::not_found("article not found"),
        UpdateArticleError::TitleTaken => LogicError::bad_request("title already exists"),
        UpdateArticleError::Db(error) => {
            LogicError::internal(format!("database query failed: {error}"))
        }
    }
}
