use database::NodeKind;
use std::path::PathBuf;

use common::request::DeleteMode;
use common::response::version::{VersionIdView, VersionListItem, VersionView};
use semver::Version;
use uuid::Uuid;

use crate::infrastructure::pdf::{PdfUpload, content_hash_rel_path};
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity_or, require_entity_visible};
use crate::logic::error::LogicError;
use crate::logic::pagination::page_offset;
use crate::logic::search::sync_article_best_effort;
use crate::repository::delete::{
    clear_soft_deleted_flag, delete_version as delete_version_node, soft_delete_version,
};
use crate::repository::role::{
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE_HARD, PERMISSION_VERSION_DELETE_SOFT,
    PERMISSION_VERSION_READ, PERMISSION_VERSION_UNDELETE_SOFT, PERMISSION_VERSION_UPDATE,
};
use crate::repository::version::{
    VersionDraft, content_hash_owner, create_version as create_version_node, parent_article_of,
    read_version as read_version_node, update_version as update_version_node, versions_of,
};

pub fn validate_version(raw: &str) -> Result<String, LogicError> {
    let trimmed = raw.trim();
    Version::parse(trimmed)
        .map(|version| version.to_string())
        .map_err(|_| LogicError::bad_request("invalid version number"))
}

pub fn pdf_final_path(storage_path: &str, hash: &str) -> Option<PathBuf> {
    content_hash_rel_path(hash).map(|relative| std::path::Path::new(storage_path).join(relative))
}

pub async fn place_uploaded_pdf(
    state: &AppState,
    upload: PdfUpload,
) -> Result<PdfUpload, LogicError> {
    let final_path = pdf_final_path(state.configurator.pdf_storage_path(), &upload.hash)
        .ok_or_else(|| LogicError::internal("invalid content hash"))?;
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            LogicError::internal(format!("failed to create pdf storage dir: {error}"))
        })?;
    }
    upload
        .place(final_path)
        .await
        .map_err(|error| LogicError::internal(format!("failed to place pdf: {error}")))
}

pub async fn remove_orphaned_pdfs(state: &AppState, hashes: &[String]) {
    for hash in hashes {
        let Some(path) = pdf_final_path(state.configurator.pdf_storage_path(), hash) else {
            continue;
        };
        if let Err(error) = tokio::fs::remove_file(&path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %path.display(),
                error = %error,
                "failed to remove orphaned pdf"
            );
        }
    }
}

pub(crate) async fn reject_duplicate_content_hash(
    state: &AppState,
    hash: &str,
) -> Result<(), LogicError> {
    let Some(owner) = content_hash_owner(&state.database, hash)? else {
        return Ok(());
    };
    let owned_version = read_version_node(&state.database, &owner.version_id)?
        .map(|entry| entry.version_number)
        .unwrap_or_default();
    Err(LogicError::bad_request(format!(
        "identical PDF already exists (version {owned_version})"
    )))
}

pub async fn create_version(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    raw_version: &str,
    raw_note: &str,
    upload: PdfUpload,
) -> Result<String, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_CREATE,
        EntityRef::Article(article_id),
    )?;

    let version_number = validate_version(raw_version)?;
    let note = validate_note(raw_note, state.configurator.max_version_note_chars())?;

    let hash = upload.hash.clone();
    reject_duplicate_content_hash(state, &hash).await?;

    let upload = place_uploaded_pdf(state, upload).await?;
    let version_id = Uuid::now_v7().to_string();
    let draft = VersionDraft {
        version_id: version_id.clone(),
        version_number,
        content_hash: hash,
        note,
    };

    match create_version_node(&state.database, article_id, &draft) {
        Ok(()) => {
            upload.keep_final();
            sync_article_best_effort(state, article_id).await;
            Ok(version_id)
        }
        Err(error) => {
            drop(upload);
            Err(error.into())
        }
    }
}

