use std::path::PathBuf;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize_or, require_visible_if_soft_deleted};
use crate::logic::error::{LogicError, database_error};
use crate::logic::session::normalize_token;
use crate::logic::version::pdf_final_path;
use crate::repository::authorization::Resource;
use crate::repository::cache::{DownloadTokenEntry, token_key};
use crate::repository::role::{PERMISSION_VERSION_READ, PERMISSION_VERSION_UNDELETE_SOFT};
use crate::repository::version::{parent_article_of, read_version};

pub async fn resolve_version_pdf_path(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    version_id: &str,
) -> Result<PathBuf, LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_VERSION_READ,
        &Resource::Version(version_id.to_string()),
        "version content not found",
    )
    .await?;
    let parent = parent_article_of(&state.graph, version_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("version content not found"))?;
    if parent != article_id {
        return Err(LogicError::not_found("version content not found"));
    }
    let entry = read_version(&state.graph, version_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("version content not found"))?;
    require_visible_if_soft_deleted(
        state,
        actor_id,
        crate::repository::schema::ENTITY_TYPE_VERSION,
        version_id,
        PERMISSION_VERSION_UNDELETE_SOFT,
        &Resource::Version(version_id.to_string()),
        "version content not found",
    )
    .await?;
    pdf_final_path(&state.config.server.pdf_storage_path, &entry.content_hash)
        .ok_or_else(|| LogicError::internal("invalid content hash"))
}

pub async fn mint_download_token(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    version_id: &str,
) -> Result<String, LogicError> {
    resolve_version_pdf_path(state, actor_id, article_id, version_id).await?;

    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash download token: {error}")))?;
    state.caches.download.insert(
        &key,
        DownloadTokenEntry {
            version_id: version_id.to_string(),
            user_id: actor_id.to_string(),
        },
    );
    Ok(format!(
        "/api/article/{article_id}/version/{version_id}/content/read?token={token}"
    ))
}

pub async fn consume_download_token(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    version_id: &str,
    raw_token: &str,
) -> Result<PathBuf, LogicError> {
    let token = normalize_token(raw_token)
        .ok_or_else(|| LogicError::bad_request("invalid or expired download token"))?;
    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash download token: {error}")))?;
    let entry = state
        .caches
        .download
        .read(&key)
        .ok_or_else(|| LogicError::bad_request("invalid or expired download token"))?;
    if entry.user_id != actor_id {
        return Err(LogicError::bad_request(
            "download token is bound to another account",
        ));
    }
    if entry.version_id != version_id {
        return Err(LogicError::not_found("version content not found"));
    }
    let consumed = state
        .caches
        .download
        .consume_if(&key, |entry| entry.user_id == actor_id);
    let Some(_consumed) = consumed else {
        return Err(LogicError::bad_request("invalid or expired download token"));
    };
    resolve_version_pdf_path(state, actor_id, article_id, version_id).await
}
