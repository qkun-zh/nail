use axum::Json;
use axum::extract::State;
use nail_common::pow::Challenge;
use nail_common::response::ResponseEnvelope;

use crate::infrastructure::state::AppState;

pub async fn create_challenge(
    State(state): State<AppState>,
) -> Json<ResponseEnvelope<Challenge>> {
    Json(ResponseEnvelope::ok(
        200,
        crate::logic::challenge::create_challenge(&state.config.server, &state.caches),
        "ok",
    ))
}
