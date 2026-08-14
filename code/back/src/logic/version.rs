use std::path::PathBuf;

use nail_common::request::DeleteMode;
use nail_common::response::version::{VersionIdView, VersionListItem, VersionListPage, VersionView};
use semver::Version;
use uuid::Uuid;

use crate::infrastructure::pdf::{PdfUpload, content_hash_rel_path};
use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize_or, is_author};
use crate::repository::authorization::Resource;
use crate::logic::error::LogicError;
use crate::logic::search::sync_article_best_effort;
use crate::repository::role::{
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE, PERMISSION_VERSION_UPDATE,
};
use crate::repository::delete::delete_version as delete_version_node;
use crate::repository::version::{
    CreateVersionError, VersionDraft, content_hash_owner, create_version as create_version_node,
    parent_article_of, read_version as read_version_node, update_version as update_version_node, versions_of,
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
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| {
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
    authorize_or(
        state,
        actor_id,
        PERMISSION_VERSION_CREATE,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;

    let version_number = validate_version(raw_version)?;
    let note = validate_note(raw_note, state.config.server.max_version_note_chars)?;

    let hash = upload.hash.clone();
    if let Some(owner) = content_hash_owner(&state.graph, &hash)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
    {
        let owned_version = read_version_node(&state.graph, &owner.version_id)
            .await
            .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
            .map(|entry| entry.version_number)
            .unwrap_or_default();
        return Err(LogicError::bad_request(format!(
            "identical PDF already exists (version {owned_version} of \"{}\")",
            owner.article_title
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
    check_if_is_author: bool,
) -> Result<VersionView, LogicError> {
    let parent_article = parent_article_of(&state.graph, version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::not_found("version not found"))?;
    if let Some(expected_article) = article_id
        && parent_article != expected_article
    {
        return Err(LogicError::not_found("version not found"));
    }

    let entry = read_version_node(&state.graph, version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::not_found("version not found"))?;

    let created_at = nail_common::time::uuidv7_timestamp_secs(version_id).unwrap_or(0);
    let mut view = VersionView {
        id: version_id.to_string(),
        version: entry.version_number,
        created_at,
        note: entry.note,
        is_author: None,
    };
    if check_if_is_author {
        view.is_author = Some(is_author(state, actor_id, None, Some(version_id), None).await?);
    }
    Ok(view)
}

pub async fn read_versions(
    state: &AppState,
    article_id: &str,
    page: u64,
    limit: u64,
) -> Result<VersionListPage, LogicError> {
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, total) = versions_of(&state.graph, article_id, limit, offset)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    let version_list: Vec<VersionListItem> = items
        .into_iter()
        .map(|item| VersionListItem {
            id: item.id.clone(),
            version: item.version_number,
            created_at: nail_common::time::uuidv7_timestamp_secs(&item.id).unwrap_or(0),
        })
        .collect();
    let has_next = page < total.div_ceil(limit);
    Ok(VersionListPage {
        version_list,
        page,
        total,
        has_next,
    })
}

pub async fn update_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    raw_note: &str,
) -> Result<VersionIdView, LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_VERSION_UPDATE,
        &Resource::Version(version_id.to_string()),
        "version not found",
    )
    .await?;
    let note = validate_note(raw_note, state.config.server.max_version_note_chars)?;
    update_version_node(&state.graph, version_id, &note)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
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
    if !matches!(mode, Some(DeleteMode::Hard)) {
        return Err(LogicError::bad_request(
            "version delete only supports mode \"hard\"",
        ));
    }
    authorize_or(
        state,
        actor_id,
        PERMISSION_VERSION_DELETE,
        &Resource::Version(version_id.to_string()),
        "version not found",
    )
    .await?;
    let parent_article = parent_article_of(&state.graph, version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    let outcome = delete_version_node(&state.graph, version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
    if let Some(parent_article) = parent_article {
        sync_article_best_effort(state, &parent_article).await;
    }
    Ok(VersionIdView {
        version_id: version_id.to_string(),
    })
}

pub(crate) fn validate_note(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(raw, max_chars as usize, true)
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
        CreateVersionError::Db(error) => {
            LogicError::internal(format!("database query failed: {error}"))
        }
    }
}
