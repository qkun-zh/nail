use std::path::PathBuf;

use nail_common::request::DeleteMode;
use nail_common::response::version::{
    VersionIdView, VersionListItem, VersionListPage, VersionView,
};
use semver::Version;
use uuid::Uuid;

use crate::infrastructure::pdf::{PdfUpload, content_hash_rel_path};
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity_or, require_entity_visible};
use crate::logic::error::{LogicError, database_error};
use crate::logic::search::sync_article_best_effort;
use crate::repository::delete::{
    clear_soft_deleted_flag, delete_version as delete_version_node, soft_delete_version,
};
use crate::repository::role::{
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE_HARD, PERMISSION_VERSION_DELETE_SOFT,
    PERMISSION_VERSION_READ, PERMISSION_VERSION_UNDELETE_SOFT, PERMISSION_VERSION_UPDATE,
};
use crate::repository::version::{
    CreateVersionError, VersionDraft, content_hash_owner, create_version as create_version_node,
    parent_article_of, read_version as read_version_node, update_version as update_version_node,
    versions_of,
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
    let final_path = pdf_final_path(&state.config.server.pdf_storage_path, &upload.hash)
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
        let Some(path) = pdf_final_path(&state.config.server.pdf_storage_path, hash) else {
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
    )
    .await?;

    let version_number = validate_version(raw_version)?;
    let note = validate_note(raw_note, state.config.server.max_version_note_chars)?;

    let hash = upload.hash.clone();
    if let Some(owner) = content_hash_owner(&state.graph, &hash)
        .await
        .map_err(database_error)?
    {
        let owned_version = read_version_node(&state.graph, &owner.version_id)
            .await
            .map_err(database_error)?
            .map(|entry| entry.version_number)
            .unwrap_or_default();
        return Err(LogicError::bad_request(format!(
            "identical PDF already exists (version {owned_version})"
        )));
    }

    let upload = place_uploaded_pdf(state, upload).await?;
    let version_id = Uuid::now_v7().to_string();
    let draft = VersionDraft {
        version_id: version_id.clone(),
        version_number,
        content_hash: hash,
        note,
    };

    match create_version_node(&state.graph, article_id, &draft).await {
        Ok(()) => {
            upload.keep_final();
            sync_article_best_effort(state, article_id).await;
            Ok(version_id)
        }
        Err(error) => {
            drop(upload);
            Err(map_create_version_error(error))
        }
    }
}

pub async fn read_version(
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
    )
    .await?;
    let parent_article = parent_article_of(&state.graph, version_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    if let Some(expected_article) = article_id
        && parent_article != expected_article
    {
        return Err(LogicError::not_found("version not found"));
    }

    let entry = read_version_node(&state.graph, version_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    require_entity_visible(state, actor_id, EntityRef::Version(version_id)).await?;

    let created_at = nail_common::time::uuidv7_timestamp_secs(version_id).unwrap_or(0);
    let view = VersionView {
        id: version_id.to_string(),
        version: entry.version_number,
        created_at,
        note: entry.note,
    };
    Ok(view)
}

pub async fn read_versions(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    page: u64,
    limit: u64,
) -> Result<VersionListPage, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_VERSION_READ,
        EntityRef::Article(article_id),
    )
    .await?;
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, has_next) = versions_of(&state.graph, article_id, limit, offset)
        .await
        .map_err(database_error)?;
    let version_list: Vec<VersionListItem> = items
        .into_iter()
        .map(|item| VersionListItem {
            id: item.id.clone(),
            version: item.version_number,
        })
        .collect();
    Ok(VersionListPage {
        version_list,
        page,
        has_next,
    })
}

pub async fn update_version(
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
    )
    .await?;
    let note = validate_note(raw_note, state.config.server.max_version_note_chars)?;
    update_version_node(&state.graph, version_id, &note)
        .await
        .map_err(database_error)?;
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
            )
            .await?;
            let parent_article = parent_article_of(&state.graph, version_id)
                .await
                .map_err(database_error)?;
            let already_deleted =
                crate::repository::delete::is_soft_deleted(&state.graph, "version", version_id)
                    .await
                    .map_err(database_error)?;
            if already_deleted {
                return Err(LogicError::bad_request("already soft-deleted"));
            }
            soft_delete_version(&state.graph, version_id)
                .await
                .map_err(database_error)?;
            if let Some(parent_article) = parent_article {
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
            )
            .await?;
            let parent_article = parent_article_of(&state.graph, version_id)
                .await
                .map_err(database_error)?;
            let outcome = delete_version_node(&state.graph, version_id)
                .await
                .map_err(database_error)?;
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
    )
    .await?;
    let hidden = crate::repository::delete::is_soft_deleted(&state.graph, "version", version_id)
        .await
        .map_err(database_error)?;
    if !hidden {
        return Err(LogicError::bad_request("not soft-deleted"));
    }
    clear_soft_deleted_flag(&state.graph, version_id)
        .await
        .map_err(database_error)?;
    if let Some(parent_article) = parent_article_of(&state.graph, version_id)
        .await
        .map_err(database_error)?
    {
        sync_article_best_effort(state, &parent_article).await;
    }
    Ok(VersionIdView {
        version_id: version_id.to_string(),
    })
}

pub(crate) fn validate_note(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn map_create_version_error(error: CreateVersionError) -> LogicError {
    match error {
        CreateVersionError::ArticleMissing => LogicError::not_found("article not found"),
        CreateVersionError::NotGreater => {
            LogicError::bad_request("new version must be strictly greater than the latest version")
        }
        CreateVersionError::InvalidNumber => LogicError::bad_request("invalid version number"),
        CreateVersionError::ContentHashTaken => {
            LogicError::bad_request("identical PDF already exists")
        }
        CreateVersionError::Db(error) => database_error(error),
    }
}
