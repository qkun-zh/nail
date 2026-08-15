
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{Entity, EntityUid, RestrictedExpression};

use crate::repo;
use crate::repo::db::DbHandle;
use crate::repo::types::VISIBILITY_PRIVATE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Article(String),
    Version(String),
    Comment(String),
    #[allow(dead_code)]
    System(String),
}

impl Resource {}

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssemblyError::ResourceNotFound => write!(f, "resource not found"),
            AssemblyError::Internal(e) => write!(f, "authorization assembly failed: {e}"),
        }
    }
}
impl std::error::Error for AssemblyError {}


fn parse_uid(text: &str) -> Result<EntityUid, AssemblyError> {
    text.parse::<EntityUid>()
        .map_err(|e| AssemblyError::Internal(format!("invalid entity uid {text:?}: {e}")))
}

fn user_uid(user_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("User::\"{user_id}\""))
}

fn role_uid(role_name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Role::\"{role_name}\""))
}

fn action_uid(action: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Action::\"{action}\""))
}

fn tag_uid(tag_name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Tag::\"{tag_name}\""))
}

fn article_uid(article_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Article::\"{article_id}\""))
}

fn version_uid(version_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Version::\"{version_id}\""))
}

fn comment_uid(comment_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Comment::\"{comment_id}\""))
}

fn visibility_uid(visibility: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Visibility::\"{visibility}\""))
}

fn expression(text: &str) -> Result<RestrictedExpression, AssemblyError> {
    RestrictedExpression::from_str(text)
        .map_err(|e| AssemblyError::Internal(format!("invalid expression {text:?}: {e}")))
}

fn resource_attrs(
    owner_id: &str,
    visibility: &str,
    tag_names: &[String],
) -> Result<HashMap<String, RestrictedExpression>, AssemblyError> {
    let owner = if owner_id.is_empty() {
        expression("User::\"\"")?
    } else {
        expression(&user_uid(owner_id)?.to_string())?
    };
    let visibility = expression(&visibility_uid(visibility)?.to_string())?;
    let scopes = if tag_names.is_empty() {
        expression("[]")?
    } else {
        let joined: Vec<String> = tag_names
            .iter()
            .map(|t| tag_uid(t).map(|uid| uid.to_string()))
            .collect::<Result<_, _>>()?;
        expression(&format!("[{}]", joined.join(", ")))?
    };
    Ok(HashMap::from([
        ("owner".to_string(), owner),
        ("visibility".to_string(), visibility),
        ("required_scopes".to_string(), scopes),
    ]))
}

pub async fn assemble_principal(
    db: &DbHandle,
    user_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let auth = repo::authorization::read_user_authorization(db, user_id)
        .await
        .map_err(|e| AssemblyError::Internal(e.to_string()))?;
    let principal = user_uid(user_id)?;
    let mut entities: Vec<Entity> = Vec::new();
    let mut seen: HashSet<EntityUid> = HashSet::new();
    let mut role_uids: HashSet<EntityUid> = HashSet::new();
    let mut scopes: Vec<String> = Vec::new();
    let mut global_role = false;

    for role in &auth.roles {
        let role_id = role_uid(&role.role_name)?;
        role_uids.insert(role_id.clone());
        let mut action_parents: HashSet<EntityUid> = HashSet::new();
        for permission in &role.permissions {
            let action_id = action_uid(permission)?;
            action_parents.insert(action_id.clone());
            if seen.insert(action_id.clone()) {
                entities.push(
                    Entity::new(action_id, HashMap::new(), HashSet::new())
                        .map_err(|e| AssemblyError::Internal(e.to_string()))?,
                );
            }
        }
        if seen.insert(role_id.clone()) {
            entities.push(
                Entity::new(role_id, HashMap::new(), action_parents)
                    .map_err(|e| AssemblyError::Internal(e.to_string()))?,
            );
        }
        if role.scopes.is_empty() {
            global_role = true;
        }
        scopes.extend(role.scopes.iter().cloned());
    }

    let global_role_expr = expression(if global_role { "true" } else { "false" })?;
    let scopes_expr = if scopes.is_empty() {
        expression("[]")?
    } else {
        let joined: Vec<String> = scopes
            .iter()
            .map(|t| tag_uid(t).map(|uid| uid.to_string()))
            .collect::<Result<_, _>>()?;
        expression(&format!("[{}]", joined.join(", ")))?
    };
    let user_entity = Entity::new(
        principal.clone(),
        HashMap::from([
            ("global_role".to_string(), global_role_expr),
            ("scopes".to_string(), scopes_expr),
        ]),
        role_uids,
    )
    .map_err(|e| AssemblyError::Internal(e.to_string()))?;
    entities.push(user_entity);
    Ok((principal, entities))
}

