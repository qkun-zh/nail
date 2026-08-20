use nail_common::request::DeleteMode;
use nail_common::response::article::{ArticleIdView, ArticleView};
use uuid::Uuid;

use crate::infrastructure::pdf::PdfUpload;
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{
    EntityRef, authorize_entity_or, authorize_global, require_entity_readable,
};
use crate::logic::error::LogicError;
use crate::logic::search::sync_article_best_effort;
use crate::logic::version::{
    place_uploaded_pdf, reject_duplicate_content_hash, remove_orphaned_pdfs, validate_note,
    validate_version,
};
use crate::repository::article::{
    ArticleDraft, ArticleUpdate, create_article as create_article_node,
    read_article as read_article_node, update_article as update_article_node,
};
use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_DELETE_HARD, PERMISSION_ARTICLE_DELETE_SOFT,
    PERMISSION_ARTICLE_DELETE_TRANSFER, PERMISSION_ARTICLE_UNDELETE_SOFT,
    PERMISSION_ARTICLE_UPDATE,
};
use crate::repository::tag::read_tag_by_name;
use crate::repository::transfer::transfer_article;
use crate::repository::version::VersionDraft;

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
    authorize_global(state, actor_id, PERMISSION_ARTICLE_CREATE).await?;
    let title = validate_title(raw_title, state.configurator.max_title_chars())?;
    let summary = validate_summary(raw_summary, state.configurator.max_summary_chars())?;
    let tags = validate_tags(state, raw_tags, state.configurator.max_tags_per_article()).await?;
    let version_number = validate_version(raw_version)?;
    let note = validate_note(raw_note, state.configurator.max_version_note_chars())?;

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

    match create_article_node(&state.database, &draft).await {
        Ok(()) => {
            upload.keep_final();
            sync_article_best_effort(state, &article_id).await;
            Ok((article_id, version_id))
        }
        Err(error) => {
            drop(upload);
            Err(error.into())
        }
    }
}

pub async fn read_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
) -> Result<ArticleView, LogicError> {
    require_entity_readable(state, actor_id, EntityRef::Article(article_id)).await?;
    let article = read_article_node(&state.database, article_id)
        .await?
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
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ARTICLE_UPDATE,
        EntityRef::Article(article_id),
    )
    .await?;
    let title = validate_title(raw_title, state.configurator.max_title_chars())?;
    let summary = validate_summary(raw_summary, state.configurator.max_summary_chars())?;
    let tags = validate_tags(state, raw_tags, state.configurator.max_tags_per_article()).await?;
    update_article_node(
        &state.database,
        article_id,
        &ArticleUpdate {
            title,
            summary,
            tags,
        },
    )
    .await?;
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
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_ARTICLE_DELETE_TRANSFER,
                EntityRef::Article(article_id),
            )
            .await?;
            transfer_article(&state.database, article_id).await?;
            sync_article_best_effort(state, article_id).await;
            Ok(ArticleIdView {
                article_id: article_id.to_string(),
            })
        }
        Some(DeleteMode::Hard) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_ARTICLE_DELETE_HARD,
                EntityRef::Article(article_id),
            )
            .await?;
            let outcome =
                crate::repository::delete::delete_article(&state.database, article_id).await?;
            remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
            sync_article_best_effort(state, article_id).await;
            Ok(ArticleIdView {
                article_id: article_id.to_string(),
            })
        }
        Some(DeleteMode::Soft) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_ARTICLE_DELETE_SOFT,
                EntityRef::Article(article_id),
            )
            .await?;
            let already_deleted =
                crate::repository::delete::is_soft_deleted(&state.database, "article", article_id)
                    .await?;
            if already_deleted {
                return Err(LogicError::bad_request("already soft-deleted"));
            }
            crate::repository::delete::soft_delete_article(&state.database, article_id).await?;
            sync_article_best_effort(state, article_id).await;
            Ok(ArticleIdView {
                article_id: article_id.to_string(),
            })
        }
        None => Err(LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\", \"soft\", or \"hard\")",
        )),
    }
}

pub async fn undelete_soft_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
) -> Result<ArticleIdView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ARTICLE_UNDELETE_SOFT,
        EntityRef::Article(article_id),
    )
    .await?;
    let hidden =
        crate::repository::delete::is_soft_deleted(&state.database, "article", article_id).await?;
    if !hidden {
        return Err(LogicError::bad_request("not soft-deleted"));
    }
    crate::repository::delete::clear_soft_deleted_flag(&state.database, article_id).await?;
    sync_article_best_effort(state, article_id).await;
    Ok(ArticleIdView {
        article_id: article_id.to_string(),
    })
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

async fn validate_tags(
    state: &AppState,
    raw: &str,
    max_tags: usize,
) -> Result<Vec<String>, LogicError> {
    let tags = nail_common::tag::parse_tags(raw, max_tags)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if tags.is_empty() {
        return Err(LogicError::bad_request("at least one tag is required"));
    }
    for name in &tags {
        if read_tag_by_name(&state.database, name).await?.is_none() {
            return Err(LogicError::bad_request(format!(
                "tag \"{name}\" does not exist"
            )));
        }
    }
    Ok(tags)
}
