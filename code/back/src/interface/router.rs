use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::infrastructure::state::AppState;
use crate::interface::{article, challenge, email, session, user, version};

pub fn build_router(state: AppState) -> Router {
    let body_limit = state
        .config
        .server
        .max_pdf_size_bytes
        .saturating_add(
            state
                .config
                .server
                .max_text_field_bytes
                .saturating_mul(5),
        )
        .saturating_add(64 * 1024);

    Router::new()
        .route("/challenge/read", get(challenge::create_challenge))
        .route("/email/read", post(email::read_email))
        .route("/user/create", post(user::create_user))
        .route("/session/read", get(session::read_session))
        .route("/session/delete", post(session::delete_session))
        .route("/user/{id}/read", get(user::read_user))
        .route("/user/read", get(user::read_users))
        .route("/user/{id}/update", post(user::update_user))
        .route("/user/{id}/delete", post(user::delete_user))
        .route("/article/read", get(article::read_articles))
        .route("/article/create", post(article::create_article))
        .route("/article/{id}/read", get(article::read_article))
        .route("/article/{id}/update", post(article::update_article))
        .route("/article/{id}/delete", post(article::delete_article))
        .route("/article/{id}/version/create", post(version::create_version))
        .route("/article/{id}/version/read", get(version::read_versions))
        .route("/version/{id}/read", get(version::read_version))
        .route("/version/{id}/update", post(version::update_version))
        .route("/version/{id}/delete", post(version::delete_version))
        .layer(DefaultBodyLimit::max(body_limit as usize))
        .with_state(state)
}
