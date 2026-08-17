use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::infrastructure::state::AppState;
use crate::interface::{
    article, challenge, comment, config, content, role, session, token, user, version,
};
use crate::logic::operations::ROUTE_ACTIONS;

pub const ROUTE_CHALLENGE_CREATE: &str = "/challenge/create";
pub const ROUTE_CONFIG_READ: &str = "/config/read";
pub const ROUTE_TOKEN_CREATE: &str = "/token/create";
pub const ROUTE_USER_CREATE: &str = "/user/create";
pub const ROUTE_USER_READ: &str = "/user/read";
pub const ROUTE_SESSION_READ: &str = "/session/read";
pub const ROUTE_SESSION_DELETE: &str = "/session/delete";
pub const ROUTE_USER_ID_READ: &str = "/user/{id}/read";
pub const ROUTE_USER_ID_UPDATE: &str = "/user/{id}/update";
pub const ROUTE_USER_ID_DELETE: &str = "/user/{id}/delete";
pub const ROUTE_USER_ID_UNDELETE_SOFT: &str = "/user/{id}/undelete-soft";
pub const ROUTE_ARTICLE_READ: &str = "/article/read";
pub const ROUTE_ARTICLE_CREATE: &str = "/article/create";
pub const ROUTE_ARTICLE_ID_READ: &str = "/article/{id}/read";
pub const ROUTE_ARTICLE_ID_UPDATE: &str = "/article/{id}/update";
pub const ROUTE_ARTICLE_ID_DELETE: &str = "/article/{id}/delete";
pub const ROUTE_ARTICLE_ID_UNDELETE_SOFT: &str = "/article/{id}/undelete-soft";
pub const ROUTE_ARTICLE_ID_VERSION_CREATE: &str = "/article/{id}/version/create";
pub const ROUTE_ARTICLE_ID_VERSION_READ: &str = "/article/{id}/version/read";
pub const ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ: &str =
    "/article/{id}/version/{version_id}/content/read";
pub const ROUTE_VERSION_ID_READ: &str = "/version/{id}/read";
pub const ROUTE_VERSION_ID_UPDATE: &str = "/version/{id}/update";
pub const ROUTE_VERSION_ID_DELETE: &str = "/version/{id}/delete";
pub const ROUTE_VERSION_ID_UNDELETE_SOFT: &str = "/version/{id}/undelete-soft";
pub const ROUTE_VERSION_ID_COMMENTS_CREATE: &str = "/version/{id}/comments/create";
pub const ROUTE_COMMENTS_ID_REPLIES_CREATE: &str = "/comments/{id}/replies/create";
pub const ROUTE_VERSION_ID_COMMENTS_READ: &str = "/version/{id}/comments/read";
pub const ROUTE_COMMENT_ID_READ: &str = "/comment/{id}/read";
pub const ROUTE_COMMENT_ID_REPLIES_READ: &str = "/comment/{id}/replies/read";
pub const ROUTE_COMMENT_ID_UPDATE: &str = "/comment/{id}/update";
pub const ROUTE_COMMENT_ID_DELETE: &str = "/comment/{id}/delete";
pub const ROUTE_COMMENT_ID_UNDELETE_SOFT: &str = "/comment/{id}/undelete-soft";
pub const ROUTE_ROLE_CREATE: &str = "/role/create";
pub const ROUTE_ROLE_READ: &str = "/role/read";
pub const ROUTE_ROLE_NAME_READ: &str = "/role/{name}/read";
pub const ROUTE_ROLE_NAME_UPDATE: &str = "/role/{name}/update";
pub const ROUTE_ROLE_NAME_DELETE: &str = "/role/{name}/delete";

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
        .route(ROUTE_CHALLENGE_CREATE, post(challenge::create_challenge))
        .route(ROUTE_CONFIG_READ, get(config::read_config))
        .route(ROUTE_TOKEN_CREATE, post(token::create_token))
        .route(ROUTE_USER_CREATE, post(user::create_user))
        .route(ROUTE_USER_READ, get(user::read_users))
        .route(ROUTE_SESSION_READ, get(session::read_session))
        .route(ROUTE_SESSION_DELETE, post(session::delete_session))
        .route(ROUTE_USER_ID_READ, get(user::read_user))
        .route(ROUTE_USER_ID_UPDATE, post(user::update_user))
        .route(ROUTE_USER_ID_DELETE, post(user::delete_user))
        .route(ROUTE_USER_ID_UNDELETE_SOFT, post(user::undelete_soft_user))
        .route(ROUTE_ARTICLE_READ, get(article::search_articles))
        .route(ROUTE_ARTICLE_CREATE, post(article::create_article))
        .route(ROUTE_ARTICLE_ID_READ, get(article::read_article))
        .route(ROUTE_ARTICLE_ID_UPDATE, post(article::update_article))
        .route(ROUTE_ARTICLE_ID_DELETE, post(article::delete_article))
        .route(
            ROUTE_ARTICLE_ID_UNDELETE_SOFT,
            post(article::undelete_soft_article),
        )
        .route(
            ROUTE_ARTICLE_ID_VERSION_CREATE,
            post(version::create_version),
        )
        .route(ROUTE_ARTICLE_ID_VERSION_READ, get(version::read_versions))
        .route(
            ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ,
            get(content::read_content),
        )
        .route(ROUTE_VERSION_ID_READ, get(version::read_version))
        .route(ROUTE_VERSION_ID_UPDATE, post(version::update_version))
        .route(ROUTE_VERSION_ID_DELETE, post(version::delete_version))
        .route(
            ROUTE_VERSION_ID_UNDELETE_SOFT,
            post(version::undelete_soft_version),
        )
        .route(
            ROUTE_VERSION_ID_COMMENTS_CREATE,
            post(comment::create_comment),
        )
        .route(
            ROUTE_COMMENTS_ID_REPLIES_CREATE,
            post(comment::create_reply),
        )
        .route(ROUTE_VERSION_ID_COMMENTS_READ, get(comment::read_comments))
        .route(ROUTE_COMMENT_ID_READ, get(comment::read_comment))
        .route(
            ROUTE_COMMENT_ID_REPLIES_READ,
            get(comment::read_comment_children),
        )
        .route(ROUTE_COMMENT_ID_UPDATE, post(comment::update_comment))
        .route(ROUTE_COMMENT_ID_DELETE, post(comment::delete_comment))
        .route(
            ROUTE_COMMENT_ID_UNDELETE_SOFT,
            post(comment::undelete_soft_comment),
        )
        .route(ROUTE_ROLE_CREATE, post(role::create_role))
        .route(ROUTE_ROLE_READ, get(role::read_roles))
        .route(ROUTE_ROLE_NAME_READ, get(role::read_role))
        .route(ROUTE_ROLE_NAME_UPDATE, post(role::update_role))
        .route(ROUTE_ROLE_NAME_DELETE, post(role::delete_role))
        .layer(DefaultBodyLimit::max(
            usize::try_from(body_limit).unwrap_or(usize::MAX),
        ))
        .with_state(state)
}
