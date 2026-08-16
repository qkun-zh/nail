use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateCommentRequest, DeleteBody};
use nail_common::response::comment::{CommentIdView, CommentListPage, CommentView};
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppPath, AppQuery};
use crate::interface::principal::Principal;

pub async fn create_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppJson(payload): AppJson<CreateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let comment_id = crate::logic::comment::create_comment(
        &state,
        &principal.user_id,
        &version_id,
        &payload.content,
    )
    .await?;
    Ok(json_response(
        StatusCode::CREATED,
        CommentIdView { comment_id },
        "created",
    ))
}

pub async fn create_reply(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(parent_comment_id): AppPath<String>,
    AppJson(payload): AppJson<CreateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let comment_id = crate::logic::comment::create_reply(
        &state,
        &principal.user_id,
        &parent_comment_id,
        &payload.content,
    )
    .await?;
    Ok(json_response(
        StatusCode::CREATED,
        CommentIdView { comment_id },
        "created",
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct CommentsReadParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn read_comments(
    State(state): State<AppState>,
    _principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppQuery(params): AppQuery<CommentsReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
        state.config.server.max_search_pages,
    )?;
    let data = crate::logic::comment::read_comments(&state, &version_id, page, limit).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(comment_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data: CommentView =
        crate::logic::comment::read_comment(&state, &principal.user_id, &comment_id).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_comment_children(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(parent_comment_id): AppPath<String>,
    AppQuery(params): AppQuery<CommentsReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
        state.config.server.max_search_pages,
    )?;
    let data: CommentListPage = crate::logic::comment::read_comment_children(
        &state,
        &principal.user_id,
        &parent_comment_id,
        page,
        limit,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(comment_id): AppPath<String>,
    AppJson(payload): AppJson<CreateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::comment::update_comment(
        &state,
        &principal.user_id,
        &comment_id,
        &payload.content,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(comment_id): AppPath<String>,
    AppJson(payload): AppJson<DeleteBody>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::comment::delete_comment(
        &state,
        &principal.user_id,
        &comment_id,
        payload.mode,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}
