use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::infrastructure::pdf::sanitize_attachment_filename;
use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;
use crate::logic::error::LogicError;
use nail_common::response::content::MintUrl;

#[derive(Debug, Default, Deserialize)]
pub struct ContentReadParams {
    pub download: Option<String>,
    pub token: Option<String>,
}

pub async fn read_content(
    State(state): State<AppState>,
    principal: Principal,
    Path((article_id, version_id)): Path<(String, String)>,
    Query(params): Query<ContentReadParams>,
) -> Result<Response, ApiError> {
    if matches!(params.download.as_deref(), Some("1") | Some("true")) {
        let url = crate::logic::download::mint_download_token(
            &state,
            &principal.user_id,
            &article_id,
            &version_id,
        )
        .await?;
        return Ok(json_response(StatusCode::OK, MintUrl { url }, "ok"));
    }

    let path = if let Some(token) = params.token.as_deref() {
        crate::logic::download::consume_download_token(
            &state,
            &principal.user_id,
            &article_id,
            &version_id,
            token,
        )
        .await?
    } else {
        crate::logic::download::resolve_version_pdf_path(&state, &article_id, &version_id).await?
    };

    serve_pdf_file(&path).await
}

async fn serve_pdf_file(path: &std::path::Path) -> Result<Response, ApiError> {
    match tokio::fs::File::open(path).await {
        Ok(file) => {
            let filename = sanitize_attachment_filename(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("article.pdf"),
            );
            let stream = tokio_util::io::ReaderStream::new(file);
            Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "application/pdf")
                .header(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\""),
                )
                .body(axum::body::Body::from_stream(stream))
                .map_err(|error| {
                    ApiError::from(LogicError::internal(format!(
                        "failed to build pdf response: {error}"
                    )))
                })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ApiError::from(LogicError::not_found("PDF file not found")))
        }
        Err(error) => {
            tracing::error!(path = %path.display(), error = %error, "failed to open pdf file");
            Err(ApiError::from(LogicError::internal("failed to open PDF file")))
        }
    }
}
