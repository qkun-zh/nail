
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use common::request::{CreateCommentRequest, DeleteBody};
use common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::api::{ApiError, logic_err, require_session, strip_table_prefix};
use crate::logic;
use crate::other::AppState;

pub async fn create_comment(
    State(state): State<AppState>,
    Path(version_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<ResponseEnvelope<serde_json::Value>>), ApiError> {
    let session_token = require_session(&state, &headers)?;

    let comment_id = logic::comment::handle_create_comment(
        &state,
        &session_token,
        &version_id,
        &payload.content,
    )
    .await
    .map_err(logic_err)?;

    Ok((
        StatusCode::CREATED,
        Json(ResponseEnvelope::ok(
            201,
            serde_json::json!({ "comment_id": comment_id }),
            "created",
        )),
    ))
}

pub async fn create_reply(
    State(state): State<AppState>,
    Path(parent_comment_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<ResponseEnvelope<serde_json::Value>>), ApiError> {
    let session_token = require_session(&state, &headers)?;

    let comment_id = logic::comment::handle_create_reply(
        &state,
        &session_token,
        &parent_comment_id,
        &payload.content,
    )
    .await
    .map_err(logic_err)?;

    Ok((
        StatusCode::CREATED,
        Json(ResponseEnvelope::ok(
            201,
            serde_json::json!({ "comment_id": comment_id }),
            "created",
        )),
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct CommentsListParams {
    page: Option<u64>,
    limit: Option<u64>,
    check_if_is_author: Option<bool>,
}

pub async fn read_comments(
    State(state): State<AppState>,
    Path(version_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<CommentsListParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;

    let page_size = state.config.server.search_page_size;
    let max_page_size = state.config.server.max_search_page_size;
    let limit = params.limit.unwrap_or(page_size).min(max_page_size).max(1);
    let page = params.page.unwrap_or(1).clamp(1, state.config.server.max_page);
    let offset = (page - 1).saturating_mul(limit);

    let (rows, total) =
        logic::comment::handle_read_comments(&state, &session_token, &version_id, limit, offset)
            .await
            .map_err(logic_err)?;

    let mut seen_users: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut user_ids: Vec<String> = Vec::new();
    let mut entries: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row
            .get("comment_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            tracing::warn!(
                row = %row,
                "comment row missing comment_id: skipped from response"
            );
            continue;
        }
        let content = row
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let user_id = row
            .get("author")
            .and_then(|v| v.as_str())
            .map(strip_table_prefix)
            .unwrap_or_default();
        let parent_id = row
            .get("parent")
            .and_then(|v| v.as_str())
            .map(strip_table_prefix);
        if !user_id.is_empty() && seen_users.insert(user_id.clone()) {
            user_ids.push(user_id.clone());
        }
        let created_at = common::time::uuidv7_timestamp_secs(&id)
            .ok_or_else(|| logic_err(logic::error::LogicError::internal("invalid comment id")))?;
        entries.push(serde_json::json!({
            "id": id,
            "content": content,
            "user_id": user_id,
            "parent_id": parent_id,
            "created_at": created_at,
        }));
    }

    let user_names = logic::user::read_author_names_by_user(&state, &user_ids).await;

    for entry in &mut entries {
        let user_id = entry
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = user_names.get(&user_id).cloned().unwrap_or_default();
        entry["user_name"] = serde_json::Value::String(name);
    }

    let mut data = serde_json::json!({
        "comments": entries,
        "has_next": offset.saturating_add(entries.len() as u64) < total,
        "total": total,
    });
    if params.check_if_is_author == Some(true) {
        let is_author =
            logic::author::handle_is_author(&state, &session_token, None, Some(&version_id), None)
                .await
                .map_err(logic_err)?;
        data["is_author"] = serde_json::json!(is_author);
    }
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<DeleteBody>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    match payload.mode.as_deref() {
        Some("transfer") => {
            logic::comment::handle_delete_comment(&state, &session_token, &comment_id)
                .await
                .map_err(logic_err)?;
        }
        Some("hard") => {
            logic::comment::handle_hard_delete_comment(&state, &session_token, &comment_id)
                .await
                .map_err(logic_err)?;
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ResponseEnvelope::err(
                    400,
                    "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
                )),
            ));
        }
    }

    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "comment_id": comment_id }),
        "deleted",
    )))
}

pub async fn update_comment(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    logic::comment::handle_update_comment_content(&state, &session_token, &comment_id, &payload.content)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "comment_id": comment_id }),
        "ok",
    )))
}
