
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    response::IntoResponse,
};
use common::hash::PdfHasher;
use common::request::{DeleteBody, UpdateArticleRequest};
use common::response::ResponseEnvelope;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::api::article_view::{build_article_view, build_article_views};
use crate::api::{ApiError, logic_err, require_session, serve_pdf_file};
use crate::logic;
use crate::other::AppState;
use crate::other::pdf::{PdfStreamGuard, PdfUpload, TempPdf};

pub(crate) async fn read_text_field(
    field: &mut axum::extract::multipart::Field<'_>,
    max_bytes: usize,
) -> Result<String, ()> {
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|_| ())? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max_bytes {
            return Err(());
        }
    }
    String::from_utf8(bytes).map_err(|_| ())
}

pub(crate) async fn stream_pdf_field(
    state: &AppState,
    field: &mut axum::extract::multipart::Field<'_>,
) -> Result<PdfUpload, ApiError> {
    let max = state.config.server.max_pdf_size_bytes;
    let tmp_dir = std::path::PathBuf::from(&state.config.server.pdf_storage_path).join(".tmp");
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| pdf_upload_err_500(format!("failed to create upload temp dir: {e}")))?;
    let tmp_path = tmp_dir.join(format!("{}.pdf", Uuid::now_v7()));
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| pdf_upload_err_500(format!("failed to create upload temp file: {e}")))?;
    let mut guard = PdfStreamGuard::new(max);
    let mut hasher = PdfHasher::new();
    let tmp = TempPdf::new(tmp_path);
    while let Some(chunk) = field.chunk().await.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "invalid file field")),
        )
    })? {
        guard.update(&chunk).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ResponseEnvelope::err(400, e.to_string())),
            )
        })?;
        file.write_all(&chunk)
            .await
            .map_err(|e| pdf_upload_err_500(format!("failed to write upload temp file: {e}")))?;
        hasher.update(&chunk);
    }
    file.flush()
        .await
        .map_err(|e| pdf_upload_err_500(format!("failed to flush upload temp file: {e}")))?;
    drop(file);
    guard.finish().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, e.to_string())),
        )
    })?;
    let hash = hasher.finalize();
    Ok(PdfUpload::received(hash, tmp))
}

fn pdf_upload_err_500(reason: String) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ResponseEnvelope::err(500, reason)),
    )
}

#[derive(Debug, Default, Deserialize)]
pub struct ArticleReadParams {
    key_word: Option<String>,
    ranges: Option<String>,
    sort: Option<String>,
    from: Option<u64>,
    to: Option<u64>,
    limit: Option<u64>,
    page: Option<u64>,
}

pub async fn read_articles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ArticleReadParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    require_session(&state, &headers)?;

    let has_search = params.key_word.is_some()
        || params.ranges.is_some()
        || params.sort.is_some()
        || params.from.is_some()
        || params.to.is_some();

    if has_search {
        let search_params = common::search::ArticleSearchParams {
            q: params.key_word,
            ranges: params.ranges,
            sort: params.sort,
            from: params.from,
            to: params.to,
            limit: params.limit,
            page: params.page,
        };
        let page = logic::article_search::handle_search_articles(&state, &search_params)
            .await
            .map_err(logic_err)?;
        let article_list: Vec<serde_json::Value> = page
            .article_list
            .into_iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "title": item.title,
                    "author": item.author,
                    "time": item.time,
                    "hits": item.hits.into_iter().map(|h| serde_json::json!({
                        "field": h.field,
                        "label": h.label,
                        "snippet": h.snippet,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        return Ok(Json(ResponseEnvelope::ok(
            200,
            serde_json::json!({
                "article_list": article_list,
                "page": page.page,
                "total": page.total,
                "total_pages": page.total_pages,
                "has_next": page.has_more,
                "has_prev": page.has_prev,
                "truncated": page.truncated,
            }),
            "ok",
        )));
    }

    let page_size = state.config.server.search_page_size;
    let max_page_size = state.config.server.max_search_page_size;
    let limit = params.limit.unwrap_or(page_size).min(max_page_size).max(1);
    let page = params
        .page
        .unwrap_or(1)
        .clamp(1, state.config.server.max_page);
    let offset = (page - 1).saturating_mul(limit);

    let (article_list, has_more, total) = logic::article::handle_read_articles(&state, limit, offset)
        .await
        .map_err(logic_err)?;
    let enriched = build_article_views(&state, article_list).await;
    let total_pages = if total == 0 { 0 } else { total.div_ceil(limit) };
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({
            "article_list": enriched,
            "page": page,
            "total": total,
            "total_pages": total_pages,
            "has_next": has_more,
            "has_prev": page > 1,
            "truncated": total_pages > state.config.server.max_search_pages,
        }),
        "ok",
    )))
}

#[derive(Debug, Default, Deserialize)]
pub struct ArticleDetailParams {
    check_if_is_author: Option<bool>,
}

