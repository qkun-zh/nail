use nail_common::request::DeleteMode;
use nail_common::response::article::{ArticleIdView, ArticleView};
use uuid::Uuid;

use crate::infrastructure::pdf::PdfUpload;
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize_create, authorize_or};
use crate::logic::error::{LogicError, database_error};
use crate::logic::search::sync_article_best_effort;
use crate::logic::version::{
    place_uploaded_pdf, remove_orphaned_pdfs, validate_note, validate_version,
};
use crate::repository::article::{
    ArticleDraft, ArticleUpdate, CreateArticleError, UpdateArticleError,
    create_article as create_article_node, read_article as read_article_node,
    update_article as update_article_node,
};
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_DELETE_HARD, PERMISSION_ARTICLE_DELETE_TRANSFER,
    PERMISSION_ARTICLE_UPDATE,
};
use crate::repository::transfer::{TransferTargetError, transfer_article};
use crate::repository::version::{VersionDraft, content_hash_owner, read_version};

pub struct ArticleCreateInput<'a> {
    pub title: &'a str,
    pub summary: &'a str,
    pub tags: &'a str,
    pub version: &'a str,
    pub note: &'a str,
    pub upload: PdfUpload,
}

pub async fn create_article(
    state: &AppState,
    actor_id: &str,
    input: ArticleCreateInput<'_>,
) -> Result<(String, String), LogicError> {
    let ArticleCreateInput {
        title: raw_title,
        summary: raw_summary,
        tags: raw_tags,
        version: raw_version,
        note: raw_note,
        upload,
    } = input;
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

pub async fn read_article(state: &AppState, article_id: &str) -> Result<ArticleView, LogicError> {
    let article = read_article_node(&state.graph, article_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("article not found"))?;

    let created_at = nail_common::time::uuidv7_timestamp_secs(&article.id).unwrap_or(0);
    let view = ArticleView {
        id: article.id,
        author_id: article.author_id,
        author_name: article.author_name,
        title: article.title,
        summary: article.summary,
        created_at,
        tags: article.tags,
    };
    Ok(view)
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
                PERMISSION_ARTICLE_DELETE_TRANSFER,
                &Resource::Article(article_id.to_string()),
                "article not found",
            )
            .await?;
            transfer_article(&state.graph, article_id)
                .await
                .map_err(|error| match error {
                    TransferTargetError::TargetMissing => {
                        LogicError::not_found("article not found")
                    }
                    TransferTargetError::TargetOwnerMissing => {
                        LogicError::internal("article has no owner")
                    }
                    TransferTargetError::NoRecycler => {
                        LogicError::internal("no recycler available")
                    }
                    TransferTargetError::Db(error) => database_error(error),
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
                PERMISSION_ARTICLE_DELETE_HARD,
                &Resource::Article(article_id.to_string()),
                "article not found",
            )
            .await?;
            let outcome = crate::repository::delete::delete_article(&state.graph, article_id)
                .await
                .map_err(database_error)?;
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
        .map_err(database_error)?
    else {
        return Ok(());
    };
    let owned_version = read_version(&state.graph, &owner.version_id)
        .await
        .map_err(database_error)?
        .map(|entry| entry.version_number)
        .unwrap_or_default();
    Err(LogicError::bad_request(format!(
        "identical PDF already exists (version {owned_version} of \"{}\")",
        owner.article_title
    )))
}

fn validate_title(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        false,
    )
    .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn validate_summary(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn validate_tags(raw: &str, max_tags: usize) -> Result<Vec<String>, LogicError> {
    let tags = nail_common::tag::parse_tags(raw, max_tags)
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
        CreateArticleError::Db(error) => database_error(error),
    }
}

fn map_update_article_error(error: UpdateArticleError) -> LogicError {
    match error {
        UpdateArticleError::Missing => LogicError::not_found("article not found"),
        UpdateArticleError::TitleTaken => LogicError::bad_request("title already exists"),
        UpdateArticleError::Db(error) => database_error(error),
    }
}
