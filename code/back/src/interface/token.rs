use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use nail_common::request::CreateTokenRequest;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::AppJson;
use crate::interface::principal::SESSION_TOKEN_HEADER;

pub async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(payload): AppJson<CreateTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session_token = headers
        .get(SESSION_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let data = crate::logic::email::create_token(&state, payload, session_token).await?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}
