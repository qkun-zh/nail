use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use nail_common::request::EmailReadRequest;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::SESSION_TOKEN_HEADER;

#[derive(Debug, Deserialize)]
pub struct EmailReadQuery {
    pub intent: Option<String>,
}

pub async fn read_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EmailReadQuery>,
    Json(payload): Json<EmailReadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(intent_value) = query.intent.as_deref() else {
        return Err(ApiError::bad_request("email intent is required"));
    };
    let Some(intent) = crate::logic::email::parse_intent(intent_value) else {
        return Err(ApiError::bad_request("invalid email intent"));
    };
    let session_token = headers
        .get(SESSION_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let data = crate::logic::email::read_email(&state, intent, payload, session_token).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}
