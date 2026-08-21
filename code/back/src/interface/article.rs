use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::{DeleteQuery, UpdateArticleRequest};
use nail_common::response::article::CreateArticleView;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppJson, AppMultipart, AppPath, AppQuery};
use crate::interface::multipart::{MultipartField, collect_fields};
use crate::interface::principal::Principal;

pub async fn search_articles(
    State(state): State<AppState>,
    principal: Principal,
    AppQuery(params): AppQuery<nail_common::request::ArticleSearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::search::search_articles(&state, &principal.user_id, &params)
        .await
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn create_article(
    State(state): State<AppState>,
    principal: Principal,
    AppMultipart(multipart): AppMultipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut fields = collect_fields(
        &state,
        multipart,
        &["file"],
        &["title", "summary", "tags", "version", "note"],
    )
    .await?;
    let title = fields
        .remove("title")
        .and_then(MultipartField::into_text)
        .ok_or_else(|| ApiError::bad_request("title is required"))?;
    let summary = fields
        .remove("summary")
        .and_then(MultipartField::into_text)
        .ok_or_else(|| ApiError::bad_request("summary is required"))?;
    let tags = fields
        .remove("tags")
        .and_then(MultipartField::into_text)
        .ok_or_else(|| ApiError::bad_request("tags is required"))?;
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

    let input = crate::logic::article::ArticleCreateInput {
        title: &title,
        summary: &summary,
        tags: &tags,
        version: &version,
        note: &note,
        upload,
    };
    let (article_id, version_id) =
        crate::logic::article::create_article(&state, &principal.user_id, input)
            .await
            .map_err(ApiError::from_logic)?;

    Ok(json_response(
        StatusCode::CREATED,
        CreateArticleView {
            article_id,
            version_id,
        },
        "created",
    ))
}

pub async fn read_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = crate::logic::article::read_article(&state, &principal.user_id, &article_id)
        .map_err(ApiError::from_logic)?;
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
    .await
    .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}

pub async fn delete_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
    AppQuery(query): AppQuery<DeleteQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::article::delete_article(&state, &principal.user_id, &article_id, query.mode)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "deleted"))
}

pub async fn undelete_soft_article(
    State(state): State<AppState>,
    principal: Principal,
    AppPath(article_id): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data =
        crate::logic::article::undelete_soft_article(&state, &principal.user_id, &article_id)
            .await
            .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "undeleted"))
}