pub async fn assemble_resource(
    db: &DbHandle,
    resource: Resource,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    match resource {
        Resource::Article(article_id) => {
            let Some(auth) = repo::authorization::read_article_authorization(db, &article_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
            else {
                return Err(AssemblyError::ResourceNotFound);
            };
            let visibility = auth
                .visibility
                .unwrap_or_else(|| VISIBILITY_PRIVATE.to_string());
            let resource_id = article_uid(&article_id)?;
            let entity = Entity::new(
                resource_id.clone(),
                resource_attrs(&auth.owner_id, &visibility, &auth.tag_names)?,
                HashSet::new(),
            )
            .map_err(|e| AssemblyError::Internal(e.to_string()))?;
            Ok((resource_id, vec![entity]))
        }
        Resource::Version(version_id) => {
            let Some(article_id) = repo::article::find_article_id_by_version(db, &version_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
            else {
                return Err(AssemblyError::ResourceNotFound);
            };
            assemble_version_chain(db, &article_id, &version_id).await
        }
        Resource::Comment(comment_id) => {
            let Some(version_id) = repo::authorization::find_version_id_by_comment(db, &comment_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
            else {
                return Err(AssemblyError::ResourceNotFound);
            };
            let Some(article_id) = repo::article::find_article_id_by_version(db, &version_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
            else {
                return Err(AssemblyError::ResourceNotFound);
            };
            let Some(auth) = repo::authorization::read_article_authorization(db, &article_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
            else {
                return Err(AssemblyError::ResourceNotFound);
            };
            let visibility = auth
                .visibility
                .unwrap_or_else(|| VISIBILITY_PRIVATE.to_string());
            let comment_owner = repo::comment::find_comment_author_id(db, &comment_id)
                .await
                .map_err(|e| AssemblyError::Internal(e.to_string()))?
                .unwrap_or_default();
            let article_id_uid = article_uid(&article_id)?;
            let version_id_uid = version_uid(&version_id)?;
            let comment_id_uid = comment_uid(&comment_id)?;
            let mut entities = Vec::new();
            let article_entity = Entity::new(
                article_id_uid.clone(),
                resource_attrs(&auth.owner_id, &visibility, &auth.tag_names)?,
                HashSet::new(),
            )
            .map_err(|e| AssemblyError::Internal(e.to_string()))?;
            let version_entity = Entity::new(
                version_id_uid.clone(),
                resource_attrs(&auth.owner_id, &visibility, &auth.tag_names)?,
                HashSet::from([article_id_uid]),
            )
            .map_err(|e| AssemblyError::Internal(e.to_string()))?;
            let comment_entity = Entity::new(
                comment_id_uid.clone(),
                resource_attrs(&comment_owner, &visibility, &auth.tag_names)?,
                HashSet::from([version_id_uid]),
            )
            .map_err(|e| AssemblyError::Internal(e.to_string()))?;
            entities.extend([article_entity, version_entity, comment_entity]);
            Ok((comment_id_uid, entities))
        }
        Resource::System(system_name) => {
            let resource_id = parse_uid(&format!("System::\"{system_name}\""))?;
            let entity = Entity::new(resource_id.clone(), HashMap::new(), HashSet::new())
                .map_err(|e| AssemblyError::Internal(e.to_string()))?;
            Ok((resource_id, vec![entity]))
        }
    }
}

async fn assemble_version_chain(
    db: &DbHandle,
    article_id: &str,
    version_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let Some(auth) = repo::authorization::read_article_authorization(db, article_id)
        .await
        .map_err(|e| AssemblyError::Internal(e.to_string()))?
    else {
        return Err(AssemblyError::ResourceNotFound);
    };
    let visibility = auth
        .visibility
        .unwrap_or_else(|| VISIBILITY_PRIVATE.to_string());
    let article_id_uid = article_uid(article_id)?;
    let version_id_uid = version_uid(version_id)?;
    let article_entity = Entity::new(
        article_id_uid.clone(),
        resource_attrs(&auth.owner_id, &visibility, &auth.tag_names)?,
        HashSet::new(),
    )
    .map_err(|e| AssemblyError::Internal(e.to_string()))?;
    let version_entity = Entity::new(
        version_id_uid.clone(),
        resource_attrs(&auth.owner_id, &visibility, &auth.tag_names)?,
        HashSet::from([article_id_uid]),
    )
    .map_err(|e| AssemblyError::Internal(e.to_string()))?;
    Ok((version_id_uid, vec![article_entity, version_entity]))
}

pub async fn assemble(
    db: &DbHandle,
    user_id: &str,
    resource: Resource,
) -> Result<AuthAssembly, AssemblyError> {
    let (principal, mut principal_entities) = assemble_principal(db, user_id).await?;
    let (resource_uid, resource_entities) = assemble_resource(db, resource).await?;
    let mut seen: HashSet<EntityUid> = HashSet::new();
    let mut entities: Vec<Entity> =
        Vec::with_capacity(principal_entities.len() + resource_entities.len());
    for entity in principal_entities.drain(..).chain(resource_entities) {
        if seen.insert(entity.uid().clone()) {
            entities.push(entity);
        }
    }
    Ok(AuthAssembly {
        principal,
        resource: resource_uid,
        entities,
    })
}
