use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::pow::Challenge;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::json_response;

pub async fn create_challenge(State(state): State<AppState>) -> impl IntoResponse {
    let challenge = crate::logic::challenge::create_challenge(&state.configurator, &state.cache);
    json_response::<Challenge>(StatusCode::OK, challenge, "ok")
}
