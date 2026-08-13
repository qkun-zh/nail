
use std::str::FromStr;

use agdb::{DbAny, DbError, DbId, QueryBuilder};

use crate::repo::db::{find_by_index_sync, read_node_sync, resolve_node_id_sync};
use crate::repo::types::*;

pub const PERMISSION_ARTICLE_CREATE: &str = "Article::Create";
pub const PERMISSION_ARTICLE_READ: &str = "Article::Read";
pub const PERMISSION_ARTICLE_UPDATE: &str = "Article::Update";
pub const PERMISSION_ARTICLE_DELETE: &str = "Article::Delete";
pub const PERMISSION_VERSION_CREATE: &str = "Version::Create";
pub const PERMISSION_VERSION_READ: &str = "Version::Read";
pub const PERMISSION_VERSION_UPDATE: &str = "Version::Update";
pub const PERMISSION_VERSION_DELETE: &str = "Version::Delete";
pub const PERMISSION_COMMENT_CREATE: &str = "Comment::Create";
pub const PERMISSION_COMMENT_READ: &str = "Comment::Read";
pub const PERMISSION_COMMENT_UPDATE: &str = "Comment::Update";
pub const PERMISSION_COMMENT_DELETE: &str = "Comment::Delete";
pub const PERMISSION_USER_READ: &str = "User::Read";
pub const PERMISSION_USER_UPDATE: &str = "User::Update";
pub const PERMISSION_USER_DELETE: &str = "User::Delete";
pub const PERMISSION_ROLE_MANAGE: &str = "Role::Manage";

pub const ALL_PERMISSIONS: &[&str] = &[
    PERMISSION_ARTICLE_CREATE,
    PERMISSION_ARTICLE_READ,
    PERMISSION_ARTICLE_UPDATE,
    PERMISSION_ARTICLE_DELETE,
    PERMISSION_VERSION_CREATE,
    PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UPDATE,
    PERMISSION_VERSION_DELETE,
    PERMISSION_COMMENT_CREATE,
    PERMISSION_COMMENT_READ,
    PERMISSION_COMMENT_UPDATE,
    PERMISSION_COMMENT_DELETE,
    PERMISSION_USER_READ,
    PERMISSION_USER_UPDATE,
    PERMISSION_USER_DELETE,
    PERMISSION_ROLE_MANAGE,
];

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_RECYCLER: &str = "recycler";
pub const ROLE_MEMBER: &str = "member";

pub const REQUIRED_ROLES: &[&str] = &[ROLE_ADMIN, ROLE_RECYCLER, ROLE_MEMBER];

#[derive(Debug, Clone, Default)]
pub struct RoleAuthorization {
    pub role_name: String,
    pub permissions: Vec<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UserAuthorization {
    pub roles: Vec<RoleAuthorization>,
    pub has_global_role: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ArticleAuthorization {
    pub owner_id: String,
    pub visibility: Option<String>,
    pub tag_names: Vec<String>,
}

pub async fn seed_permissions(db: &crate::repo::DbHandle) -> anyhow::Result<()> {
    let schema = cedar_policy::Schema::from_str(crate::authorization::SCHEMA)
        .map_err(|e| anyhow::anyhow!("invalid authorization schema: {e}"))?;
    let mut db = db.write().await;
    for action_uid in schema.actions() {
        let name = action_uid.id().unescaped().to_string();
        if !find_by_index_sync(&db, KEY_PERMISSION_NAME, &name)?.is_empty() {
            continue;
        }
        db.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias_of(ENTITY_TYPE_PERMISSION, &name)])
                .values(PermissionRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_PERMISSION.to_string(),
                    permission_name: name,
                })
                .query(),
        )?;
    }
    Ok(())
}

pub async fn seed_user_zero_roles(db: &crate::repo::DbHandle, user_id: &str) -> Result<(), DbError> {
    for role_name in REQUIRED_ROLES {
        create_role(db, role_name).await?;
        if *role_name == ROLE_ADMIN {
            for permission in ALL_PERMISSIONS {
                grant_permission_to_role(db, role_name, permission).await?;
            }
        } else if *role_name == ROLE_MEMBER {
            grant_permission_to_role(db, role_name, PERMISSION_ARTICLE_CREATE).await?;
            grant_permission_to_role(db, role_name, PERMISSION_COMMENT_CREATE).await?;
        }
        hold_role(db, user_id, role_name).await?;
    }
    Ok(())
}

pub async fn create_role(db: &crate::repo::DbHandle, name: &str) -> Result<String, DbError> {
    let mut db = db.write().await;
    create_role_sync(&mut db, name)
}

#[allow(dead_code)]
pub(crate) fn create_role_sync(db: &mut DbAny, name: &str) -> Result<String, DbError> {
    if !find_by_index_sync(db, KEY_ROLE_NAME, name)?.is_empty() {
        return Ok(name.to_string());
    }
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_ROLE, name)])
            .values(RoleRow {
                db_id: None,
                entity_type: ENTITY_TYPE_ROLE.to_string(),
                role_name: name.to_string(),
            })
            .query(),
    )?;
    Ok(name.to_string())
}

