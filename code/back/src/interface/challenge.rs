use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::pow::Challenge;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::json_response;

pub async fn create_challenge(State(state): State<AppState>) -> impl IntoResponse {
    let challenge = crate::logic::challenge::create_challenge(&state.config.server, &state.caches);
    json_response::<Challenge>(StatusCode::OK, challenge, "ok")
}
