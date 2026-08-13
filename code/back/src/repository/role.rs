use agdb::{DbError, DbErrorType, QueryBuilder};

use crate::repository::graph::{DbHandle, find_by_index_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ENTITY_TYPE_PERMISSION, ENTITY_TYPE_ROLE, ENTITY_TYPE_USER, EDGE_ROLE_GRANT_PERMISSION,
    EDGE_USER_HOLD_ROLE, KEY_PERMISSION_NAME, KEY_ROLE_NAME, KEY_TYPE, PermissionRow, RoleRow,
    alias_of,
};

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

pub async fn create_role(db: &DbHandle, name: &str) -> Result<String, DbError> {
    let mut guard = db.write().await;
    if !find_by_index_sync(&guard, KEY_ROLE_NAME, name)?.is_empty() {
        return Ok(name.to_string());
    }
    guard.exec_mut(
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

pub async fn create_permission(db: &DbHandle, name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    if !find_by_index_sync(&guard, KEY_PERMISSION_NAME, name)?.is_empty() {
        return Ok(());
    }
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_PERMISSION, name)])
            .values(PermissionRow {
                db_id: None,
                entity_type: ENTITY_TYPE_PERMISSION.to_string(),
                permission_name: name.to_string(),
            })
            .query(),
    )?;
    Ok(())
}

pub async fn grant_permission_to_role(
    db: &DbHandle,
    role_name: &str,
    permission_name: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let role_id = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let permission_id = resolve_node_id_sync(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_PERMISSION, permission_name))?;
    let edges = guard.exec(
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
    if !edges.elements.iter().any(|edge| edge.to == permission_id) {
        guard.exec_mut(
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

pub async fn hold_role(db: &DbHandle, user_id: &str, role_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let user_db_id = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| not_found(ENTITY_TYPE_USER, user_id))?;
    let role_db_id = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let edges = guard.exec(
        QueryBuilder::search()
            .from(user_db_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    if !edges.elements.iter().any(|edge| edge.to == role_db_id) {
        guard.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(user_db_id)
                .to([role_db_id])
                .values([[(KEY_TYPE, EDGE_USER_HOLD_ROLE).into()]])
                .query(),
        )?;
    }
    Ok(())
}

pub async fn user_holds_role(db: &DbHandle, user_id: &str, role_name: &str) -> Result<bool, DbError> {
    let guard = db.read().await;
    let Some(user_db_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(false);
    };
    let Some(role_db_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(false);
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .from(user_db_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    Ok(edges.elements.iter().any(|edge| edge.to == role_db_id))
}

fn not_found(kind: &str, name: &str) -> DbError {
    DbError::query(DbErrorType::NotFound, format!("{kind} {name}"))
}
