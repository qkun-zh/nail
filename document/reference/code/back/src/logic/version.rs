
use common::text::validate_ascii_text;
use semver::Version;
use uuid::Uuid;

use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE, PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UPDATE, authorize, authorize_or,
};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::other::pdf::PdfUpload;
use crate::repo;
use crate::repo::article::CreateVersionError;

pub(crate) fn validate_version(raw_version: &str) -> Result<String, LogicError> {
    let trimmed = raw_version.trim();
    if trimmed.is_empty() {
        return Err(LogicError::bad_request("version cannot be empty"));
    }
    let parsed = Version::parse(trimmed)
        .map_err(|e| LogicError::bad_request(format!("invalid version: {}", e)))?;
    Ok(parsed.to_string())
}

pub async fn resolve_version_of_comment(
    state: &AppState,
    comment_id: &str,
) -> Result<Option<String>, LogicError> {
    repo::authorization::find_version_id_by_comment(&state.db, comment_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))
}

pub async fn resolve_article_id_of_version(
    state: &AppState,
    version_id: &str,
) -> Result<Option<String>, LogicError> {
    repo::article::find_article_id_by_version(&state.db, version_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))
}

pub async fn get_public_pdf_path(
    state: &AppState,
    article_id: &str,
    version_id: &str,
) -> Result<String, LogicError> {
    let belongs = repo::article::version_belongs_to_article(&state.db, version_id, article_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    if !belongs {
        return Err(LogicError::not_found("article version not found"));
    }
    let version_entry = repo::article::read_version(&state.db, version_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("article version not found"))?;
    let rel = content_hash_to_rel_path(&version_entry.content_hash)?;
    Ok(format!("{}/{}", state.config.server.pdf_storage_path, rel))
}

pub(crate) fn validate_content_hash(hash: &str) -> Result<(), LogicError> {
    let valid = hash.len() == 32
        && hash
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if !valid {
        return Err(LogicError::bad_request("invalid content hash"));
    }
    Ok(())
}

pub(crate) fn content_hash_to_rel_path(hash: &str) -> Result<String, LogicError> {
    validate_content_hash(hash)?;
    match crate::repo::util::content_hash_rel_path(hash) {
        Some(rel) => Ok(rel),
        None => Err(LogicError::bad_request("invalid content hash")),
    }
}

pub async fn handle_create_version(
    state: &AppState,
    session_token: &str,
    article_id: &str,
    raw_version: &str,
    note: &str,
    upload: PdfUpload,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_VERSION_CREATE,
        &Resource::Article(article_id.to_string()),
        "article not found",
    )
    .await?;

    let version = validate_version(raw_version)?;
    let note = validate_ascii_text(
        note,
        state.config.server.max_version_note_chars as usize,
        true,
    )
    .map_err(|e| LogicError::bad_request(e.to_string()))?;

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

    let rel = content_hash_to_rel_path(&hash)?;
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

    if let Err(e) =
        repo::article::create_version(&state.db, article_id, &version_id, &version, &hash, &note)
            .await
    {
        let referenced = match repo::article::find_version_by_hash(&state.db, &hash).await {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(qe) => {
                tracing::warn!(
                    error = %qe,
                    "failed to recheck pdf reference after create_version rejected; keeping file"
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
            CreateVersionError::ArticleNotFound => LogicError::not_found("article not found"),
            CreateVersionError::VersionNotGreater => LogicError::bad_request(
                "new version must be strictly greater than the latest version",
            ),
            CreateVersionError::InvalidVersion => LogicError::bad_request("invalid version number"),
            CreateVersionError::ContentHashExists => {
                LogicError::bad_request("identical PDF already exists")
            }
            CreateVersionError::Db(err) => {
                LogicError::internal(format!("failed to create version: {err}"))
            }
        });
    }

    upload.keep_final();

    if let Err(e) = repo::search::sync_article(&state.search, &state.db, article_id).await {
        tracing::warn!(article_id = %article_id, error = %e, "search index sync after create_version failed");
    }
    Ok(version_id)
}

pub async fn handle_read_article_versions(
    state: &AppState,
    article_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<serde_json::Value>, u64), LogicError> {
    let (version_list, total) =
        repo::article::read_article_versions(&state.db, article_id, limit, offset)
            .await
            .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    Ok((version_list, total))
}

pub async fn handle_read_version(
    state: &AppState,
    session_token: &str,
    version_id: &str,
    article_id: Option<&str>,
) -> Result<Option<repo::article::VersionEntry>, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_VERSION_READ,
        &Resource::Version(version_id.to_string()),
        "version not found",
    )
    .await?;
    let entry = repo::article::read_version(&state.db, version_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    let Some(entry) = entry else {
        return Ok(None);
    };
    if let Some(aid) = article_id {
        let belongs = repo::article::version_belongs_to_article(&state.db, version_id, aid)
            .await
            .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
        if !belongs {
            return Ok(None);
        }
    }
    Ok(Some(entry))
}

pub(crate) async fn cleanup_pdf_files_by_hashes(state: &AppState, hashes: &[String]) {
    for hash in hashes {
        let referenced = repo::article::find_version_by_hash(&state.db, hash)
            .await
            .ok()
            .flatten()
            .is_some();
        if referenced {
            continue;
        }
        let Ok(rel) = content_hash_to_rel_path(hash) else {
            continue;
        };
        let path = format!("{}/{}", state.config.server.pdf_storage_path, rel);
        if let Err(e) = tokio::fs::remove_file(path).await {
            tracing::warn!(hash = %hash, error = %e, "failed to remove orphan pdf file");
        }
    }
}

pub async fn handle_update_version_note(
    state: &AppState,
    session_token: &str,
    version_id: &str,
    raw_note: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(
        state,
        &user_id,
        PERMISSION_VERSION_UPDATE,
        &Resource::Version(version_id.to_string()),
    )
    .await?;
    let note = validate_ascii_text(
        raw_note,
        state.config.server.max_version_note_chars as usize,
        true,
    )
    .map_err(|e| LogicError::bad_request(e.to_string()))?;
    repo::article::update_version_note(&state.db, version_id, &note)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("version not found"))
}

pub async fn handle_hard_delete_version(
    state: &AppState,
    session_token: &str,
    version_id: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(
        state,
        &user_id,
        PERMISSION_VERSION_DELETE,
        &Resource::Version(version_id.to_string()),
    )
    .await?;
    let outcome = repo::hard_delete::hard_delete_version(&state.db, version_id)
        .await
        .map_err(|e| LogicError::internal(format!("failed to delete version: {e}")))?;
    cleanup_pdf_files_by_hashes(state, &outcome.removed_pdf_hashes).await;
    Ok(())
}
