use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use common::request::{LogoutRequest, UserDeleteRequest, UserUpdateRequest};
use common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::api::{ApiError, logic_err, require_session};
use crate::logic;
use crate::other::AppState;

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    logic::user::handle_logout(&state, &payload.pow, &session_token)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({}),
        "deleted",
    )))
}

#[derive(Debug, Default, Deserialize)]
pub struct UserReadParams {
    name: Option<bool>,
    email_hash: Option<bool>,
}

pub async fn read_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<UserReadParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let actor = logic::authenticate::authenticate_session(&state, &session_token)
        .map_err(logic_err)?;
    let mut data = serde_json::Map::new();
    if user_id == actor {
        if params.name.unwrap_or(true) {
            let name = logic::user::handle_read_name(&state, &session_token)
                .await
                .map_err(logic_err)?;
            data.insert("name".to_string(), serde_json::json!(name));
        }
        if params.email_hash.unwrap_or(false) {
            if let Ok(Some(entry)) = logic::user::handle_read_self_email_hash(&state, &actor).await
            {
                data.insert("email_hash".to_string(), serde_json::json!(entry));
            }
        }
    } else {
        let detail = logic::user::handle_read_user_manage(&state, &session_token, &user_id)
            .await
            .map_err(logic_err)?;
        let obj = detail.as_object().cloned().unwrap_or_default();
        for (k, v) in obj {
            let want = match k.as_str() {
                "name" => params.name.unwrap_or(true),
                "email_hash" => params.email_hash.unwrap_or(true),
                _ => true,
            };
            if want {
                data.insert(k, v);
            }
        }
    }
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::Value::Object(data),
        "ok",
    )))
}

#[derive(Debug, Default, Deserialize)]
pub struct UserListParams {
    page: Option<u64>,
    limit: Option<u64>,
}

pub async fn read_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<UserListParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let page_size = state.config.server.search_page_size;
    let max_page_size = state.config.server.max_search_page_size;
    let limit = params.limit.unwrap_or(page_size).min(max_page_size).max(1);
    let page = params.page.unwrap_or(1).clamp(1, state.config.server.max_page);
    let offset = (page - 1).saturating_mul(limit);
    let (items, total) = logic::user::handle_list_users(&state, &session_token, limit, offset)
        .await
        .map_err(logic_err)?;
    let user_list: Vec<serde_json::Value> = items
        .into_iter()
        .map(|(id, name, email_hash)| {
            serde_json::json!({ "id": id, "name": name, "email_hash": email_hash })
        })
        .collect();
    let has_next = offset.saturating_add(user_list.len() as u64) < total;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "user_list": user_list, "has_next": has_next, "total": total }),
        "ok",
    )))
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UserUpdateRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;

    match (payload.old_email_token, payload.new_email_token) {
        (Some(old_token), Some(new_token)) => {
            let pow = payload.pow.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ResponseEnvelope::err(
                        400,
                        "pow is required to confirm the email update",
                    )),
                )
            })?;
            let new_session_token = logic::email::handle_email_update_confirm(
                &state,
                &pow,
                &old_token,
                &new_token,
                &session_token,
            )
            .await
            .map_err(logic_err)?;
            return Ok(Json(ResponseEnvelope::ok(
                200,
                serde_json::json!({ "session_token": new_session_token }),
                "ok",
            )));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ResponseEnvelope::err(
                    400,
                    "old_email_token and new_email_token must both be provided",
                )),
            ));
        }
        (None, None) => {}
    }

    if let Some(raw_name) = payload.name {
        let name = logic::user::handle_admin_update_name(
            &state,
            &session_token,
            &user_id,
            &raw_name,
        )
        .await
        .map_err(logic_err)?;
        return Ok(Json(ResponseEnvelope::ok(
            200,
            serde_json::json!({ "name": name }),
            "ok",
        )));
    }
    let pow = payload.pow.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "pow is required")),
        )
    })?;
    let name = logic::user::handle_update_name(&state, &pow, &session_token)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "name": name }),
        "ok",
    )))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UserDeleteRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    match payload.mode.as_deref() {
        Some("transfer") => {
            logic::user::handle_deregister_confirm(&state, &payload.pow, &session_token)
                .await
                .map_err(logic_err)?;
            Ok(Json(ResponseEnvelope::ok(
                200,
                serde_json::json!({}),
                "deleted",
            )))
        }
        Some("hard") => {
            logic::user::handle_hard_delete_user(&state, &session_token, &user_id)
                .await
                .map_err(logic_err)?;
            Ok(Json(ResponseEnvelope::ok(
                200,
                serde_json::json!({ "user_id": user_id }),
                "deleted",
            )))
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(
                400,
                "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
            )),
        )),
    }
}
