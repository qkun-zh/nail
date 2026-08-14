use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{DeleteBody, UpdateArticleRequest};
use nail_common::response::article::CreateArticleView;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::infrastructure::pdf::{PdfStreamGuard, PdfUpload, TempPdf};
use nail_common::hash::PdfHasher;
use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppMultipart, AppPath, AppQuery};
use crate::interface::principal::Principal;

pub async fn read_articles(
    State(state): State<AppState>,
    _principal: Principal,
    AppQuery(params): AppQuery<nail_common::request::ArticleSearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::article::read_articles(&state, &params).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn create_article(
    State(state): State<AppState>,
    principal: Principal,
    AppMultipart(mut multipart): AppMultipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut title = None;
    let mut summary = None;
    let mut tags = None;
    let mut version = None;
    let mut note = None;
    let mut upload = None;

    while let Some(field) = multipart.next_field().await.map_err(map_multipart_error)? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => upload = Some(stream_pdf_field(&state, field).await?),
            "title" => title = Some(read_text_field(&state, field).await?),
            "summary" => summary = Some(read_text_field(&state, field).await?),
            "tags" => tags = Some(read_text_field(&state, field).await?),
            "version" => version = Some(read_text_field(&state, field).await?),
            "note" => note = Some(read_text_field(&state, field).await?),
            _ => {
                drop(field);
            }
        }
    }

    let title = title.ok_or_else(|| ApiError::bad_request("title is required"))?;
    let summary = summary.ok_or_else(|| ApiError::bad_request("summary is required"))?;
    let tags = tags.ok_or_else(|| ApiError::bad_request("tags is required"))?;
    let version = version.ok_or_else(|| ApiError::bad_request("version is required"))?;
    let note = note.ok_or_else(|| ApiError::bad_request("note is required"))?;
    let upload = upload.ok_or_else(|| ApiError::bad_request("file is required"))?;

    let (article_id, version_id) = crate::logic::article::create_article(
        &state,
        &principal.user_id,
        &title,
        &summary,
        &tags,
        &version,
        &note,
        upload,
    )
    .await?;

    Ok(json_response(
        StatusCode::CREATED,
        CreateArticleView {
            article_id,
            version_id,
        },
        "created",
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct ArticleReadParams {
    pub check_if_is_author: Option<bool>,
}

pub async fn read_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppQuery(params): AppQuery<ArticleReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::article::read_article(
        &state,
        &principal.user_id,
        &article_id,
        params.check_if_is_author.unwrap_or(false),
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn update_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppJson(payload): AppJson<UpdateArticleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::article::update_article(
        &state,
        &principal.user_id,
        &article_id,
        &payload.title,
        &payload.summary,
        &payload.tags,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn delete_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppJson(payload): AppJson<DeleteBody>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::article::delete_article(
        &state,
        &principal.user_id,
        &article_id,
        payload.mode,
    )
    .await?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}

pub(crate) async fn read_text_field(
    state: &AppState,
    field: axum::extract::multipart::Field<'_>,
) -> Result<String, ApiError> {
    let bytes = field.bytes().await.map_err(map_multipart_error)?;
    if bytes.len() as u64 > state.config.server.max_text_field_bytes {
        return Err(ApiError::bad_request("text field too large"));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| ApiError::bad_request("text field must be UTF-8"))
}

pub(crate) async fn stream_pdf_field(
    state: &AppState,
    mut field: axum::extract::multipart::Field<'_>,
) -> Result<PdfUpload, ApiError> {
    let temp_path = std::path::Path::new(&state.config.server.pdf_storage_path)
        .join(".tmp")
        .join(format!("{}.pdf", uuid::Uuid::now_v7()));
    let temp = TempPdf::new(temp_path.clone());
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|error| ApiError::from(crate::logic::error::LogicError::internal(format!("failed to create temp pdf: {error}"))))?;
    let mut guard = PdfStreamGuard::new(state.config.server.max_pdf_size_bytes);
    let mut hasher = PdfHasher::new();

    while let Some(chunk) = field.chunk().await.map_err(map_multipart_error)? {
        guard
            .update(&chunk)
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|error| {
            ApiError::from(crate::logic::error::LogicError::internal(format!(
                "failed to write temp pdf: {error}"
            )))
        })?;
    }
    guard
        .finish()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    file.flush().await.map_err(|error| {
        ApiError::from(crate::logic::error::LogicError::internal(format!(
            "failed to flush temp pdf: {error}"
        )))
    })?;
    drop(file);

    Ok(PdfUpload::received(hasher.finalize(), temp))
}

pub(crate) fn map_multipart_error(error: axum::extract::multipart::MultipartError) -> ApiError {
    ApiError {
        status: error.status(),
        message: error.body_text(),
    }
}
