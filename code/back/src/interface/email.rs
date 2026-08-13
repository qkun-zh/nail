use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use nail_common::request::EmailReadRequest;
use nail_common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;
use crate::interface::principal::SESSION_TOKEN_HEADER;

#[derive(Debug, Deserialize)]
pub struct EmailReadQuery {
    pub intent: Option<String>,
}

pub async fn read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EmailReadQuery>,
    Json(payload): Json<EmailReadRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
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
    let data = crate::logic::email::handle_email_read(&state, intent, payload, session_token).await?;
    Ok(Json(ResponseEnvelope::ok(200, data, "ok")))
}
