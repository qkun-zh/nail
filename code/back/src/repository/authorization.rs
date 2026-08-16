use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use agdb::{DbError, QueryBuilder};
use cedar_policy::{Entity, EntityUid, RestrictedExpression};

use crate::repository::comment::{owner_of_comment, version_of_comment};
use crate::repository::graph::{DbHandle, read_node_sync, read_rows_sync, resolve_node_id_sync};
use crate::repository::role::RoleView;
use crate::repository::schema::{
    EDGE_ARTICLE_APPLY_TAG, EDGE_ROLE_APPLY_TAG, EDGE_ROLE_GRANT_PERMISSION,
    EDGE_USER_AUTHOR_ARTICLE, EDGE_USER_HOLD_ROLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, IdRow,
    KEY_TYPE, PermissionRow, RoleRow, TagRow,
};
use crate::repository::version::parent_article_of;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Article(String),
    Version(String),
    Comment(String),
    System(String),
}

#[derive(Debug, Clone, Default)]
pub struct UserAuthorization {
    pub roles: Vec<RoleView>,
    pub has_global_role: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ArticleAuthorization {
    pub owner_id: String,
    pub tag_names: Vec<String>,
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

pub async fn read_user_authorization(
    db: &DbHandle,
    user_id: &str,
) -> Result<UserAuthorization, DbError> {
    let guard = db.read().await;
    let Some(user) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(UserAuthorization::default());
    };
    let role_edges = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut authorization = UserAuthorization::default();
    for edge in &role_edges.elements {
        let Some(role_name) = read_node_sync::<RoleRow>(&guard, edge.to)?.map(|row| row.role_name)
        else {
            continue;
        };
        let mut role = RoleView {
            role_name,
            ..Default::default()
        };
        for name in read_edges::<PermissionRow>(&guard, edge.to, EDGE_ROLE_GRANT_PERMISSION)?
            .into_iter()
            .map(|row| row.permission_name)
        {
            role.permissions.push(name);
        }
        for name in read_edges::<TagRow>(&guard, edge.to, EDGE_ROLE_APPLY_TAG)?
            .into_iter()
            .map(|row| row.tag_name)
        {
            role.scopes.push(name);
        }
        if role.scopes.is_empty() {
            authorization.has_global_role = true;
        }
        authorization.roles.push(role);
    }
    Ok(authorization)
}

pub async fn read_article_authorization(
    db: &DbHandle,
    article_id: &str,
) -> Result<Option<ArticleAuthorization>, DbError> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    let mut authorization = ArticleAuthorization::default();
    let owner_edges = guard.exec(
        QueryBuilder::search()
            .to(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_AUTHOR_ARTICLE)
            .query(),
    )?;
    if let Some(edge) = owner_edges.elements.first() {
        authorization.owner_id = read_node_sync::<IdRow>(&guard, edge.from)?
            .map(|row| row.id)
            .unwrap_or_default();
    }
    for name in read_edges::<TagRow>(&guard, article, EDGE_ARTICLE_APPLY_TAG)?
        .into_iter()
        .map(|row| row.tag_name)
    {
        authorization.tag_names.push(name);
    }
    Ok(Some(authorization))
}

pub async fn assemble_principal(
    db: &DbHandle,
    user_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let authorization = read_user_authorization(db, user_id)
        .await
        .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    let principal = user_uid(user_id)?;
    let mut entities: Vec<Entity> = Vec::new();
    let mut seen: HashSet<EntityUid> = HashSet::new();
    let mut role_uids: HashSet<EntityUid> = HashSet::new();
    let mut scopes: Vec<String> = Vec::new();
    let mut global_role = false;

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
        if role.scopes.is_empty() {
            global_role = true;
        }
        scopes.extend(role.scopes.iter().cloned());
    }

    let global_role_expr = expression(if global_role { "true" } else { "false" })?;
    let scopes_expr = set_expression(&scopes, tag_uid)?;
    let user_entity = Entity::new(
        principal.clone(),
        HashMap::from([
            ("global_role".to_string(), global_role_expr),
            ("scopes".to_string(), scopes_expr),
        ]),
        role_uids,
    )
    .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    entities.push(user_entity);
    Ok((principal, entities))
}

