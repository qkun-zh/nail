use std::path::PathBuf;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity_or, require_entity_visible};
use crate::logic::error::LogicError;
use crate::logic::session::{hash_canonical_token, hash_token};
use crate::logic::version::pdf_final_path;
use crate::repository::cache::DownloadTokenEntry;
use crate::repository::role::PERMISSION_VERSION_READ;
use crate::repository::version::{parent_article_of, read_version};

pub async fn resolve_version_pdf_path(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    version_id: &str,
) -> Result<PathBuf, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_READ,
        EntityRef::Version(version_id),
    )
    .await?;
    let parent = parent_article_of(&state.database, version_id)
        .await?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    if parent != article_id {
        return Err(LogicError::not_found("version not found"));
    }
    let entry = read_version(&state.database, version_id)
        .await?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    require_entity_visible(state, actor_id, EntityRef::Version(version_id)).await?;
    pdf_final_path(state.configurator.pdf_storage_path(), &entry.content_hash)
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
    let key = hash_canonical_token(&token)?;
    state.cache.download.insert(
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
    let key = hash_token(
        raw_token,
        LogicError::bad_request("invalid or expired download token"),
    )?;
    let entry = state
        .cache
        .download
        .read(&key)
        .ok_or_else(|| LogicError::bad_request("invalid or expired download token"))?;
    if entry.user_id != actor_id {
        return Err(LogicError::bad_request(
            "download token is bound to another account",
        ));
    }
    if entry.version_id != version_id {
        return Err(LogicError::not_found("version not found"));
    }
    let consumed = state
        .cache
        .download
        .consume_if(&key, |entry| entry.user_id == actor_id);
    let Some(_consumed) = consumed else {
        return Err(LogicError::bad_request("invalid or expired download token"));
    };
    resolve_version_pdf_path(state, actor_id, article_id, version_id).await
}