pub async fn read_article(
    State(state): State<AppState>,
    Path(article_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<ArticleDetailParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;

    let article = logic::article::handle_read_article(&state, &session_token, &article_id)
        .await
        .map_err(logic_err)?;

    let mut view = build_article_view(&state, article).await;
    if params.check_if_is_author == Some(true) {
        let is_author = logic::author::handle_is_author(
            &state,
            &session_token,
            Some(&article_id),
            None,
            None,
        )
        .await
        .map_err(logic_err)?;
        view["is_author"] = serde_json::json!(is_author);
    }
    Ok(Json(ResponseEnvelope::ok(200, view, "ok")))
}

pub async fn create_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ResponseEnvelope<serde_json::Value>>), ApiError> {
    let session_token = require_session(&state, &headers)?;

    let mut title = String::new();
    let mut summary = String::new();
    let mut raw_tags = String::new();
    let mut version = String::new();
    let mut note = String::new();
    let mut upload: Option<PdfUpload> = None;

    while let Some(mut field) = multipart.next_field().await.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "malformed multipart body")),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = read_text_field(
                    &mut field,
                    state.config.server.max_text_field_bytes as usize,
                )
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ResponseEnvelope::err(400, "invalid title field")),
                    )
                })?;
            }
            "summary" => {
                summary = read_text_field(
                    &mut field,
                    state.config.server.max_text_field_bytes as usize,
                )
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ResponseEnvelope::err(400, "invalid summary field")),
                    )
                })?;
            }
            "tags" => {
                raw_tags = read_text_field(
                    &mut field,
                    state.config.server.max_text_field_bytes as usize,
                )
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ResponseEnvelope::err(400, "invalid tags field")),
                    )
                })?;
            }
            "version" => {
                version = read_text_field(
                    &mut field,
                    state.config.server.max_text_field_bytes as usize,
                )
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ResponseEnvelope::err(
                            400,
                            "version field too large or not UTF-8",
                        )),
                    )
                })?;
            }
            "note" => {
                note = read_text_field(
                    &mut field,
                    state.config.server.max_text_field_bytes as usize,
                )
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ResponseEnvelope::err(
                            400,
                            "note field too large or not UTF-8",
                        )),
                    )
                })?;
            }
            "file" => {
                upload = Some(stream_pdf_field(&state, &mut field).await?);
            }
            _ => {}
        }
    }

    if title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "title is required")),
        ));
    }
    if summary.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "summary is required")),
        ));
    }
    if common::tag::parse_hashtag_tags(&raw_tags, state.config.server.max_tags_per_article)
        .is_ok_and(|tags| tags.is_empty())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "at least one tag required")),
        ));
    }
    if version.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "version is required")),
        ));
    }
    if note.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "note is required")),
        ));
    }
    if upload.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "PDF file is required")),
        ));
    }

    let (article_id, version_id) = logic::article::handle_create_article(
        &state,
        &session_token,
        &title,
        &summary,
        &raw_tags,
        &version,
        &note,
        upload.expect("file field presence checked above"),
    )
    .await
    .map_err(logic_err)?;

    Ok((
        StatusCode::CREATED,
        Json(ResponseEnvelope::ok(
            201,
            serde_json::json!({
                "article_id": article_id,
                "version_id": version_id,
            }),
            "created",
        )),
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct ContentReadParams {
    download: Option<String>,
    version_id: Option<String>,
    token: Option<String>,
}

pub async fn serve_public_pdf(
    State(state): State<AppState>,
    Path((article_id, version_id)): Path<(String, String)>,
    headers: HeaderMap,
    Query(params): Query<ContentReadParams>,
) -> Result<Response, ApiError> {
    let session_token = require_session(&state, &headers)?;

    if params.download.as_deref() == Some("1") || params.download.as_deref() == Some("true") {
        logic::download::handle_mint_download_url(
            &state,
            &session_token,
            &article_id,
            &version_id,
        )
        .await
        .map_err(logic_err)?;
        let new_url = format!(
            "/api/article/{article_id}/version/{version_id}/content/read?version_id={version_id}"
        );
        return Ok(Json(ResponseEnvelope::ok(
            200,
            serde_json::json!({ "url": new_url }),
            "ok",
        ))
        .into_response());
    }

    if params.version_id.is_some() || params.token.is_some() {
        let pdf_path =
            logic::download::handle_consume_download(&state, &session_token, &version_id)
                .await
                .map_err(logic_err)?;
        return serve_pdf_file(&pdf_path).await;
    }

    let pdf_path =
        logic::article::handle_get_pdf_path(&state, &session_token, &article_id, &version_id)
            .await
            .map_err(logic_err)?;
    serve_pdf_file(&pdf_path).await
}

pub async fn update_article(
    State(state): State<AppState>,
    Path(article_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateArticleRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;

    let total_text_len = payload.title.len() + payload.summary.len() + payload.tags.len();
    if total_text_len > state.config.server.max_text_field_bytes as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "text fields too large")),
        ));
    }

    logic::article::handle_update_article(
        &state,
        &session_token,
        &article_id,
        &payload.title,
        &payload.summary,
        &payload.tags,
    )
    .await
    .map_err(logic_err)?;

    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "article_id": article_id }),
        "ok",
    )))
}

pub async fn delete_article(
    State(state): State<AppState>,
    Path(article_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<DeleteBody>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    match payload.mode.as_deref() {
        Some("transfer") => {
            logic::article::handle_delete_article(&state, &session_token, &article_id)
                .await
                .map_err(logic_err)?;
        }
        Some("hard") => {
            logic::article::handle_hard_delete_article(&state, &session_token, &article_id)
                .await
                .map_err(logic_err)?;
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ResponseEnvelope::err(
                    400,
                    "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
                )),
            ));
        }
    }

    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "article_id": article_id }),
        "deleted",
    )))
}