pub async fn assemble_resource(
    db: &DbHandle,
    resource: Resource,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    match resource {
        Resource::Article(article_id) => {
            let authorization = read_article_authorization(db, &article_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let resource_uid = article_uid(&article_id)?;
            let entity = Entity::new(
                resource_uid.clone(),
                resource_attrs(&authorization.owner_id, &authorization.tag_names)?,
                HashSet::new(),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            Ok((resource_uid, vec![entity]))
        }
        Resource::Version(version_id) => {
            let article_id = parent_article_of(db, &version_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            assemble_version_chain(db, &article_id, &version_id).await
        }
        Resource::Comment(comment_id) => {
            let version_id = version_of_comment(db, &comment_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let article_id = parent_article_of(db, &version_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let authorization = read_article_authorization(db, &article_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .ok_or(AssemblyError::ResourceNotFound)?;
            let comment_owner = owner_of_comment(db, &comment_id)
                .await
                .map_err(|error| AssemblyError::Internal(error.to_string()))?
                .unwrap_or_default();

            let article_entity_uid = article_uid(&article_id)?;
            let version_entity_uid = version_uid(&version_id)?;
            let comment_entity_uid = comment_uid(&comment_id)?;
            let article_entity = Entity::new(
                article_entity_uid.clone(),
                resource_attrs(&authorization.owner_id, &authorization.tag_names)?,
                HashSet::new(),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            let version_entity = Entity::new(
                version_entity_uid.clone(),
                resource_attrs(&authorization.owner_id, &authorization.tag_names)?,
                HashSet::from([article_entity_uid]),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            let comment_entity = Entity::new(
                comment_entity_uid.clone(),
                resource_attrs(&comment_owner, &authorization.tag_names)?,
                HashSet::from([version_entity_uid]),
            )
            .map_err(|error| AssemblyError::Internal(error.to_string()))?;
            Ok((
                comment_entity_uid,
                vec![article_entity, version_entity, comment_entity],
            ))
        }
        Resource::System(name) => {
            let resource_uid = system_uid(&name)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
    }
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

async fn assemble_version_chain(
    db: &DbHandle,
    article_id: &str,
    version_id: &str,
) -> Result<(EntityUid, Vec<Entity>), AssemblyError> {
    let authorization = read_article_authorization(db, article_id)
        .await
        .map_err(|error| AssemblyError::Internal(error.to_string()))?
        .ok_or(AssemblyError::ResourceNotFound)?;
    let article_entity_uid = article_uid(article_id)?;
    let version_entity_uid = version_uid(version_id)?;
    let article_entity = Entity::new(
        article_entity_uid.clone(),
        resource_attrs(&authorization.owner_id, &authorization.tag_names)?,
        HashSet::new(),
    )
    .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    let version_entity = Entity::new(
        version_entity_uid.clone(),
        resource_attrs(&authorization.owner_id, &authorization.tag_names)?,
        HashSet::from([article_entity_uid]),
    )
    .map_err(|error| AssemblyError::Internal(error.to_string()))?;
    Ok((version_entity_uid, vec![article_entity, version_entity]))
}

fn read_edges<T>(guard: &agdb::DbAny, from: agdb::DbId, edge_type: &str) -> Result<Vec<T>, DbError>
where
    T: agdb::DbType<ValueType = T> + agdb::DbTypeMarker,
{
    let edges = guard.exec(
        QueryBuilder::search()
            .from(from)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    let ids: Vec<agdb::DbId> = edges.elements.iter().map(|edge| edge.to).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    read_rows_sync::<T>(guard, &ids)
}

fn parse_uid(text: &str) -> Result<EntityUid, AssemblyError> {
    text.parse::<EntityUid>()
        .map_err(|error| AssemblyError::Internal(format!("invalid entity uid {text:?}: {error}")))
}

fn user_uid(user_id: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("User::\"{user_id}\""))
}

fn role_uid(role_name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Role::\"{role_name}\""))
}

fn tag_uid(tag_name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Tag::\"{tag_name}\""))
}

fn action_uid(action: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("Action::\"{action}\""))
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

fn system_uid(name: &str) -> Result<EntityUid, AssemblyError> {
    parse_uid(&format!("System::\"{name}\""))
}

fn expression(text: &str) -> Result<RestrictedExpression, AssemblyError> {
    RestrictedExpression::from_str(text)
        .map_err(|error| AssemblyError::Internal(format!("invalid expression {text:?}: {error}")))
}

fn set_expression(
    values: &[String],
    uid_builder: impl Fn(&str) -> Result<EntityUid, AssemblyError>,
) -> Result<RestrictedExpression, AssemblyError> {
    if values.is_empty() {
        return expression("[]");
    }
    let joined = values
        .iter()
        .map(|value| uid_builder(value).map(|uid| uid.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    expression(&format!("[{}]", joined.join(", ")))
}

fn resource_attrs(
    owner_id: &str,
    tag_names: &[String],
) -> Result<HashMap<String, RestrictedExpression>, AssemblyError> {
    let owner = if owner_id.is_empty() {
        expression("User::\"\"")?
    } else {
        expression(&user_uid(owner_id)?.to_string())?
    };
    Ok(HashMap::from([
        ("owner".to_string(), owner),
        (
            "required_scopes".to_string(),
            set_expression(tag_names, tag_uid)?,
        ),
    ]))
}
