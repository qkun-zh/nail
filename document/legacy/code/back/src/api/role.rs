
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::{Json, extract::Query, extract::State};
use common::request::{CreateRoleRequest, RoleUpdateRequest};
use common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::api::{ApiError, logic_err, require_session};
use crate::logic;
use crate::other::AppState;

#[derive(Debug, Default, Deserialize)]
pub struct RoleListParams {
    page: Option<u64>,
    limit: Option<u64>,
}

fn paginate(state: &AppState, page: Option<u64>, limit: Option<u64>) -> (u64, u64) {
    let page_size = state.config.server.search_page_size;
    let max_page_size = state.config.server.max_search_page_size;
    let limit = limit.unwrap_or(page_size).min(max_page_size).max(1);
    let page = page.unwrap_or(1).clamp(1, state.config.server.max_page);
    (limit, (page - 1).saturating_mul(limit))
}

pub async fn create_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let name = logic::role::handle_create_role(&state, &session_token, &payload.name)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        201,
        serde_json::json!({ "name": name }),
        "created",
    )))
}

pub async fn read_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<RoleListParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let (limit, offset) = paginate(&state, params.page, params.limit);
    let (items, total) = logic::role::handle_read_roles(&state, &session_token, limit, offset)
        .await
        .map_err(logic_err)?;
    let role_list: Vec<serde_json::Value> = items
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "permissions": r.permissions,
                "scopes": r.scopes,
                "member_count": r.member_count,
            })
        })
        .collect();
    let has_next = offset.saturating_add(role_list.len() as u64) < total;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "role_list": role_list, "has_next": has_next, "total": total }),
        "ok",
    )))
}

pub async fn read_role(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let detail = logic::role::handle_read_role(&state, &session_token, &name)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(200, detail, "ok")))
}

pub async fn update_role(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<RoleUpdateRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let changes = payload.permissions.unwrap_or_default();
    let tags = payload.tags.unwrap_or_default();
    let users = payload.users.unwrap_or_default();
    let name = logic::role::handle_update_role(
        &state,
        &session_token,
        &name,
        &changes.add,
        &changes.remove,
        &tags.add,
        &tags.remove,
        &users.add,
        &users.remove,
    )
    .await
    .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "name": name }),
        "ok",
    )))
}

pub async fn delete_role(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<common::request::DeleteBody>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    if payload.mode.as_deref() != Some("hard") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "role delete only supports mode \"hard\"")),
        ));
    }
    logic::role::handle_delete_role(&state, &session_token, &name)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "name": name }),
        "deleted",
    )))
}
