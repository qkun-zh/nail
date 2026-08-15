use axum::Json;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::{get, post};
use common::response::ResponseEnvelope;

use crate::other::AppState;

mod article;
mod article_view;
mod authenticate;
mod comment;
mod meta;
mod role;
mod user;
mod version;

pub(crate) type ApiError = (StatusCode, Json<ResponseEnvelope<serde_json::Value>>);

pub(crate) fn logic_err(e: crate::logic::error::LogicError) -> ApiError {
    if let crate::logic::error::LogicError::Internal(_) = &e {
        tracing::error!(error = %e, "request failed with internal error");
    }
    if let crate::logic::error::LogicError::Forbidden(_) = &e {
        tracing::warn!(error = %e, "forbidden request (valid session, wrong actor)");
    }
    let (code, message) = e.into_pair();
    (code, Json(ResponseEnvelope::err(code.as_u16(), message)))
}

pub const SESSION_TOKEN_HEADER: &str = "session-token";

pub(crate) fn get_session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(SESSION_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub(crate) fn strip_table_prefix(record_id: &str) -> String {
    crate::repo::util::strip_record_id(record_id)
}

pub(crate) fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, ApiError> {
    let token = get_session_token(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ResponseEnvelope::err(
                401,
                "missing session-token header",
            )),
        )
    })?;
    crate::logic::authenticate::authenticate_session(state, &token).map_err(logic_err)?;
    Ok(token)
}

pub(crate) fn sanitize_attachment_filename(filename: &str) -> String {
    let safe: String = filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    if safe.is_empty() {
        "article.pdf".to_string()
    } else {
        safe
    }
}

pub(crate) async fn serve_pdf_file(path: &str) -> Result<axum::response::Response, ApiError> {
    match tokio::fs::File::open(path).await {
        Ok(file) => {
            let filename = sanitize_attachment_filename(
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("article.pdf"),
            );
            let stream = tokio_util::io::ReaderStream::new(file);
            let response = axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "application/pdf")
                .header(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(axum::body::Body::from_stream(stream))
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ResponseEnvelope::err(
                            500,
                            format!("failed to build response: {e}"),
                        )),
                    )
                })?;
            Ok(response)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err((
            StatusCode::NOT_FOUND,
            Json(ResponseEnvelope::err(404, "PDF file not found")),
        )),
        Err(e) => {
            tracing::error!(error = %e, path = %path, "failed to open pdf file");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseEnvelope::err(500, "failed to open PDF file")),
            ))
        }
    }
}

const MULTIPART_OVERHEAD_BYTES: usize = 64 * 1024;

const MULTIPART_MAX_TEXT_FIELDS: usize = 5;

pub fn router(state: crate::other::AppState) -> anyhow::Result<Router> {
    let body_limit = state.config.server.max_pdf_size_bytes as usize
        + MULTIPART_MAX_TEXT_FIELDS * state.config.server.max_text_field_bytes as usize
        + MULTIPART_OVERHEAD_BYTES;
    Ok(Router::new()
        .route("/challenge/read", get(authenticate::issue_challenge))
        .route("/config/read", get(meta::read_config))
        .route("/email/read", post(authenticate::email_read))
        .route("/user/create", post(authenticate::redeem_token))
        .route("/session/read", get(authenticate::verify_session))
        .route("/session/delete", post(user::logout))
        .route("/user/{id}/read", get(user::read_user))
        .route("/user/{id}/update", post(user::update_user))
        .route("/user/{id}/delete", post(user::delete_user))
        .route("/user/read", get(user::read_users))
        .route("/article/read", get(article::read_articles))
        .route("/article/create", post(article::create_article))
        .route("/article/{id}/read", get(article::read_article))
        .route("/article/{id}/update", post(article::update_article))
        .route("/article/{id}/delete", post(article::delete_article))
        .route(
            "/article/{id}/version/{version_id}/content/read",
            get(article::serve_public_pdf),
        )
        .route("/article/{id}/version/create", post(version::create_version))
        .route("/article/{id}/version/read", get(version::read_versions))
        .route("/version/{id}/read", get(version::read_version))
        .route("/version/{id}/update", post(version::update_version))
        .route("/version/{id}/delete", post(version::delete_version))
        .route("/version/{id}/comments/create", post(comment::create_comment))
        .route("/comments/{id}/replies/create", post(comment::create_reply))
        .route("/version/{id}/comments/read", get(comment::read_comments))
        .route("/comment/{id}/update", post(comment::update_comment))
        .route("/comment/{id}/delete", post(comment::delete_comment))
        .route("/role/create", post(role::create_role))
        .route("/role/read", get(role::read_roles))
        .route("/role/{name}/read", get(role::read_role))
        .route("/role/{name}/update", post(role::update_role))
        .route("/role/{name}/delete", post(role::delete_role))
        .layer(DefaultBodyLimit::max(body_limit))
        .with_state(state))
}
