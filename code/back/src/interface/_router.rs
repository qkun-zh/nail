use axum::Router;
use axum::routing::{get, post};

use crate::infrastructure::state::AppState;
use crate::interface::{challenge, email, session, user};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/challenge/read", get(challenge::create_challenge))
        .route("/email/read", post(email::read_email))
        .route("/user/create", post(user::create_user))
        .route("/session/read", get(session::read_session))
        .route("/session/delete", post(session::delete_session))
        .route("/user/read", get(user::read_users))
        .route("/user/{id}/read", get(user::read_user))
        .route("/user/{id}/update", post(user::update_user))
        .route("/user/{id}/delete", post(user::delete_user))
        .with_state(state)
}
