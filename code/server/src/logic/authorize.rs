use authorizer::RequestContext;
use database::NodeKind;

use crate::infrastructure::authorizer::AuthorizationError;
use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UNDELETE_SOFT, PERMISSION_COMMENT_READ,
    PERMISSION_COMMENT_UNDELETE_SOFT, PERMISSION_ROLE_READ, PERMISSION_TAG_READ,
    PERMISSION_USER_READ, PERMISSION_USER_UNDELETE_SOFT, PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UNDELETE_SOFT,
};

impl From<AuthorizationError> for LogicError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Denied => LogicError::forbidden("you are denied"),
            AuthorizationError::ResourceNotFound => LogicError::not_found("resource not found"),
            AuthorizationError::BadRequest(_) => {
                LogicError::bad_request("invalid authorization request")
            }
            AuthorizationError::Internal(msg) => LogicError::internal(msg),
        }
    }
}

pub fn require_visible_if_soft_deleted(
    state: &AppState,
    actor_id: &str,
    kind: NodeKind,
    business_id: &str,
    undelete_action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    if crate::repository::delete::is_soft_deleted(&state.database, kind, business_id)?
        && state
            .authorizer
            .authorize(actor_id, undelete_action, resource)
            .is_err()
    {
        return Err(LogicError::not_found(not_found_message));
    }
    Ok(())
}

pub fn authorize(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    state
        .authorizer
        .authorize(actor_id, action, resource)
        .map_err(LogicError::from)
}

pub fn authorize_anonymous(
    state: &AppState,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    state
        .authorizer
        .authorize("anonymous", action, resource)
        .map_err(LogicError::from)
}

pub fn authorize_or(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    match authorize(state, actor_id, action, resource) {
        Ok(()) => Ok(()),
        Err(LogicError::NotFound(_)) => Err(LogicError::not_found(not_found_message)),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy)]
pub enum EntityRef<'a> {
    Article(&'a str),
    Version(&'a str),
    Comment(&'a str),
    User(&'a str),
    Tag(&'a str),
    Role(&'a str),
}

impl EntityRef<'_> {
    pub fn id(&self) -> &str {
        match *self {
            EntityRef::Article(id)
            | EntityRef::Version(id)
            | EntityRef::Comment(id)
            | EntityRef::User(id)
            | EntityRef::Tag(id)
            | EntityRef::Role(id) => id,
        }
    }

    pub fn resource(&self) -> Resource {
        match *self {
            EntityRef::Article(id) => Resource::Article(id.to_string()),
            EntityRef::Version(id) => Resource::Version(id.to_string()),
            EntityRef::Comment(id) => Resource::Comment(id.to_string()),
            EntityRef::User(id) => Resource::User(id.to_string()),
            EntityRef::Tag(id) => Resource::Tag(id.to_string()),
            EntityRef::Role(id) => Resource::Role(id.to_string()),
        }
    }

    pub fn not_found_message(&self) -> &'static str {
        match *self {
            EntityRef::Article(_) => "article not found",
            EntityRef::Version(_) => "version not found",
            EntityRef::Comment(_) => "comment not found",
            EntityRef::User(_) => "user not found",
            EntityRef::Tag(_) => "tag not found",
            EntityRef::Role(_) => "role not found",
        }
    }

    pub fn read_permission(&self) -> &'static str {
        match *self {
            EntityRef::Article(_) => PERMISSION_ARTICLE_READ,
            EntityRef::Version(_) => PERMISSION_VERSION_READ,
            EntityRef::Comment(_) => PERMISSION_COMMENT_READ,
            EntityRef::User(_) => PERMISSION_USER_READ,
            EntityRef::Tag(_) => PERMISSION_TAG_READ,
            EntityRef::Role(_) => PERMISSION_ROLE_READ,
        }
    }

    pub fn visibility(&self) -> Option<(NodeKind, &'static str)> {
        match *self {
            EntityRef::Article(_) => Some((NodeKind::Article, PERMISSION_ARTICLE_UNDELETE_SOFT)),
            EntityRef::Version(_) => Some((NodeKind::Version, PERMISSION_VERSION_UNDELETE_SOFT)),
            EntityRef::Comment(_) => Some((NodeKind::Comment, PERMISSION_COMMENT_UNDELETE_SOFT)),
            EntityRef::User(_) => Some((NodeKind::User, PERMISSION_USER_UNDELETE_SOFT)),
            EntityRef::Tag(_) | EntityRef::Role(_) => None,
        }
    }
}

pub fn authorize_global(state: &AppState, actor_id: &str, action: &str) -> Result<(), LogicError> {
    authorize(
        state,
        actor_id,
        action,
        &Resource::Virtual("any".to_string()),
    )
}

pub fn authorize_entity(
    state: &AppState,
    actor_id: &str,
    action: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    authorize(state, actor_id, action, &entity.resource())
}

/// Authorization honouring a [`RequestContext`]. Only callers that vouch for
/// the request metadata (the email-confirmed deregistration flow) pass a
/// non-default context.
pub fn authorize_entity_ctx(
    state: &AppState,
    actor_id: &str,
    action: &str,
    entity: EntityRef<'_>,
    context: RequestContext,
) -> Result<(), LogicError> {
    state
        .authorizer
        .authorize_ctx(actor_id, action, &entity.resource(), context)
        .map_err(LogicError::from)
}

pub fn authorize_entity_or(
    state: &AppState,
    actor_id: &str,
    action: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    authorize_or(
        state,
        actor_id,
        action,
        &entity.resource(),
        entity.not_found_message(),
    )
}

pub fn require_entity_visible(
    state: &AppState,
    actor_id: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    let Some((entity_type, undelete_action)) = entity.visibility() else {
        return Ok(());
    };
    require_visible_if_soft_deleted(
        state,
        actor_id,
        entity_type,
        entity.id(),
        undelete_action,
        &entity.resource(),
        entity.not_found_message(),
    )
}

pub fn require_entity_readable(
    state: &AppState,
    actor_id: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    authorize_entity_or(state, actor_id, entity.read_permission(), entity)?;
    require_entity_visible(state, actor_id, entity)
}
