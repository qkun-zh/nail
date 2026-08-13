use axum::Router;
use axum::routing::{get, post};

use crate::infrastructure::state::AppState;
use crate::interface::{challenge, email, registration, session};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/challenge/read", get(challenge::issue))
        .route("/email/read", post(email::read))
        .route("/user/create", post(registration::create))
        .route("/session/read", get(session::read))
        .route("/session/delete", post(session::delete))
        .with_state(state)
}
