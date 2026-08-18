use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateTagRequest, DeleteBody, DeleteMode, TagUpdateRequest};
use nail_common::response::tag::TagNameView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppPath, AppQuery};
use crate::interface::principal::Principal;

#[derive(Debug, Default, Deserialize)]
pub struct TagListParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn create_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppJson(payload): AppJson<CreateTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let (id, name) =
        crate::logic::tag::create_tag(&state, &principal.user_id, &payload.name).await?;
    Ok(json_response(
        StatusCode::CREATED,
        TagNameView { id, name },
        "created",
    ))
}

pub async fn read_tags(
    State(state): State<AppState>,
    principal: Principal,
    AppQuery(params): AppQuery<TagListParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
        state.config.server.max_search_pages,
    )?;
    let data = crate::logic::tag::read_tags(&state, &principal.user_id, page, limit).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(tag_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::tag::read_tag(&state, &principal.user_id, &tag_id).await?;
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
    let (id, name) =
        crate::logic::tag::update_tag(&state, &principal.user_id, &tag_id, &name).await?;
    Ok(json_response(
        StatusCode::OK,
        TagNameView { id, name },
        "ok",
    ))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(tag_id): AppPath<String>,
    AppJson(payload): AppJson<DeleteBody>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.mode != Some(DeleteMode::Hard) {
        return Err(ApiError::bad_request(
            "tag delete only supports mode \"hard\"",
        ));
    }
    crate::logic::tag::delete_tag(&state, &principal.user_id, &tag_id).await?;
    Ok(json_response(
        StatusCode::OK,
        serde_json::json!({ "id": tag_id }),
        "deleted",
    ))
}