pub fn read_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    article_id: Option<&str>,
) -> Result<VersionView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_READ,
        EntityRef::Version(version_id),
    )?;
    let parent_article = parent_article_of(&state.database, version_id)?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    if let Some(expected_article) = article_id
        && parent_article != expected_article
    {
        return Err(LogicError::not_found("version not found"));
    }

    let entry = read_version_node(&state.database, version_id)?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    require_entity_visible(state, actor_id, EntityRef::Version(version_id))?;

    let created_at = common::time::uuidv7_secs_or_zero(version_id);
    let view = VersionView {
        id: version_id.to_string(),
        version: entry.version_number,
        created_at,
        note: entry.note,
    };
    Ok(view)
}

pub fn read_versions(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    page: u64,
    limit: u64,
) -> Result<common::response::ListPage<VersionListItem>, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_READ,
        EntityRef::Article(article_id),
    )?;
    let total = crate::repository::version::count_versions_of(&state.database, article_id)?;
    let offset = page_offset(page, limit);
    let (items, has_next) = versions_of(&state.database, article_id, limit, offset)?;
    let items: Vec<VersionListItem> = items
        .into_iter()
        .map(|item| VersionListItem {
            id: item.id.clone(),
            version: item.version_number,
        })
        .collect();
    Ok(common::response::ListPage {
        items,
        has_next,
        total,
    })
}

pub fn update_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    raw_note: &str,
) -> Result<VersionIdView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_UPDATE,
        EntityRef::Version(version_id),
    )?;
    let note = validate_note(raw_note, state.configurator.max_version_note_chars())?;
    update_version_node(&state.database, version_id, &note)?;
    Ok(VersionIdView {
        version_id: version_id.to_string(),
    })
}

pub async fn delete_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    mode: Option<DeleteMode>,
) -> Result<VersionIdView, LogicError> {
    match mode {
        Some(DeleteMode::Soft) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_VERSION_DELETE_SOFT,
                EntityRef::Version(version_id),
            )?;
            let parent_article = parent_article_of(&state.database, version_id)?;
            let already_deleted = crate::repository::delete::is_soft_deleted(
                &state.database,
                NodeKind::Version,
                version_id,
            )?;
            if already_deleted {
                return Err(LogicError::bad_request("already soft-deleted"));
            }
            soft_delete_version(&state.database, version_id)?;
            if let Some(parent_article) = parent_article {
                crate::repository::delete::refresh_live_latest_version(
                    &state.database,
                    &parent_article,
                )?;
                sync_article_best_effort(state, &parent_article).await;
            }
            Ok(VersionIdView {
                version_id: version_id.to_string(),
            })
        }
        Some(DeleteMode::Hard) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_VERSION_DELETE_HARD,
                EntityRef::Version(version_id),
            )?;
            let parent_article = parent_article_of(&state.database, version_id)?;
            let outcome = delete_version_node(&state.database, version_id)?;
            remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
            if let Some(parent_article) = parent_article {
                sync_article_best_effort(state, &parent_article).await;
            }
            Ok(VersionIdView {
                version_id: version_id.to_string(),
            })
        }
        Some(DeleteMode::Transfer) | None => Err(LogicError::bad_request(
            "version delete only supports mode \"soft\" or \"hard\"",
        )),
    }
}

pub async fn undelete_soft_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
) -> Result<VersionIdView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_UNDELETE_SOFT,
        EntityRef::Version(version_id),
    )?;
    let hidden =
        crate::repository::delete::is_soft_deleted(&state.database, NodeKind::Version, version_id)?;
    if !hidden {
        return Err(LogicError::bad_request("not soft-deleted"));
    }
    clear_soft_deleted_flag(&state.database, version_id)?;
    if let Some(parent_article) = parent_article_of(&state.database, version_id)? {
        crate::repository::delete::refresh_live_latest_version(&state.database, &parent_article)?;
        sync_article_best_effort(state, &parent_article).await;
    }
    Ok(VersionIdView {
        version_id: version_id.to_string(),
    })
}

pub(crate) fn validate_note(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    crate::logic::error::validate_ascii_text_capped(raw, max_chars, true)
}
