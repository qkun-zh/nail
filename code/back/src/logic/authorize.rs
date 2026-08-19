use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::authorization::{Resource, assemble, assemble_resource};
use crate::repository::role::{
    PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UNDELETE_SOFT, PERMISSION_COMMENT_READ,
    PERMISSION_COMMENT_UNDELETE_SOFT, PERMISSION_ROLE_READ, PERMISSION_TAG_READ,
    PERMISSION_USER_READ, PERMISSION_USER_UNDELETE_SOFT, PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UNDELETE_SOFT,
};
use crate::repository::schema::{
    ENTITY_TYPE_ARTICLE, ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION,
};

pub async fn require_visible_if_soft_deleted(
    state: &AppState,
    actor_id: &str,
    entity_type: &str,
    business_id: &str,
    undelete_action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    if crate::repository::delete::is_soft_deleted(&state.graph, entity_type, business_id).await?
        && authorize(state, actor_id, undelete_action, resource)
            .await
            .is_err()
    {
        return Err(LogicError::not_found(not_found_message));
    }
    Ok(())
}

pub async fn authorize(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    let assembly = assemble(&state.graph, actor_id, resource.clone()).await?;
    let allowed = crate::infrastructure::cedar::decide(
        &assembly.principal,
        action,
        &assembly.resource,
        assembly.entities,
    )
    .map_err(|error| LogicError::internal(format!("authorization evaluation failed: {error}")))?;
    if allowed {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

pub async fn authorize_anonymous(
    state: &AppState,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    let (resource_uid, resource_entities) =
        assemble_resource(&state.graph, resource.clone()).await?;
    let principal = "User::\"anonymous\""
        .parse::<cedar_policy::EntityUid>()
        .map_err(|error| LogicError::internal(format!("invalid anonymous principal: {error}")))?;
    let allowed =
        crate::infrastructure::cedar::decide(&principal, action, &resource_uid, resource_entities)
            .map_err(|error| {
                LogicError::internal(format!("authorization evaluation failed: {error}"))
            })?;
    if allowed {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

pub async fn authorize_or(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    match authorize(state, actor_id, action, resource).await {
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

    pub fn visibility(&self) -> Option<(&'static str, &'static str)> {
        match *self {
            EntityRef::Article(_) => Some((ENTITY_TYPE_ARTICLE, PERMISSION_ARTICLE_UNDELETE_SOFT)),
            EntityRef::Version(_) => Some((ENTITY_TYPE_VERSION, PERMISSION_VERSION_UNDELETE_SOFT)),
            EntityRef::Comment(_) => Some((ENTITY_TYPE_COMMENT, PERMISSION_COMMENT_UNDELETE_SOFT)),
            EntityRef::User(_) => Some((ENTITY_TYPE_USER, PERMISSION_USER_UNDELETE_SOFT)),
            EntityRef::Tag(_) | EntityRef::Role(_) => None,
        }
    }
}

pub async fn authorize_global(
    state: &AppState,
    actor_id: &str,
    action: &str,
) -> Result<(), LogicError> {
    authorize(
        state,
        actor_id,
        action,
        &Resource::Virtual("any".to_string()),
    )
    .await
}

pub async fn authorize_entity(
    state: &AppState,
    actor_id: &str,
    action: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    authorize(state, actor_id, action, &entity.resource()).await
}

pub async fn authorize_entity_or(
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
    .await
}

pub async fn require_entity_visible(
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
    .await
}

pub async fn require_entity_readable(
    state: &AppState,
    actor_id: &str,
    entity: EntityRef<'_>,
) -> Result<(), LogicError> {
    authorize_entity_or(state, actor_id, entity.read_permission(), entity).await?;
    require_entity_visible(state, actor_id, entity).await
}
