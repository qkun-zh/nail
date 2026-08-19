use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{CreateCommentRequest, DeleteBody};
use nail_common::response::comment::{CommentIdView, CommentView};

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppPaged, AppPath};
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
    .await
    .map_err(ApiError::from_logic)?;
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
    .await
    .map_err(ApiError::from_logic)?;
    Ok(json_response(
        StatusCode::CREATED,
        CommentIdView { comment_id },
        "created",
    ))
}

pub async fn read_comments(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppPaged((page, limit)): AppPaged,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::comment::read_comments(&state, &principal.user_id, &version_id, page, limit)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(comment_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data: CommentView =
        crate::logic::comment::read_comment(&state, &principal.user_id, &comment_id)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn read_comment_children(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(parent_comment_id): AppPath<String>,
    AppPaged((page, limit)): AppPaged,
) -> Result<impl IntoResponse, ApiError> {
    let data: nail_common::response::ListPage<CommentView> =
        crate::logic::comment::read_comment_children(
            &state,
            &principal.user_id,
            &parent_comment_id,
            page,
            limit,
        )
        .await
        .map_err(ApiError::from_logic)?;
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
    .await
    .map_err(ApiError::from_logic)?;
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
    .await
    .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}

pub async fn undelete_soft_comment(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(comment_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::comment::undelete_soft_comment(&state, &principal.user_id, &comment_id)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "undeleted"))
}