pub async fn grant_permission_to_role(
    db: &crate::repo::DbHandle,
    role_name: &str,
    permission_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let permission_id = resolve_node_id_sync(&db, ENTITY_TYPE_PERMISSION, permission_name)?
        .ok_or_else(|| {
            DbError::query(
                agdb::DbErrorType::NotFound,
                format!("permission {permission_name}"),
            )
        })?;
    let exists = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_GRANT_PERMISSION)
            .query(),
    )?;
    if !exists.elements.iter().any(|edge| edge.to == permission_id) {
        db.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(role_id)
                .to([permission_id])
                .values([[(KEY_TYPE, EDGE_ROLE_GRANT_PERMISSION).into()]])
                .query(),
        )?;
    }
    Ok(())
}

pub async fn hold_role(
    db: &crate::repo::DbHandle,
    user_id: &str,
    role_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let user_id_db = resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("user {user_id}")))?;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let exists = db.exec(
        QueryBuilder::search()
            .from(user_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    if !exists.elements.iter().any(|edge| edge.to == role_id) {
        db.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(user_id_db)
                .to([role_id])
                .values([[(KEY_TYPE, EDGE_USER_HOLD_ROLE).into()]])
                .query(),
        )?;
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn apply_tag_to_role(
    db: &crate::repo::DbHandle,
    role_name: &str,
    tag_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let tag_id = crate::repo::tag::get_or_create_tag_sync(&mut db, tag_name)?;
    let exists = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_APPLY_TAG)
            .query(),
    )?;
    if !exists.elements.iter().any(|edge| edge.to == tag_id) {
        db.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(role_id)
                .to([tag_id])
                .values([[(KEY_TYPE, EDGE_ROLE_APPLY_TAG).into()]])
                .query(),
        )?;
    }
    Ok(())
}

pub async fn read_user_authorization(
    db: &crate::repo::DbHandle,
    user_id: &str,
) -> Result<UserAuthorization, DbError> {
    let db = db.read().await;
    read_user_authorization_sync(&db, user_id)
}

pub(crate) fn read_user_authorization_sync(
    db: &DbAny,
    user_id: &str,
) -> Result<UserAuthorization, DbError> {
    let Some(user_id_db) = resolve_node_id_sync(db, ENTITY_TYPE_USER, user_id)? else {
        return Ok(UserAuthorization::default());
    };
    let role_edges = db.exec(
        QueryBuilder::search()
            .from(user_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut auth = UserAuthorization::default();
    for edge in &role_edges.elements {
        let role_id = edge.to;
        let Some(role_name) = read_node_sync::<RoleRow>(db, role_id)?.map(|r| r.role_name) else {
            continue;
        };
        let mut role = RoleAuthorization {
            role_name,
            ..Default::default()
        };
        let perm_edges = db.exec(
            QueryBuilder::search()
                .from(role_id)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_ROLE_GRANT_PERMISSION)
                .query(),
        )?;
        for perm_edge in &perm_edges.elements {
            if let Some(name) =
                read_node_sync::<PermissionRow>(db, perm_edge.to)?.map(|r| r.permission_name)
            {
                role.permissions.push(name);
            }
        }
        let tag_edges = db.exec(
            QueryBuilder::search()
                .from(role_id)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_ROLE_APPLY_TAG)
                .query(),
        )?;
        for tag_edge in &tag_edges.elements {
            if let Some(name) = read_node_sync::<TagRow>(db, tag_edge.to)?.map(|r| r.tag_name) {
                role.scopes.push(name);
            }
        }
        if role.scopes.is_empty() {
            auth.has_global_role = true;
        }
        auth.roles.push(role);
    }
    Ok(auth)
}

pub async fn read_article_authorization(
    db: &crate::repo::DbHandle,
    article_id: &str,
) -> Result<Option<ArticleAuthorization>, DbError> {
    let db = db.read().await;
    read_article_authorization_sync(&db, article_id)
}

