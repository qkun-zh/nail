use axum::Json;
use axum::extract::State;
use nail_common::pow::Challenge;
use nail_common::response::ResponseEnvelope;

use crate::infrastructure::state::AppState;

pub async fn issue(
    State(state): State<AppState>,
) -> Json<ResponseEnvelope<Challenge>> {
    Json(ResponseEnvelope::ok(
        200,
        crate::logic::challenge::issue_challenge(&state.config.server, &state.caches),
        "ok",
    ))
}
