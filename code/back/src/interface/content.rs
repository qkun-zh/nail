use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::infrastructure::pdf::sanitize_attachment_filename;
use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::{AppPath, AppQuery};
use crate::interface::principal::Principal;
use crate::logic::error::LogicError;
use nail_common::response::content::MintUrl;

#[derive(Debug, Default, Deserialize)]
pub struct ContentReadParams {
    pub mode: Option<String>,
    pub token: Option<String>,
}

pub async fn read_content(
    State(state): State<AppState>,
    principal: Principal,
    AppPath((article_id, version_id)): AppPath<(String, String)>,
    AppQuery(params): AppQuery<ContentReadParams>,
) -> Result<Response, ApiError> {
    if matches!(params.mode.as_deref(), Some("download")) {
        let url = crate::logic::download::mint_download_token(
            &state,
            &principal.user_id,
            &article_id,
            &version_id,
        )
        .await
        .map_err(ApiError::from_logic)?;
        return Ok(json_response(StatusCode::OK, MintUrl { url }, "ok"));
    }

    let Some(token) = params.token.as_deref() else {
        return Err(ApiError::bad_request("missing download token"));
    };
    let path = crate::logic::download::consume_download_token(
        &state,
        &principal.user_id,
        &article_id,
        &version_id,
        token,
    )
    .await
    .map_err(ApiError::from_logic)?;

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
                    ApiError::from_logic(LogicError::internal(format!(
                        "failed to build pdf response: {error}"
                    )))
                })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(ApiError::from_logic(
            LogicError::not_found("PDF file not found"),
        )),
        Err(error) => {
            tracing::error!(path = %path.display(), error = %error, "failed to open pdf file");
            Err(ApiError::from_logic(LogicError::internal(
                "failed to open PDF file",
            )))
        }
    }
}