pub(crate) fn read_article_authorization_sync(
    db: &DbAny,
    article_id: &str,
) -> Result<Option<ArticleAuthorization>, DbError> {
    let Some(article_id_db) = resolve_node_id_sync(db, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    let mut auth = ArticleAuthorization::default();
    let owner_edges = db.exec(
        QueryBuilder::search()
            .to(article_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    if let Some(edge) = owner_edges.elements.first() {
        auth.owner_id = read_node_sync::<IdRow>(db, edge.from)?
            .map(|r| r.id)
            .unwrap_or_default();
    }
    auth.visibility = read_node_sync::<ArticleRow>(db, article_id_db)?.and_then(|r| r.visibility);
    let tag_edges = db.exec(
        QueryBuilder::search()
            .from(article_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    for tag_edge in &tag_edges.elements {
        if let Some(name) = read_node_sync::<TagRow>(db, tag_edge.to)?.map(|r| r.tag_name) {
            auth.tag_names.push(name);
        }
    }
    Ok(Some(auth))
}

pub async fn find_version_id_by_comment(
    db: &crate::repo::DbHandle,
    comment_id: &str,
) -> Result<Option<String>, DbError> {
    let db = db.read().await;
    let Some(comment_id_db) = resolve_node_id_sync(&db, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    let edges = db.exec(
        QueryBuilder::search()
            .from(comment_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_TO_VERSION)
            .query(),
    )?;
    Ok(edges
        .elements
        .first()
        .map(|el| el.to)
        .map(|id| read_node_sync::<IdRow>(&db, id).map(|r| r.map(|row| row.id)))
        .transpose()?
        .flatten())
}

#[allow(dead_code)]
pub(crate) fn business_id_of(db: &DbAny, id: DbId) -> Result<Option<String>, DbError> {
    read_node_sync::<IdRow>(db, id).map(|r| r.map(|row| row.id))
}


pub async fn revoke_permission_from_role(
    db: &crate::repo::DbHandle,
    role_name: &str,
    permission_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let permission_id = resolve_node_id_sync(&db, ENTITY_TYPE_PERMISSION, permission_name)?
        .ok_or_else(|| {
            DbError::query(
                agdb::DbErrorType::NotFound,
                format!("permission {permission_name}"),
            )
        })?;
    let edges = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_GRANT_PERMISSION)
            .query(),
    )?;
    if let Some(edge) = edges.elements.iter().find(|e| e.to == permission_id) {
        db.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

pub async fn unhold_role(
    db: &crate::repo::DbHandle,
    user_id: &str,
    role_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let user_id_db = resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("user {user_id}")))?;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let edges = db.exec(
        QueryBuilder::search()
            .from(user_id_db)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    if let Some(edge) = edges.elements.iter().find(|e| e.to == role_id) {
        db.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

pub async fn remove_tag_from_role(
    db: &crate::repo::DbHandle,
    role_name: &str,
    tag_name: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    let role_id = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("role {role_name}")))?;
    let tag_id = resolve_node_id_sync(&db, ENTITY_TYPE_TAG, tag_name)?
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, format!("tag {tag_name}")))?;
    let edges = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_APPLY_TAG)
            .query(),
    )?;
    if let Some(edge) = edges.elements.iter().find(|e| e.to == tag_id) {
        db.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

pub async fn read_role_authorization(
    db: &crate::repo::DbHandle,
    role_name: &str,
) -> Result<Option<RoleAuthorization>, DbError> {
    let db = db.read().await;
    let Some(role_id) = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(None);
    };
    Ok(Some(read_role_authorization_sync(&db, role_id)?))
}

pub(crate) fn read_role_authorization_sync(
    db: &DbAny,
    role_id: DbId,
) -> Result<RoleAuthorization, DbError> {
    let Some(row) = read_node_sync::<RoleRow>(db, role_id)? else {
        return Err(DbError::query(
            agdb::DbErrorType::NotFound,
            "role row missing",
        ));
    };
    let mut role = RoleAuthorization {
        role_name: row.role_name,
        ..Default::default()
    };
    let perm_edges = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_GRANT_PERMISSION)
            .query(),
    )?;
    for edge in &perm_edges.elements {
        if let Some(name) =
            read_node_sync::<PermissionRow>(db, edge.to)?.map(|r| r.permission_name)
        {
            role.permissions.push(name);
        }
    }
    let tag_edges = db.exec(
        QueryBuilder::search()
            .from(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ROLE_APPLY_TAG)
            .query(),
    )?;
    for edge in &tag_edges.elements {
        if let Some(name) = read_node_sync::<TagRow>(db, edge.to)?.map(|r| r.tag_name) {
            role.scopes.push(name);
        }
    }
    Ok(role)
}

pub async fn list_roles(db: &crate::repo::DbHandle) -> Result<Vec<RoleAuthorization>, DbError> {
    let db = db.read().await;
    let result = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ROLE)
            .query(),
    )?;
    let mut roles = Vec::new();
    for el in &result.elements {
        if let Ok(role) = read_role_authorization_sync(&db, el.id) {
            roles.push(role);
        }
    }
    roles.sort_by(|a, b| a.role_name.cmp(&b.role_name));
    Ok(roles)
}

pub async fn read_role_members(
    db: &crate::repo::DbHandle,
    role_name: &str,
) -> Result<Vec<String>, DbError> {
    let db = db.read().await;
    let Some(role_id) = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(Vec::new());
    };
    let edges = db.exec(
        QueryBuilder::search()
            .to(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut members = Vec::new();
    for edge in &edges.elements {
        if let Some(row) = read_node_sync::<IdRow>(&db, edge.from)? {
            members.push(row.id);
        }
    }
    members.sort();
    Ok(members)
}

pub async fn delete_role(db: &crate::repo::DbHandle, role_name: &str) -> Result<(), DbError> {
    let mut db = db.write().await;
    if let Some(role_id) = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)? {
        db.exec_mut(QueryBuilder::remove().ids([role_id]).query())?;
    }
    Ok(())
}
