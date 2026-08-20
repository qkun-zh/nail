use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, patch, post, put};

use crate::infrastructure::state::AppState;
use crate::interface::{
    article, challenge, comment, config, content, role, session, tag, token, user, version,
};
pub const ROUTE_CHALLENGES: &str = "/challenges";
pub const ROUTE_CONFIG: &str = "/config";
pub const ROUTE_TOKENS: &str = "/tokens";
pub const ROUTE_USER: &str = "/user";
pub const ROUTE_USERS: &str = "/users";
pub const ROUTE_SESSION: &str = "/session";
pub const ROUTE_USERS_ID: &str = "/users/{id}";
pub const ROUTE_USERS_ID_RESTORE: &str = "/users/{id}/restore";
pub const ROUTE_ARTICLES: &str = "/articles";
pub const ROUTE_ARTICLES_ID: &str = "/articles/{id}";
pub const ROUTE_ARTICLES_ID_RESTORE: &str = "/articles/{id}/restore";
pub const ROUTE_ARTICLES_ID_VERSIONS: &str = "/articles/{id}/versions";
pub const ROUTE_ARTICLES_ID_VERSIONS_VID_CONTENT: &str =
    "/articles/{id}/versions/{version_id}/content";
pub const ROUTE_VERSIONS_ID: &str = "/versions/{id}";
pub const ROUTE_VERSIONS_ID_RESTORE: &str = "/versions/{id}/restore";
pub const ROUTE_VERSIONS_ID_COMMENTS: &str = "/versions/{id}/comments";
pub const ROUTE_COMMENTS_ID: &str = "/comments/{id}";
pub const ROUTE_COMMENTS_ID_RESTORE: &str = "/comments/{id}/restore";
pub const ROUTE_COMMENTS_ID_REPLIES: &str = "/comments/{id}/replies";
pub const ROUTE_ROLES: &str = "/roles";
pub const ROUTE_ROLES_ID: &str = "/roles/{id}";
pub const ROUTE_TAGS: &str = "/tags";
pub const ROUTE_TAGS_ID: &str = "/tags/{id}";
pub const ROUTE_ARTICLES_ID_TAGS_TID: &str = "/articles/{id}/tags/{tid}";

pub fn build_router(state: AppState) -> Router {
    let body_limit = state.configurator.max_request_body_bytes();

    Router::new()
        .route(ROUTE_CHALLENGES, post(challenge::create_challenge))
        .route(ROUTE_CONFIG, get(config::read_config))
        .route(ROUTE_TOKENS, post(token::create_token))
        .route(ROUTE_USER, get(session::read_session))
        .route(ROUTE_USERS, post(user::create_user))
        .route(ROUTE_USERS, get(user::read_users))
        .route(ROUTE_SESSION, delete(session::delete_session))
        .route(ROUTE_USERS_ID, get(user::read_user))
        .route(ROUTE_USERS_ID, patch(user::update_user))
        .route(ROUTE_USERS_ID, delete(user::delete_user))
        .route(ROUTE_USERS_ID_RESTORE, post(user::undelete_soft_user))
        .route(ROUTE_ARTICLES, get(article::search_articles))
        .route(ROUTE_ARTICLES, post(article::create_article))
        .route(ROUTE_ARTICLES_ID, get(article::read_article))
        .route(ROUTE_ARTICLES_ID, patch(article::update_article))
        .route(ROUTE_ARTICLES_ID, delete(article::delete_article))
        .route(
            ROUTE_ARTICLES_ID_RESTORE,
            post(article::undelete_soft_article),
        )
        .route(ROUTE_ARTICLES_ID_VERSIONS, post(version::create_version))
        .route(ROUTE_ARTICLES_ID_VERSIONS, get(version::read_versions))
        .route(
            ROUTE_ARTICLES_ID_VERSIONS_VID_CONTENT,
            get(content::read_content),
        )
        .route(ROUTE_VERSIONS_ID, get(version::read_version))
        .route(ROUTE_VERSIONS_ID, patch(version::update_version))
        .route(ROUTE_VERSIONS_ID, delete(version::delete_version))
        .route(
            ROUTE_VERSIONS_ID_RESTORE,
            post(version::undelete_soft_version),
        )
        .route(ROUTE_VERSIONS_ID_COMMENTS, post(comment::create_comment))
        .route(ROUTE_VERSIONS_ID_COMMENTS, get(comment::read_comments))
        .route(ROUTE_COMMENTS_ID, get(comment::read_comment))
        .route(ROUTE_COMMENTS_ID, patch(comment::update_comment))
        .route(ROUTE_COMMENTS_ID, delete(comment::delete_comment))
        .route(
            ROUTE_COMMENTS_ID_RESTORE,
            post(comment::undelete_soft_comment),
        )
        .route(ROUTE_COMMENTS_ID_REPLIES, post(comment::create_reply))
        .route(
            ROUTE_COMMENTS_ID_REPLIES,
            get(comment::read_comment_children),
        )
        .route(ROUTE_ROLES, post(role::create_role))
        .route(ROUTE_ROLES, get(role::read_roles))
        .route(ROUTE_ROLES_ID, get(role::read_role))
        .route(ROUTE_ROLES_ID, patch(role::update_role))
        .route(ROUTE_ROLES_ID, delete(role::delete_role))
        .route(ROUTE_TAGS, post(tag::create_tag))
        .route(ROUTE_TAGS, get(tag::read_tags))
        .route(ROUTE_TAGS_ID, get(tag::read_tag))
        .route(ROUTE_TAGS_ID, patch(tag::update_tag))
        .route(ROUTE_TAGS_ID, delete(tag::delete_tag))
        .route(ROUTE_ARTICLES_ID_TAGS_TID, put(tag::apply_tag))
        .route(ROUTE_ARTICLES_ID_TAGS_TID, delete(tag::unapply_tag))
        .layer(DefaultBodyLimit::max(
            usize::try_from(body_limit).unwrap_or(usize::MAX),
        ))
        .with_state(state)
}
