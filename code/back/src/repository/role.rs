use agdb::{DbError, DbErrorType, QueryBuilder};

use crate::repository::graph::{
    DbHandle, find_by_index, incoming_edges, insert_edge, outgoing_edges, read_node, read_rows,
    resolve_node_id,
};
use crate::repository::schema::{
    EDGE_ROLE_GRANT_PERMISSION, EDGE_USER_HOLD_ROLE, ENTITY_TYPE_PERMISSION, ENTITY_TYPE_ROLE,
    ENTITY_TYPE_USER, IdRow, KEY_PERMISSION_NAME, KEY_ROLE_NAME, KEY_TYPE, PermissionRow, RoleRow,
    alias_of,
};

include!(concat!(env!("OUT_DIR"), "/permissions.rs"));

#[cfg(test)]
pub fn permission_vocabulary() -> &'static [&'static str] {
    &[
        PERMISSION_ARTICLE_CREATE,
        PERMISSION_ARTICLE_READ,
        PERMISSION_ARTICLE_UPDATE,
        PERMISSION_ARTICLE_DELETE_HARD,
        PERMISSION_ARTICLE_DELETE_TRANSFER,
        PERMISSION_ARTICLE_DELETE_SOFT,
        PERMISSION_ARTICLE_UNDELETE_SOFT,
        PERMISSION_VERSION_CREATE,
        PERMISSION_VERSION_READ,
        PERMISSION_VERSION_UPDATE,
        PERMISSION_VERSION_DELETE_HARD,
        PERMISSION_VERSION_DELETE_SOFT,
        PERMISSION_VERSION_UNDELETE_SOFT,
        PERMISSION_COMMENT_CREATE,
        PERMISSION_COMMENT_READ,
        PERMISSION_COMMENT_UPDATE,
        PERMISSION_COMMENT_DELETE_HARD,
        PERMISSION_COMMENT_DELETE_TRANSFER,
        PERMISSION_COMMENT_DELETE_SOFT,
        PERMISSION_COMMENT_UNDELETE_SOFT,
        PERMISSION_USER_CREATE,
        PERMISSION_USER_READ,
        PERMISSION_USER_UPDATE,
        PERMISSION_USER_DELETE_HARD,
        PERMISSION_USER_DELETE_TRANSFER,
        PERMISSION_USER_DELETE_SOFT,
        PERMISSION_USER_UNDELETE_SOFT,
        PERMISSION_ROLE_CREATE,
        PERMISSION_ROLE_READ,
        PERMISSION_ROLE_UPDATE,
        PERMISSION_ROLE_DELETE,
        PERMISSION_ROLE_GRANT,
        PERMISSION_ROLE_REVOKE,
        PERMISSION_TAG_CREATE,
        PERMISSION_TAG_READ,
        PERMISSION_TAG_UPDATE,
        PERMISSION_TAG_DELETE,
        PERMISSION_TAG_APPLY,
        PERMISSION_TAG_UNAPPLY,
    ]
}

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_RECYCLER: &str = "recycler";
pub const ROLE_MEMBER: &str = "member";

pub const REQUIRED_ROLES: &[&str] = &[ROLE_ADMIN, ROLE_RECYCLER, ROLE_MEMBER];

pub async fn create_role(db: &DbHandle, name: &str) -> Result<String, DbError> {
    let mut guard = db.write().await;
    if let Some(existing) = read_role_by_name_sync(&guard, name)? {
        return Ok(existing.id);
    }
    let role_id = uuid::Uuid::now_v7().to_string();
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_ROLE, &role_id)])
            .values(RoleRow {
                db_id: None,
                entity_type: ENTITY_TYPE_ROLE.to_string(),
                id: role_id.clone(),
                role_name: name.to_string(),
            })
            .query(),
    )?;
    Ok(role_id)
}

