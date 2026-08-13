use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use nail_common::response::ResponseEnvelope;

use crate::logic::error::LogicError;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }
}

impl From<LogicError> for ApiError {
    fn from(error: LogicError) -> Self {
        match &error {
            LogicError::Internal(_) => {
                tracing::error!(error = %error, "request failed with internal error");
            }
            LogicError::Forbidden(_) => {
                tracing::warn!(error = %error, "forbidden request (valid session, wrong actor)");
            }
            _ => {}
        }
        let (status, message) = error.into_pair();
        Self { status, message }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = self.status.as_u16();
        (
            self.status,
            Json(ResponseEnvelope::<serde_json::Value>::err(code, self.message)),
        )
            .into_response()
    }
}
