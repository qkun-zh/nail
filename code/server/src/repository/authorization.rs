#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{Entity, EntityUid, RestrictedExpression};
use database::{Database, EdgeKind, Error, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::comment::{owner_of_comment, version_of_comment};
use crate::repository::role::RoleView;
use crate::repository::schema::{IdRow, PermissionRow, RoleRow};
use crate::repository::version::parent_article_of;

include!(concat!(env!("OUT_DIR"), "/cedar_entities.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Article(String),
    Version(String),
    Comment(String),
    Role(String),
    User(String),
    Tag(String),
    Virtual(String),
}

#[derive(Debug, Clone, Default)]
pub struct UserAuthorization {
    pub roles: Vec<RoleView>,
}

#[derive(Debug, Clone, Default)]
pub struct ArticleAuthorization {
    pub owner_id: String,
}

pub struct AuthAssembly {
    pub principal: EntityUid,
    pub resource: EntityUid,
    pub entities: Vec<Entity>,
}

#[derive(Debug)]
pub enum AssemblyError {
    ResourceNotFound,
    Internal(String),
}

impl std::fmt::Display for AssemblyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResourceNotFound => formatter.write_str("resource not found"),
            Self::Internal(message) => {
                write!(formatter, "authorization assembly failed: {message}")
            }
        }
    }
}

impl std::error::Error for AssemblyError {}

impl From<Error> for AssemblyError {
    fn from(error: Error) -> Self {
        Self::Internal(error.to_string())
    }
}

pub fn read_user_authorization(db: &Database, user_id: &str) -> Result<UserAuthorization, Error> {
    db.read(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(UserAuthorization::default());
        };
        let role_edges = scope.outgoing(user, EdgeKind::UserHoldRole)?;
        let mut authorization = UserAuthorization::default();
        for role_node in role_edges {
            let Some(row) = scope.scope_read_node::<RoleRow>(role_node)? else {
                continue;
            };
            let mut role = RoleView {
                role_name: row.role_name,
                ..Default::default()
            };
            let grants = scope.outgoing(role_node, EdgeKind::RoleGrantPermission)?;
            let permissions = scope.scope_read_nodes::<PermissionRow>(&grants)?;
            role.permissions
                .extend(permissions.into_iter().map(|p| p.permission_name));
            authorization.roles.push(role);
        }
        Ok(authorization)
    })
}

pub fn read_article_authorization(
    db: &Database,
    article_id: &str,
) -> Result<Option<ArticleAuthorization>, Error> {
    db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(None);
        };
        let mut authorization = ArticleAuthorization::default();
        if let Some(owner) = scope
            .incoming(article, EdgeKind::UserAuthorArticle)?
            .first()
            && let Some(row) = scope.scope_read_node::<IdRow>(*owner)?
        {
            authorization.owner_id = row.id;
        }
        Ok(Some(authorization))
    })
}

pub fn assemble_principal(
    db: &Database,
    user_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let authorization = read_user_authorization(db, user_id)
        .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    let principal = user_uid(user_id)?;
    let mut entities: Vec<Entity> = Vec::new();
    let mut seen: HashSet<EntityUid> = HashSet::new();
    let mut role_uids: HashSet<EntityUid> = HashSet::new();

    for role in &authorization.roles {
        let role_uid = role_uid(&role.role_name)?;
        role_uids.insert(role_uid.clone());
        let mut action_parents: HashSet<EntityUid> = HashSet::new();
        for permission in &role.permissions {
            let action_uid = action_uid(permission)?;
            action_parents.insert(action_uid.clone());
            if seen.insert(action_uid.clone()) {
                entities.push(Entity::new_no_attrs(action_uid, HashSet::new()));
            }
        }
        if seen.insert(role_uid.clone()) {
            entities.push(Entity::new_no_attrs(role_uid, action_parents));
        }
    }

    entities.push(Entity::new_no_attrs(principal.clone(), role_uids));
    Ok((principal, entities))
}

