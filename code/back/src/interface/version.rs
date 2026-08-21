use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::DeleteQuery;
use nail_common::response::version::VersionIdView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppMultipart, AppPaged, AppPath, AppQuery};
use crate::interface::multipart::{MultipartField, collect_fields};
use crate::interface::principal::Principal;

pub async fn create_version(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppMultipart(multipart): AppMultipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut fields = collect_fields(&state, multipart, &["file"], &["version", "note"]).await?;
    let version = fields
        .remove("version")
        .and_then(MultipartField::into_text)
        .ok_or_else(|| ApiError::bad_request("version is required"))?;
    let note = fields
        .remove("note")
        .and_then(MultipartField::into_text)
        .ok_or_else(|| ApiError::bad_request("note is required"))?;
    let upload = fields
        .remove("file")
        .and_then(MultipartField::into_pdf)
        .ok_or_else(|| ApiError::bad_request("file is required"))?;

    let version_id = crate::logic::version::create_version(
        &state,
        &principal.user_id,
        &article_id,
        &version,
        &note,
        upload,
    )
    .await
    .map_err(ApiError::from_logic)?;

    Ok(json_response(
        StatusCode::CREATED,
        VersionIdView { version_id },
        "created",
    ))
}

pub async fn read_versions(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppPaged((page, limit)): AppPaged,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::version::read_versions(&state, &principal.user_id, &article_id, page, limit)
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

#[derive(Debug, Default, Deserialize)]
pub struct VersionReadParams {
    pub article_id: Option<String>,
}

pub async fn read_version(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppQuery(params): AppQuery<VersionReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::version::read_version(
        &state,
        &principal.user_id,
        &version_id,
        params.article_id.as_deref(),
    )
    .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

#[derive(Debug, Deserialize)]
pub struct UpdateVersionNoteRequest {
    pub note: String,
}

pub async fn update_version(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppJson(payload): AppJson<UpdateVersionNoteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::version::update_version(
        &state,
        &principal.user_id,
        &version_id,
        &payload.note,
    )
    .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn delete_version(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
    AppQuery(query): AppQuery<DeleteQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::version::delete_version(&state, &principal.user_id, &version_id, query.mode)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}

pub async fn undelete_soft_version(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(version_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::version::undelete_soft_version(&state, &principal.user_id, &version_id)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "undeleted"))
}
