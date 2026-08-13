use axum::Json;
use axum::extract::State;
use nail_common::request::TokenRequest;
use nail_common::response::ResponseEnvelope;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = crate::logic::authenticate::handle_token_exchange(&state, &payload.pow).await?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "session_token": session_token }),
        "ok",
    )))
}
