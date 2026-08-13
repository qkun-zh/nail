use axum::Json;
use axum::extract::{Path, Query, State};
use nail_common::request::{UserDeleteRequest, UserUpdateRequest};
use nail_common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;
use crate::interface::principal::Principal;

const DEFAULT_PAGE_SIZE: u64 = 8;
const MAX_PAGE_SIZE: u64 = 200;
const MAX_PAGE: u64 = 10_000;

#[derive(Debug, Default, Deserialize)]
pub struct UserReadParams {
    pub name: Option<bool>,
    pub email_hash: Option<bool>,
}

pub async fn read(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Query(params): Query<UserReadParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let name_requested = params.name.unwrap_or(true);
    let email_hash_requested = params.email_hash.unwrap_or(false);
    let data = crate::logic::user::read_user_profile(
        &state,
        &principal.user_id,
        &user_id,
        name_requested,
        email_hash_requested,
    )
    .await?;
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}

#[derive(Debug, Default, Deserialize)]
pub struct UserListParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<UserListParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let page = params.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let data = crate::logic::user::list_users(&state, &principal.user_id, page, limit).await?;
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}

pub async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Json(payload): Json<UserUpdateRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let data =
        crate::logic::user::update_user(&state, &principal.user_id, &user_id, payload).await?;
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}

pub async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Json(payload): Json<UserDeleteRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let data =
        crate::logic::user::delete_user(&state, &principal.user_id, &user_id, payload).await?;
    Ok(Json(ResponseEnvelope::ok(200, data, "deleted")))
}
