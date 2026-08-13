use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{TokenRequest, UserDeleteRequest, UserUpdateRequest};
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;

const DEFAULT_PAGE_SIZE: u64 = 8;
const MAX_PAGE_SIZE: u64 = 200;
const MAX_PAGE: u64 = 10_000;

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = crate::logic::user::create_user(&state, &payload.pow).await?;
    let session_token = crate::logic::session::create_session(&state, &user_id)?;
    Ok(json_response::<serde_json::Value>(
        StatusCode::OK,
        serde_json::json!({ "session_token": session_token }),
        "ok",
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct UserReadParams {
    pub name: Option<bool>,
    pub email_hash: Option<bool>,
}

pub async fn read_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Query(params): Query<UserReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let name_requested = params.name.unwrap_or(true);
    let email_hash_requested = params.email_hash.unwrap_or(false);
    let data = crate::logic::user::read_user(
        &state,
        &principal.user_id,
        &user_id,
        name_requested,
        email_hash_requested,
    )
    .await?;
    Ok(json_response::<serde_json::Value>(StatusCode::OK, data, "ok"))
}

#[derive(Debug, Default, Deserialize)]
pub struct UsersReadParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn read_users(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<UsersReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let page = params.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let data = crate::logic::user::read_users(&state, &principal.user_id, page, limit).await?;
    Ok(json_response::<serde_json::Value>(StatusCode::OK, data, "ok"))
}

pub async fn update_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Json(payload): Json<UserUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::user::update_user(&state, &principal.user_id, &user_id, payload).await?;
    Ok(json_response::<serde_json::Value>(StatusCode::OK, data, "ok"))
}

pub async fn delete_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(user_id): Path<String>,
    Json(payload): Json<UserDeleteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::user::delete_user(&state, &principal.user_id, &user_id, payload).await?;
    Ok(json_response::<serde_json::Value>(
        StatusCode::OK,
        data,
        "deleted",
    ))
}
