
use common::tag::parse_hashtag_tags;
use common::text::validate_ascii_text;
use uuid::Uuid;

use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{
    PERMISSION_ARTICLE_DELETE, PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UPDATE, authorize,
    authorize_or,
};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::other::pdf::PdfUpload;
use crate::repo;
use crate::repo::article::{CreateArticleError, UpdateArticleError};

#[allow(clippy::too_many_arguments)]
pub async fn handle_create_article(
    state: &AppState,
    session_token: &str,
    title: &str,
    summary: &str,
    raw_tags: &str,
    version: &str,
    note: &str,
    upload: PdfUpload,
) -> Result<(String, String), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    crate::authorization::gate::authorize_create(
        state,
        &user_id,
        crate::authorization::gate::PERMISSION_ARTICLE_CREATE,
    )
    .await?;

    let max_title_chars = state.config.server.max_title_chars as usize;
    let title = validate_ascii_text(title, max_title_chars, false)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    let max_summary_chars = state.config.server.max_summary_chars as usize;
    let summary = validate_ascii_text(summary, max_summary_chars, true)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    let max_tags = state.config.server.max_tags_per_article;
    let tag_names = parse_hashtag_tags(raw_tags, max_tags)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    if tag_names.is_empty() {
        return Err(LogicError::bad_request("at least one tag required"));
    }
    let version = crate::logic::version::validate_version(version)?;
    let note = validate_ascii_text(
        note,
        state.config.server.max_version_note_chars as usize,
        true,
    )
    .map_err(|e| LogicError::bad_request(e.to_string()))?;

    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    let hash = upload.hash.clone();
    if let Some((vid, title)) = repo::article::find_version_by_hash(&state.db, &hash)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
    {
        return Err(LogicError::bad_request(format!(
            "identical PDF already exists (version {vid} of \"{title}\")"
        )));
    }

    let rel = crate::logic::version::content_hash_to_rel_path(&hash)?;
    let pdf_full_path = format!("{}/{}", state.config.server.pdf_storage_path, rel);

    let pdf_dir = std::path::Path::new(&pdf_full_path)
        .parent()
        .ok_or_else(|| LogicError::internal("invalid PDF path: no parent directory"))?;
    tokio::fs::create_dir_all(pdf_dir)
        .await
        .map_err(|e| LogicError::internal(format!("failed to create pdf directory: {}", e)))?;
    let upload = upload
        .place(std::path::PathBuf::from(&pdf_full_path))
        .await
        .map_err(|e| LogicError::internal(format!("failed to place pdf: {}", e)))?;

    if let Err(e) = repo::article::create_article(
        &state.db,
        &article_id,
        &user_id,
        &title,
        &summary,
        &tag_names,
        &version_id,
        &version,
        &hash,
        &note,
    )
    .await
    {
        let referenced = match repo::article::find_version_by_hash(&state.db, &hash).await {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(qe) => {
                tracing::warn!(
                    error = %qe,
                    "failed to recheck pdf reference after create_article rejected; keeping file"
                );
                true
            }
        };
        let upload = if referenced {
            upload.keep_final()
        } else {
            upload
        };
        drop(upload);
        return Err(match e {
            CreateArticleError::TitleAlreadyExists => {
                if referenced {
                    LogicError::bad_request("identical PDF already exists")
                } else {
                    LogicError::bad_request("title already exists".to_string())
                }
            }
            CreateArticleError::AuthorNotFound => {
                LogicError::unauthorized("user not found")
            }
            CreateArticleError::TagNotFound => LogicError::internal("tag not found".to_string()),
            CreateArticleError::Db(e) => {
                LogicError::internal(format!("failed to create article: {}", e))
            }
        });
    }

    upload.keep_final();

    if let Err(e) = repo::search::sync_article(&state.search, &state.db, &article_id).await {
        tracing::warn!(article_id = %article_id, error = %e, "search index sync after create failed");
    }

    Ok((article_id, version_id))
}

