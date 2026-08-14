use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateCommentRequest, DeleteBody};
use nail_common::response::comment::CommentIdView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;

pub async fn create_comment(
    State(state): State<AppState>,
    principal: Principal,
    Path(version_id): Path<String>,
    Json(payload): Json<CreateCommentRequest>,
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
    Path(parent_comment_id): Path<String>,
    Json(payload): Json<CreateCommentRequest>,
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
    pub check_if_is_author: Option<bool>,
}

pub async fn read_comments(
    State(state): State<AppState>,
    principal: Principal,
    Path(version_id): Path<String>,
    Query(params): Query<CommentsReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
    );
    let data = crate::logic::comment::read_comments(
        &state,
        &principal.user_id,
        &version_id,
        page,
        limit,
        params.check_if_is_author.unwrap_or(false),
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_comment(
    State(state): State<AppState>,
    principal: Principal,
    Path(comment_id): Path<String>,
    Json(payload): Json<CreateCommentRequest>,
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
    Path(comment_id): Path<String>,
    Json(payload): Json<DeleteBody>,
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
