use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateRoleRequest, DeleteMode, DeleteQuery, RoleUpdateRequest};
use nail_common::response::NamedRef;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppPaged, AppPath, AppQuery};
use crate::interface::principal::Principal;

pub async fn create_role(
    State(state): State<AppState>,
    principal: Principal,
    AppJson(payload): AppJson<CreateRoleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let (id, name) = crate::logic::role::create_role(&state, &principal.user_id, &payload.name)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(
        StatusCode::CREATED,
        NamedRef { id, name },
        "created",
    ))
}

pub async fn read_roles(
    State(state): State<AppState>,
    principal: Principal,
    AppPaged((page, limit)): AppPaged,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::role::read_roles(&state, &principal.user_id, page, limit)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_role(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(role_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::role::read_role(&state, &principal.user_id, &role_id)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_role(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(role_id): AppPath<String>,
    AppJson(payload): AppJson<RoleUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let permissions = payload.permissions.unwrap_or_default();
    let users = payload.users.unwrap_or_default();
    let update = crate::logic::role::RoleUpdate {
        permissions_add: &permissions.add,
        permissions_remove: &permissions.remove,
        users_add: &users.add,
        users_remove: &users.remove,
    };
    let view = crate::logic::role::update_role(&state, &principal.user_id, &role_id, &update)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, view, "ok"))
}

pub async fn delete_role(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(role_id): AppPath<String>,
    AppQuery(query): AppQuery<DeleteQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.mode != Some(DeleteMode::Hard) {
        return Err(ApiError::bad_request(
            "role delete only supports mode \"hard\"",
        ));
    }
    let view = crate::logic::role::delete_role(&state, &principal.user_id, &role_id)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, view, "deleted"))
}
