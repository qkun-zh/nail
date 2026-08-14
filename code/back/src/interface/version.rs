use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::DeleteBody;
use nail_common::response::version::VersionIdView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::article::{read_text_field, stream_pdf_field};
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;

pub async fn create_version(
    State(state): State<AppState>,
    principal: Principal,
    Path(article_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut version = None;
    let mut note = None;
    let mut upload = None;

    while let Some(field) = multipart.next_field().await.map_err(super::article::map_multipart_error)? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => upload = Some(stream_pdf_field(&state, field).await?),
            "version" => version = Some(read_text_field(&state, field).await?),
            "note" => note = Some(read_text_field(&state, field).await?),
            _ => {
                drop(field);
            }
        }
    }

    let version = version.ok_or_else(|| ApiError::bad_request("version is required"))?;
    let note = note.ok_or_else(|| ApiError::bad_request("note is required"))?;
    let upload = upload.ok_or_else(|| ApiError::bad_request("file is required"))?;

    let version_id = crate::logic::version::create_version(
        &state,
        &principal.user_id,
        &article_id,
        &version,
        &note,
        upload,
    )
    .await?;

    Ok(json_response(
        StatusCode::CREATED,
        VersionIdView { version_id },
        "created",
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct VersionsReadParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn read_versions(
    State(state): State<AppState>,
    _principal: Principal,
    Path(article_id): Path<String>,
    Query(params): Query<VersionsReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let (page, limit) = crate::logic::pagination::clamp_page_limit(
        params.page,
        params.limit,
        state.config.server.search_page_size,
    );
    let data = crate::logic::version::read_versions(&state, &article_id, page, limit).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

#[derive(Debug, Default, Deserialize)]
pub struct VersionReadParams {
    pub article_id: Option<String>,
    pub check_if_is_author: Option<bool>,
}

pub async fn read_version(
    State(state): State<AppState>,
    principal: Principal,
    Path(version_id): Path<String>,
    Query(params): Query<VersionReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::version::read_version(
        &state,
        &principal.user_id,
        &version_id,
        params.article_id.as_deref(),
        params.check_if_is_author.unwrap_or(false),
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

#[derive(Debug, Deserialize)]
pub struct UpdateVersionNoteRequest {
    pub note: String,
}

pub async fn update_version(
    State(state): State<AppState>,
    principal: Principal,
    Path(version_id): Path<String>,
    Json(payload): Json<UpdateVersionNoteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::version::update_version(
        &state,
        &principal.user_id,
        &version_id,
        &payload.note,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn delete_version(
    State(state): State<AppState>,
    principal: Principal,
    Path(version_id): Path<String>,
    Json(payload): Json<DeleteBody>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::version::delete_version(
        &state,
        &principal.user_id,
        &version_id,
        payload.mode,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}
