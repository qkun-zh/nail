use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateRoleRequest, DeleteBody, DeleteMode, RoleUpdateRequest};
use nail_common::response::role::RoleNameView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;

#[derive(Debug, Default, Deserialize)]
pub struct RoleListParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn create_role(
    State(state): State<AppState>,
    principal: Principal,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name = crate::logic::role::create_role(&state, &principal.user_id, &payload.name).await?;
    Ok(json_response(
        StatusCode::CREATED,
        RoleNameView { name },
        "created",
    ))
}

pub async fn read_roles(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<RoleListParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
    );
    let data = crate::logic::role::read_roles(&state, &principal.user_id, page, limit).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_role(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::role::read_role(&state, &principal.user_id, &name).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_role(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(payload): Json<RoleUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let permissions = payload.permissions.unwrap_or_default();
    let tags = payload.tags.unwrap_or_default();
    let users = payload.users.unwrap_or_default();
    let name = crate::logic::role::update_role(
        &state,
        &principal.user_id,
        &name,
        &permissions.add,
        &permissions.remove,
        &tags.add,
        &tags.remove,
        &users.add,
        &users.remove,
    )
    .await?;
    Ok(json_response(StatusCode::OK, RoleNameView { name }, "ok"))
}

pub async fn delete_role(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(payload): Json<DeleteBody>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.mode != Some(DeleteMode::Hard) {
        return Err(ApiError::bad_request("role delete only supports mode \"hard\""));
    }
    let data = crate::logic::role::delete_role(&state, &principal.user_id, &name).await?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}