pub async fn handle_update_article(
    state: &AppState,
    session_token: &str,
    article_id: &str,
    title: &str,
    summary: &str,
    raw_tags: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_UPDATE,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;

    let max_title_chars = state.config.server.max_title_chars as usize;
    let title = validate_ascii_text(title, max_title_chars, false)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    let max_summary_chars = state.config.server.max_summary_chars as usize;
    let summary = validate_ascii_text(summary, max_summary_chars, true)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    let max_tags = state.config.server.max_tags_per_article;
    let tag_names = parse_hashtag_tags(raw_tags, max_tags)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    if tag_names.is_empty() {
        return Err(LogicError::bad_request("at least one tag required"));
    }

    repo::article::update_article(
        &state.db, article_id, &user_id, &title, &summary, &tag_names,
    )
    .await
    .map_err(|e| match e {
        UpdateArticleError::TitleAlreadyExists => {
            LogicError::bad_request("title already exists".to_string())
        }
        UpdateArticleError::NotFound => LogicError::not_found("article not found".to_string()),
        UpdateArticleError::TagNotFound => LogicError::internal("tag not found".to_string()),
        UpdateArticleError::Db(e) => {
            LogicError::internal(format!("failed to update article: {}", e))
        }
    })?;

    if let Err(e) = repo::search::sync_article(&state.search, &state.db, article_id).await {
        tracing::warn!(article_id = %article_id, error = %e, "search index sync after update failed");
    }
    Ok(())
}

pub async fn handle_delete_article(
    state: &AppState,
    session_token: &str,
    article_id: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_DELETE,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;

    repo::article::transfer_article_ownership(&state.db, article_id)
        .await
        .map_err(|e| match e {
            crate::repo::transfer::TargetTransferError::TargetNotFound => {
                LogicError::not_found("article not found")
            }
            crate::repo::transfer::TargetTransferError::NoRecycler => {
                LogicError::internal("no recycler available")
            }
            crate::repo::transfer::TargetTransferError::Db(e) => {
                LogicError::internal(format!("failed to transfer article ownership: {e}"))
            }
        })?;

    if let Err(e) = repo::search::sync_article(&state.search, &state.db, article_id).await {
        tracing::warn!(article_id = %article_id, error = %e, "search index sync after delete failed");
    }

    tracing::info!(
        user_id = %user_id,
        article_id = %article_id,
        "article deleted, ownership transferred to recycler"
    );

    Ok(())
}

pub async fn handle_read_articles(
    state: &AppState,
    limit: u64,
    offset: u64,
) -> Result<(Vec<serde_json::Value>, bool, u64), LogicError> {
    let total = repo::search::count_articles(&state.db)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    let limit = limit.max(1).min(state.config.server.max_search_page_size);
    let total_pages = if total == 0 { 0 } else { total.div_ceil(limit) };
    let page = offset / limit + 1;
    if page > total_pages {
        return Ok((Vec::new(), false, total));
    }
    let article_list = repo::search::list_articles_page(&state.db, limit, offset)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    let has_more = offset.saturating_add(article_list.len() as u64) < total;
    Ok((article_list, has_more, total))
}

pub async fn handle_read_article(
    state: &AppState,
    session_token: &str,
    article_id: &str,
) -> Result<serde_json::Value, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_READ,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;
    let article = repo::article::read_article(&state.db, article_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("article not found"))?;

    Ok(article)
}

pub async fn handle_get_pdf_path(
    state: &AppState,
    session_token: &str,
    article_id: &str,
    version_id: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_READ,
        &Resource::Article(article_id.to_string()),
        "article version not found",
    )
    .await?;
    crate::logic::version::get_public_pdf_path(state, article_id, version_id).await
}

pub async fn handle_hard_delete_article(
    state: &AppState,
    session_token: &str,
    article_id: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(
        state,
        &user_id,
        PERMISSION_ARTICLE_DELETE,
        &Resource::Article(article_id.to_string()),
    )
    .await?;
    let outcome = repo::hard_delete::hard_delete_article(&state.db, article_id)
        .await
        .map_err(|e| LogicError::internal(format!("failed to delete article: {e}")))?;
    crate::logic::version::cleanup_pdf_files_by_hashes(state, &outcome.removed_pdf_hashes).await;
    if let Err(e) = repo::search::rebuild_index(&state.search, &state.db).await {
        tracing::warn!(article_id = %article_id, error = %e, "search index rebuild after hard delete failed");
    }
    Ok(())
}
