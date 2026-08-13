
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use common::request::DeleteBody;
use common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::api::article::{read_text_field, stream_pdf_field};
use crate::api::{ApiError, logic_err, require_session, strip_table_prefix};
use crate::logic;
use crate::other::AppState;
use crate::other::pdf::PdfUpload;

#[derive(Debug, Default, Deserialize)]
pub struct VersionListParams {
    limit: Option<u64>,
    page: Option<u64>,
}

pub async fn read_versions(
    State(state): State<AppState>,
    Path(article_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<VersionListParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    crate::api::require_session(&state, &headers)?;

    let page_size = state.config.server.search_page_size;
    let max_page_size = state.config.server.max_search_page_size;
    let limit = params.limit.unwrap_or(page_size).min(max_page_size).max(1);
    let page = params
        .page
        .unwrap_or(1)
        .clamp(1, state.config.server.max_page);
    let offset = (page - 1).saturating_mul(limit);

    let (version_list, total) =
        logic::version::handle_read_article_versions(&state, &article_id, limit, offset)
            .await
            .map_err(logic_err)?;

    let version_list_out: Vec<serde_json::Value> = version_list
        .iter()
        .map(|version_entry| {
            let version_id = version_entry
                .get("id")
                .and_then(|v| v.as_str())
                .map(strip_table_prefix)
                .unwrap_or_default();
            let version = version_entry
                .get("version_number")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let created_at_secs = common::time::uuidv7_timestamp_secs(&version_id).unwrap_or(0);
            serde_json::json!({
                "id": version_id,
                "version": version,
                "created_at": created_at_secs,
            })
        })
        .collect();

    let has_more = offset.saturating_add(version_list_out.len() as u64) < total;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({
            "version_list": version_list_out,
            "page": page,
            "total": total,
            "has_next": has_more,
        }),
        "ok",
    )))
}

#[derive(Debug, Default, Deserialize)]
pub struct VersionQueryParams {
    article_id: Option<String>,
    check_if_is_author: Option<bool>,
}

pub async fn read_version(
    State(state): State<AppState>,
    Path(version_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<VersionQueryParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;

    let Some(entry) = logic::version::handle_read_version(
        &state,
        &session_token,
        &version_id,
        params.article_id.as_deref(),
    )
    .await
    .map_err(logic_err)?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ResponseEnvelope::err(404, "version not found")),
        ));
    };
    let created_at =
        common::time::uuidv7_timestamp_secs(&crate::api::strip_table_prefix(&version_id))
            .unwrap_or(0);
    let mut data = serde_json::json!({
        "id": version_id,
        "version": entry.version_number,
        "created_at": created_at,
        "note": entry.note,
    });
    if params.check_if_is_author == Some(true) {
        let is_author =
            logic::author::handle_is_author(&state, &session_token, None, Some(&version_id), None)
                .await
                .map_err(logic_err)?;
        data["is_author"] = serde_json::json!(is_author);
    }
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}

pub async fn create_version(
    State(state): State<AppState>,
    Path(article_id): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ResponseEnvelope<serde_json::Value>>), ApiError> {
    let session_token = crate::api::require_session(&state, &headers)?;

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
            "version" => {
                let max_bytes = state.config.server.max_text_field_bytes as usize;
                version = read_text_field(&mut field, max_bytes).await.map_err(|_| {
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
                let max_bytes = state.config.server.max_text_field_bytes as usize;
                note = read_text_field(&mut field, max_bytes).await.map_err(|_| {
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

    if version.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "version is required")),
        ));
    }
    if upload.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "PDF file is required")),
        ));
    }

    let version_id = logic::version::handle_create_version(
        &state,
        &session_token,
        &article_id,
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
            serde_json::json!({ "version_id": version_id }),
            "created",
        )),
    ))
}

pub async fn update_version(
    State(state): State<AppState>,
    Path(version_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateVersionNoteRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    logic::version::handle_update_version_note(
        &state,
        &session_token,
        &version_id,
        &payload.note,
    )
    .await
    .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "version_id": version_id }),
        "ok",
    )))
}

pub async fn delete_version(
    State(state): State<AppState>,
    Path(version_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<DeleteBody>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    if payload.mode.as_deref() != Some("hard") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(
                400,
                "version delete only supports mode \"hard\"",
            )),
        ));
    }
    logic::version::handle_hard_delete_version(&state, &session_token, &version_id)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "version_id": version_id }),
        "deleted",
    )))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateVersionNoteRequest {
    pub note: String,
}