pub fn assemble_resource(
    db: &Database,
    resource: Resource,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    match resource {
        Resource::Article(article_id) => {
            let authorization = read_article_authorization(db, &article_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let resource_uid = article_uid(&article_id)?;
            let entity = Entity::new(
                resource_uid.clone(),
                resource_attrs(&authorization.owner_id)?,
                HashSet::new(),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            Ok((resource_uid, vec![entity]))
        }
        Resource::Version(version_id) => {
            let article_id = parent_article_of(db, &version_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            assemble_version_chain(db, &article_id, &version_id)
        }
        Resource::Comment(comment_id) => {
            let version_id = version_of_comment(db, &comment_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let article_id = parent_article_of(db, &version_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let authorization = read_article_authorization(db, &article_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let comment_owner = owner_of_comment(db, &comment_id)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .unwrap_or_default();

            let article_entity_uid = article_uid(&article_id)?;
            let version_entity_uid = version_uid(&version_id)?;
            let comment_entity_uid = comment_uid(&comment_id)?;
            let article_entity = Entity::new(
                article_entity_uid.clone(),
                resource_attrs(&authorization.owner_id)?,
                HashSet::new(),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            let version_entity = Entity::new(
                version_entity_uid.clone(),
                resource_attrs(&authorization.owner_id)?,
                HashSet::from([article_entity_uid]),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            let comment_entity = Entity::new(
                comment_entity_uid.clone(),
                resource_attrs(&comment_owner)?,
                HashSet::from([version_entity_uid]),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            Ok((
                comment_entity_uid,
                vec![article_entity, version_entity, comment_entity],
            ))
        }
        Resource::Role(name) => {
            let resource_uid = role_uid(&name)?;
            let view = crate::repository::role::read_role(db, &name)
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let action_parents: HashSet<EntityUid> = view
                .permissions
                .iter()
                .map(|permission| action_uid(permission))
                .collect::<Result<_, _>>()?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), action_parents);
            Ok((resource_uid, vec![entity]))
        }
        Resource::User(user_id) => {
            let exists = node_exists(db, NodeKind::User, &user_id)?;
            if !exists {
                return Err(AssemblyError::ResourceNotFound);
            }
            let resource_uid = user_uid(&user_id)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
        Resource::Tag(tag_id) => {
            let exists = node_exists(db, NodeKind::Tag, &tag_id)?;
            if !exists {
                return Err(AssemblyError::ResourceNotFound);
            }
            let resource_uid = tag_uid(&tag_id)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
        Resource::Virtual(name) => {
            let resource_uid = virtual_uid(&name)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
    }
}

fn node_exists(db: &Database, kind: NodeKind, business_id: &str) -> Result<bool, Error> {
    db.read(|scope| Ok(scope.resolve(kind, business_id)?.is_some()))
}

pub fn assemble(
    db: &Database,
    user_id: &str,
    resource: Resource,
) -> Result<AuthAssembly, AssemblyError> {
    let (principal, mut principal_entities) = assemble_principal(db, user_id)?;
    let (resource_uid, resource_entities) = assemble_resource(db, resource)?;
    let mut positions: HashMap<EntityUid, usize> = HashMap::new();
    let mut entities: Vec<Entity> =
        Vec::with_capacity(principal_entities.len() + resource_entities.len());
    for entity in principal_entities.drain(..).chain(resource_entities) {
        if let Some(index) = positions.get(&entity.uid()) {
            let index = *index;
            entities[index] = entity;
        } else {
            positions.insert(entity.uid().clone(), entities.len());
            entities.push(entity);
        }
    }
    Ok(AuthAssembly {
        principal,
        resource: resource_uid,
        entities,
    })
}

fn assemble_version_chain(
    db: &Database,
    article_id: &str,
    version_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let authorization = read_article_authorization(db, article_id)
        .map_err(|error| AssemblyError::Internal(error.to_string()))?
        .ok_or(AssemblyError::ResourceNotFound)?;
    let article_entity_uid = article_uid(article_id)?;
    let version_entity_uid = version_uid(version_id)?;
    let article_entity = Entity::new(
        article_entity_uid.clone(),
        resource_attrs(&authorization.owner_id)?,
        HashSet::new(),
    )
    .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    let version_entity = Entity::new(
        version_entity_uid.clone(),
        resource_attrs(&authorization.owner_id)?,
        HashSet::from([article_entity_uid]),
    )
    .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    Ok((version_entity_uid, vec![article_entity, version_entity]))
}

fn parse_uid(text: &str) -> Result<EntityUid, AssemblyError> {
    text.parse::<EntityUid>()
        .map_err(|error| AssemblyError::Internal(format!("invalid entity uid {text:?}: {error}")))
}

fn user_uid(user_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_USER}::\"{user_id}\""))
}

fn role_uid(role_name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_ROLE}::\"{role_name}\""))
}

fn action_uid(action: &str) -> Result<EntityUid, AssemblyError> {
    // Action entities are assembled at runtime only; the schema declares no
    // Action entity, so the type name is a fixed literal.
    parse_uid(&format!("Action::\"{action}\""))
}

fn article_uid(article_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_ARTICLE}::\"{article_id}\""))
}

fn version_uid(version_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_VERSION}::\"{version_id}\""))
}

fn comment_uid(comment_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_COMMENT}::\"{comment_id}\""))
}

fn tag_uid(tag_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_TAG}::\"{tag_id}\""))
}

fn virtual_uid(name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("{CEDAR_ENTITY_VIRTUAL}::\"{name}\""))
}

fn expression(text: &str) -> Result<RestrictedExpression, AssemblyError> {
    RestrictedExpression::from_str(text)
        .map_err(|error| AssemblyError::Internal(format!("invalid expression {text:?}: {error}")))
}

fn resource_attrs(owner_id: &str) -> Result<HashMap<String, RestrictedExpression>, AssemblyError> {
    let owner = if owner_id.is_empty() {
        expression("User::\"\"")?
    } else {
        expression(&user_uid(owner_id)?.to_string())?
    };
    Ok(HashMap::from([("owner".to_string(), owner)]))
}
