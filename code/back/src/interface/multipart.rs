use std::collections::HashMap;

use nail_common::hash::PdfHasher;
use tokio::io::AsyncWriteExt;

use crate::infrastructure::pdf::{PdfStreamGuard, PdfUpload, TempPdf};
use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;

pub(crate) async fn read_text_field(
    state: &AppState,
    field: axum::extract::multipart::Field<'_>,
) -> Result<String, ApiError> {
    let bytes = field
        .bytes()
        .await
        .map_err(|error| map_multipart_error(&error))?;
    if bytes.len() as u64 > state.config.server.max_text_field_bytes {
        return Err(ApiError::bad_request("text field too large"));
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| ApiError::bad_request("text field must be UTF-8"))
}

pub(crate) async fn stream_pdf_field(
    state: &AppState,
    mut field: axum::extract::multipart::Field<'_>,
) -> Result<PdfUpload, ApiError> {
    let temp_path = std::path::Path::new(&state.config.server.pdf_storage_path)
        .join(".tmp")
        .join(format!("{}.pdf", uuid::Uuid::now_v7()));
    let temp = TempPdf::new(temp_path.clone());
    let mut file = tokio::fs::File::create(&temp_path).await.map_err(|error| {
        ApiError::from(crate::logic::error::LogicError::internal(format!(
            "failed to create temp pdf: {error}"
        )))
    })?;
    let mut guard = PdfStreamGuard::new(state.config.server.max_pdf_size_bytes);
    let mut hasher = PdfHasher::new();

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| map_multipart_error(&error))?
    {
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

pub(crate) fn map_multipart_error(error: &axum::extract::multipart::MultipartError) -> ApiError {
    tracing::debug!(error = %error, "multipart form rejected");
    ApiError {
        status: error.status(),
        message: "invalid multipart form data".to_string(),
    }
}

pub(crate) enum MultipartField {
    Pdf(PdfUpload),
    Text(String),
}

impl MultipartField {
    pub(crate) fn into_text(self) -> Option<String> {
        match self {
            Self::Text(value) => Some(value),
            Self::Pdf(_) => None,
        }
    }

    pub(crate) fn into_pdf(self) -> Option<PdfUpload> {
        match self {
            Self::Pdf(upload) => Some(upload),
            Self::Text(_) => None,
        }
    }
}

pub(crate) async fn collect_fields(
    state: &AppState,
    mut multipart: axum::extract::Multipart,
    pdf_field_names: &[&str],
    text_field_names: &[&str],
) -> Result<HashMap<String, MultipartField>, ApiError> {
    let mut fields = HashMap::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| map_multipart_error(&error))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if pdf_field_names.contains(&name.as_str()) {
            fields.insert(
                name,
                MultipartField::Pdf(stream_pdf_field(state, field).await?),
            );
        } else if text_field_names.contains(&name.as_str()) {
            fields.insert(
                name,
                MultipartField::Text(read_text_field(state, field).await?),
            );
        } else {
            drop(field);
        }
    }
    Ok(fields)
}