pub async fn create_permission(db: &DbHandle, name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    if !find_by_index(&guard, KEY_PERMISSION_NAME, name)?.is_empty() {
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
    let role_id = resolve_role_id_by_name_sync(&guard, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let permission_id = resolve_node_id(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_PERMISSION, permission_name))?;
    let edges = outgoing_edges(&guard, role_id, EDGE_ROLE_GRANT_PERMISSION)?;
    if !edges.iter().any(|edge| edge.to == permission_id) {
        insert_edge(
            &mut guard,
            EDGE_ROLE_GRANT_PERMISSION,
            role_id.into(),
            permission_id.into(),
        )?;
    }
    Ok(())
}

pub async fn hold_role(db: &DbHandle, user_id: &str, role_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let user_db_id = resolve_node_id(&guard, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| not_found(ENTITY_TYPE_USER, user_id))?;
    let role_db_id = resolve_role_id_by_name_sync(&guard, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let edges = outgoing_edges(&guard, user_db_id, EDGE_USER_HOLD_ROLE)?;
    if !edges.iter().any(|edge| edge.to == role_db_id) {
        insert_edge(
            &mut guard,
            EDGE_USER_HOLD_ROLE,
            user_db_id.into(),
            role_db_id.into(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
pub async fn user_holds_role(
    db: &DbHandle,
    user_id: &str,
    role_name: &str,
) -> Result<bool, DbError> {
    let guard = db.read().await;
    let Some(user_db_id) = resolve_node_id(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(false);
    };
    let Some(role_db_id) = resolve_role_id_by_name_sync(&guard, role_name)? else {
        return Ok(false);
    };
    let edges = outgoing_edges(&guard, user_db_id, EDGE_USER_HOLD_ROLE)?;
    Ok(edges.iter().any(|edge| edge.to == role_db_id))
}

pub async fn users_holding_role(db: &DbHandle, role_name: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(role_db_id) = resolve_role_id_by_name_sync(&guard, role_name)? else {
        return Ok(Vec::new());
    };
    let edges = incoming_edges(&guard, role_db_id, EDGE_USER_HOLD_ROLE)?;
    let mut users = Vec::new();
    for edge in &edges {
        if let Some(row) = read_node::<IdRow>(&guard, edge.from)? {
            users.push(row.id);
        }
    }
    Ok(users)
}

#[cfg(test)]
pub async fn user_holds_permission(
    db: &DbHandle,
    user_id: &str,
    permission_name: &str,
) -> Result<bool, DbError> {
    let guard = db.read().await;
    let Some(user_db_id) = resolve_node_id(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(false);
    };
    let Some(permission_db_id) = resolve_node_id(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
    else {
        return Ok(false);
    };
    let held_roles = outgoing_edges(&guard, user_db_id, EDGE_USER_HOLD_ROLE)?;
    for role_edge in &held_roles {
        let grants = outgoing_edges(&guard, role_edge.to, EDGE_ROLE_GRANT_PERMISSION)?;
        if grants.iter().any(|grant| grant.to == permission_db_id) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleView {
    pub id: String,
    pub role_name: String,
    pub permissions: Vec<String>,
}

pub async fn read_role_by_id(db: &DbHandle, role_id: &str) -> Result<Option<RoleView>, DbError> {
    let guard = db.read().await;
    let Some(role_db_id) = resolve_node_id(&guard, ENTITY_TYPE_ROLE, role_id)? else {
        return Ok(None);
    };
    Ok(Some(read_role_view_sync(&guard, role_db_id)?))
}

pub async fn read_role(db: &DbHandle, role_name: &str) -> Result<Option<RoleView>, DbError> {
    let guard = db.read().await;
    read_role_by_name_sync(&guard, role_name)
}

fn read_role_by_name_sync(
    guard: &agdb::DbAny,
    role_name: &str,
) -> Result<Option<RoleView>, DbError> {
    let Some(role_id) = resolve_role_id_by_name_sync(guard, role_name)? else {
        return Ok(None);
    };
    Ok(Some(read_role_view_sync(guard, role_id)?))
}

fn resolve_role_id_by_name_sync(
    guard: &agdb::DbAny,
    role_name: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    Ok(find_by_index(guard, KEY_ROLE_NAME, role_name)?
        .first()
        .copied())
}

pub async fn read_roles(db: &DbHandle) -> Result<Vec<RoleView>, DbError> {
    let guard = db.read().await;
    let result = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ROLE)
            .query(),
    )?;
    let mut roles = Vec::new();
    for element in &result.elements {
        roles.push(read_role_view_sync(&guard, element.id)?);
    }
    roles.sort_by(|left, right| left.role_name.cmp(&right.role_name));
    Ok(roles)
}

pub async fn read_role_members(db: &DbHandle, role_name: &str) -> Result<Vec<String>, DbError> {
    let mut members = users_holding_role(db, role_name).await?;
    members.sort();
    Ok(members)
}

pub async fn roles_of_user(db: &DbHandle, user_id: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(user_db_id) = resolve_node_id(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(Vec::new());
    };
    let edges = outgoing_edges(&guard, user_db_id, EDGE_USER_HOLD_ROLE)?;
    let mut roles = Vec::new();
    for edge in &edges {
        if let Some(row) = read_node::<RoleRow>(&guard, edge.to)? {
            roles.push(row.role_name);
        }
    }
    roles.sort();
    Ok(roles)
}

pub async fn revoke_permission_from_role(
    db: &DbHandle,
    role_name: &str,
    permission_name: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let role_id = resolve_role_id_by_name_sync(&guard, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let permission_id = resolve_node_id(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_PERMISSION, permission_name))?;
    remove_outgoing_edge(
        &mut guard,
        role_id,
        permission_id,
        EDGE_ROLE_GRANT_PERMISSION,
    )?;
    Ok(())
}

pub async fn unhold_role(db: &DbHandle, user_id: &str, role_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let user_db_id = resolve_node_id(&guard, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| not_found(ENTITY_TYPE_USER, user_id))?;
    let role_db_id = resolve_role_id_by_name_sync(&guard, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    remove_outgoing_edge(&mut guard, user_db_id, role_db_id, EDGE_USER_HOLD_ROLE)?;
    Ok(())
}

pub async fn delete_role(db: &DbHandle, role_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    if let Some(role_id) = resolve_role_id_by_name_sync(&guard, role_name)? {
        guard.exec_mut(QueryBuilder::remove().ids([role_id]).query())?;
    }
    Ok(())
}

fn read_role_view_sync(db: &agdb::DbAny, role_id: agdb::DbId) -> Result<RoleView, DbError> {
    let row =
        read_node::<RoleRow>(db, role_id)?.ok_or_else(|| not_found(ENTITY_TYPE_ROLE, "row"))?;
    let mut role = RoleView {
        id: row.id,
        role_name: row.role_name,
        ..Default::default()
    };
    for permission in read_edge_rows::<PermissionRow>(db, role_id, EDGE_ROLE_GRANT_PERMISSION)? {
        role.permissions.push(permission.permission_name);
    }
    Ok(role)
}

fn read_edge_rows<T>(db: &agdb::DbAny, from: agdb::DbId, edge_type: &str) -> Result<Vec<T>, DbError>
where
    T: agdb::DbType<ValueType = T> + agdb::DbTypeMarker,
{
    let edges = outgoing_edges(db, from, edge_type)?;
    let ids: Vec<agdb::DbId> = edges.iter().map(|edge| edge.to).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    read_rows::<T>(db, &ids)
}

fn remove_outgoing_edge(
    db: &mut agdb::DbAny,
    from: agdb::DbId,
    to: agdb::DbId,
    edge_type: &str,
) -> Result<(), DbError> {
    let edges = outgoing_edges(db, from, edge_type)?;
    if let Some(edge) = edges.iter().find(|edge| edge.to == to) {
        db.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

fn not_found(kind: &str, name: &str) -> DbError {
    DbError::query(DbErrorType::NotFound, format!("{kind} {name}"))
}
