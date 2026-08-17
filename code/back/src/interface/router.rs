use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::infrastructure::state::AppState;
use crate::interface::{
    article, challenge, comment, config, content, role, session, token, user, version,
};
use crate::logic::operations::ROUTE_ACTIONS;

pub fn build_router(state: AppState) -> Router {
    let body_limit = state
        .config
        .server
        .max_pdf_size_bytes
        .saturating_add(state.config.server.max_text_field_bytes.saturating_mul(5))
        .saturating_add(64 * 1024);

    for (route, actions) in ROUTE_ACTIONS {
        tracing::debug!(route, actions = ?actions, "route authorization inventory");
    }

    Router::new()
        .route("/challenge/create", post(challenge::create_challenge))
        .route("/config/read", get(config::read_config))
        .route("/token/create", post(token::create_token))
        .route("/user/create", post(user::create_user))
        .route("/session/read", get(session::read_session))
        .route("/session/delete", post(session::delete_session))
        .route("/user/{id}/read", get(user::read_user))
        .route("/user/{id}/update", post(user::update_user))
        .route("/user/{id}/delete", post(user::delete_user))
        .route("/article/read", get(article::search_articles))
        .route("/article/create", post(article::create_article))
        .route("/article/{id}/read", get(article::read_article))
        .route("/article/{id}/update", post(article::update_article))
        .route("/article/{id}/delete", post(article::delete_article))
        .route(
            "/article/{id}/undelete-soft",
            post(article::undelete_soft_article),
        )
        .route(
            "/article/{id}/version/create",
            post(version::create_version),
        )
        .route("/article/{id}/version/read", get(version::read_versions))
        .route(
            "/article/{id}/version/{version_id}/content/read",
            get(content::read_content),
        )
        .route("/version/{id}/read", get(version::read_version))
        .route("/version/{id}/update", post(version::update_version))
        .route("/version/{id}/delete", post(version::delete_version))
        .route(
            "/version/{id}/undelete-soft",
            post(version::undelete_soft_version),
        )
        .route(
            "/version/{id}/comments/create",
            post(comment::create_comment),
        )
        .route("/comments/{id}/replies/create", post(comment::create_reply))
        .route("/version/{id}/comments/read", get(comment::read_comments))
        .route("/comment/{id}/read", get(comment::read_comment))
        .route(
            "/comment/{id}/replies/read",
            get(comment::read_comment_children),
        )
        .route("/comment/{id}/update", post(comment::update_comment))
        .route("/comment/{id}/delete", post(comment::delete_comment))
        .route("/comment/{id}/restore", post(comment::restore_comment))
        .route("/role/create", post(role::create_role))
        .route("/role/read", get(role::read_roles))
        .route("/role/{name}/read", get(role::read_role))
        .route("/role/{name}/update", post(role::update_role))
        .route("/role/{name}/delete", post(role::delete_role))
        .layer(DefaultBodyLimit::max(
            usize::try_from(body_limit).unwrap_or(usize::MAX),
        ))
        .with_state(state)
}
