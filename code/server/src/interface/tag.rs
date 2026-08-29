use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::request::{CreateTagRequest, DeleteMode, DeleteQuery, TagUpdateRequest};
use common::response::NamedRef;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppPath, AppQuery, PagedQueryParams};
use crate::interface::principal::Principal;

pub async fn create_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppJson(payload): AppJson<CreateTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let (id, name) = crate::logic::tag::create_tag(&state, &principal.user_id, &payload.name)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(
        StatusCode::CREATED,
        NamedRef { id, name },
        "created",
    ))
}

pub async fn read_tags(
    State(state): State<AppState>,
    principal: Principal,
    AppQuery(query): AppQuery<PagedQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page(
        query.page,
        query.limit,
        state.config.server.tag_page_size,
    );
    let data = crate::logic::tag::read_tags(&state, &principal.user_id, page, limit)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(tag_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::tag::read_tag(&state, &principal.user_id, &tag_id)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(tag_id): AppPath<String>,
    AppJson(payload): AppJson<TagUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name = payload
        .name
        .ok_or_else(|| ApiError::bad_request("name is required"))?;
    let (id, name) = crate::logic::tag::update_tag(&state, &principal.user_id, &tag_id, &name)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, NamedRef { id, name }, "ok"))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(tag_id): AppPath<String>,
    AppQuery(query): AppQuery<DeleteQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.mode != Some(DeleteMode::Hard) {
        return Err(ApiError::bad_request(
            "tag delete only supports mode \"hard\"",
        ));
    }
    crate::logic::tag::delete_tag(&state, &principal.user_id, &tag_id)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(
        StatusCode::OK,
        serde_json::json!({ "id": tag_id }),
        "deleted",
    ))
}

pub async fn apply_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(params): AppPath<TagArticleParams>,
) -> Result<impl IntoResponse, ApiError> {
    crate::logic::tag::apply_tag(&state, &principal.user_id, &params.id, &params.tid)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, serde_json::json!({}), "ok"))
}

pub async fn unapply_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(params): AppPath<TagArticleParams>,
) -> Result<impl IntoResponse, ApiError> {
    crate::logic::tag::unapply_tag(&state, &principal.user_id, &params.id, &params.tid)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, serde_json::json!({}), "ok"))
}

#[derive(Debug, Deserialize)]
pub struct TagArticleParams {
    pub id: String,
    pub tid: String,
}
