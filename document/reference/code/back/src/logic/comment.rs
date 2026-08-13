
use common::text::validate_ascii_text;
use uuid::Uuid;

use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{
    PERMISSION_COMMENT_DELETE, PERMISSION_COMMENT_READ, PERMISSION_COMMENT_UPDATE, authorize,
    authorize_or,
};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo;
use crate::repo::comment::CreateCommentError;

fn validate_comment_body(raw: &str, max_chars: usize) -> Result<String, LogicError> {
    validate_ascii_text(raw, max_chars, true).map_err(|e| LogicError::bad_request(e.to_string()))
}

pub async fn handle_create_comment(
    state: &AppState,
    session_token: &str,
    version_id: &str,
    content: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    crate::authorization::gate::authorize_create(
        state,
        &user_id,
        crate::authorization::gate::PERMISSION_COMMENT_CREATE,
    )
    .await?;
    let content =
        validate_comment_body(content, state.config.server.max_comment_body_chars as usize)?;
    let comment_id = Uuid::now_v7().to_string();
    repo::comment::create_top_level_comment(&state.db, &comment_id, &user_id, version_id, &content)
        .await
        .map_err(|e| map_create_comment_error(e, false, 0))?;
    if let Ok(Some(article_id)) =
        crate::logic::version::resolve_article_id_of_version(state, version_id).await
    {
        if let Err(e) = repo::search::sync_article(&state.search, &state.db, &article_id).await {
            tracing::warn!(article_id = %article_id, error = %e, "search index sync after comment failed");
        }
    }
    Ok(comment_id)
}

pub async fn handle_create_reply(
    state: &AppState,
    session_token: &str,
    parent_comment_id: &str,
    content: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    crate::authorization::gate::authorize_create(
        state,
        &user_id,
        crate::authorization::gate::PERMISSION_COMMENT_CREATE,
    )
    .await?;
    let content =
        validate_comment_body(content, state.config.server.max_comment_body_chars as usize)?;
    let comment_id = Uuid::now_v7().to_string();
    repo::comment::create_reply_comment(
        &state.db,
        &comment_id,
        &user_id,
        parent_comment_id,
        &content,
        state.config.server.max_comment_tree_depth as usize,
    )
    .await
    .map_err(|e| {
        map_create_comment_error(e, true, state.config.server.max_comment_tree_depth as usize)
    })?;
    if let Ok(Some(version_id)) =
        crate::logic::version::resolve_version_of_comment(state, parent_comment_id).await
        && let Ok(Some(article_id)) =
            crate::logic::version::resolve_article_id_of_version(state, &version_id).await
    {
        if let Err(e) = repo::search::sync_article(&state.search, &state.db, &article_id).await {
            tracing::warn!(article_id = %article_id, error = %e, "search index sync after reply failed");
        }
    }
    Ok(comment_id)
}

fn map_create_comment_error(
    e: CreateCommentError,
    is_reply: bool,
    max_tree_depth: usize,
) -> LogicError {
    match e {
        CreateCommentError::TargetNotFound if is_reply => LogicError::not_found(
            "reply target not found (the parent comment may have been removed)",
        ),
        CreateCommentError::TargetNotFound => {
            LogicError::not_found("comment target not found (the version may have been removed)")
        }
        CreateCommentError::CommentIdExists => LogicError::bad_request("comment id already exists"),
        CreateCommentError::CommentTreeTooDeep => LogicError::bad_request(format!(
            "comment thread too deep (max {} reply layers)",
            max_tree_depth
        )),
        CreateCommentError::Db(err) => {
            LogicError::internal(format!("failed to create comment: {err}"))
        }
    }
}

pub async fn handle_delete_comment(
    state: &AppState,
    session_token: &str,
    comment_id: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_COMMENT_DELETE,
        &Resource::Comment(comment_id.to_string()),
        "comment not found",
    )
    .await?;

    repo::comment::transfer_comment_ownership(&state.db, comment_id)
        .await
        .map_err(|e| match e {
            crate::repo::transfer::TargetTransferError::TargetNotFound => {
                LogicError::not_found("comment not found")
            }
            crate::repo::transfer::TargetTransferError::NoRecycler => {
                LogicError::internal("no recycler available")
            }
            crate::repo::transfer::TargetTransferError::Db(e) => {
                LogicError::internal(format!("failed to transfer comment ownership: {e}"))
            }
        })?;

    if let Ok(Some(version_id)) =
        crate::logic::version::resolve_version_of_comment(state, comment_id).await
        && let Ok(Some(article_id)) =
            crate::logic::version::resolve_article_id_of_version(state, &version_id).await
    {
        if let Err(e) = repo::search::sync_article(&state.search, &state.db, &article_id).await {
            tracing::warn!(article_id = %article_id, error = %e, "search index sync after comment delete failed");
        }
    }

    tracing::info!(
        user_id = %user_id,
        comment_id = %comment_id,
        "comment deleted, ownership transferred to recycler"
    );

    Ok(())
}

pub async fn handle_read_comments(
    state: &AppState,
    session_token: &str,
    version_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<serde_json::Value>, u64), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_COMMENT_READ,
        &Resource::Version(version_id.to_string()),
        "version not found",
    )
    .await?;
    let version_exists = repo::article::read_version(&state.db, version_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .is_some();
    if !version_exists {
        return Err(LogicError::not_found("version not found"));
    }
    repo::comment::read_comments_page_by_version(
        &state.db,
        version_id,
        state.config.server.max_comment_tree_depth as usize,
        limit,
        offset,
    )
    .await
    .map_err(|e| LogicError::internal(format!("database query failed: {e}")))
}

pub async fn handle_update_comment_content(
    state: &AppState,
    session_token: &str,
    comment_id: &str,
    raw_content: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(
        state,
        &user_id,
        PERMISSION_COMMENT_UPDATE,
        &Resource::Comment(comment_id.to_string()),
    )
    .await?;
    let content = validate_comment_body(
        raw_content,
        state.config.server.max_comment_body_chars as usize,
    )?;
    repo::comment::update_comment_content(&state.db, comment_id, &content)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("comment not found"))
}

pub async fn handle_hard_delete_comment(
    state: &AppState,
    session_token: &str,
    comment_id: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(
        state,
        &user_id,
        PERMISSION_COMMENT_DELETE,
        &Resource::Comment(comment_id.to_string()),
    )
    .await?;
    repo::hard_delete::hard_delete_comment(&state.db, comment_id)
        .await
        .map_err(|e| LogicError::internal(format!("failed to delete comment: {e}")))?;
    Ok(())
}
