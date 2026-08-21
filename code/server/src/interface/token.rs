use axum::extract::State;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use common::request::CreateTokenRequest;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::AppJson;
use crate::interface::principal::read_session_token;

pub async fn create_token(
    State(state): State<AppState>,
    parts: Parts,
    AppJson(payload): AppJson<CreateTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session_token = read_session_token(&parts);
    let data = crate::logic::email::create_token(&state, payload, session_token)
        .await
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, data, "ok"))
}
