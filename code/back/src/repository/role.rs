use agdb::{DbError, DbErrorType, QueryBuilder};

use crate::repository::graph::{
    DbHandle, find_by_index_sync, read_node_sync, read_rows_sync, resolve_node_id_sync,
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
        PERMISSION_VERSION_DELETE_TRANSFER,
        PERMISSION_VERSION_DELETE_SOFT,
        PERMISSION_VERSION_UNDELETE_SOFT,
        PERMISSION_COMMENT_CREATE,
        PERMISSION_COMMENT_READ,
        PERMISSION_COMMENT_UPDATE,
        PERMISSION_COMMENT_DELETE_HARD,
        PERMISSION_COMMENT_DELETE_TRANSFER,
        PERMISSION_COMMENT_DELETE_SOFT,
        PERMISSION_COMMENT_RESTORE,
        PERMISSION_USER_READ,
        PERMISSION_USER_UPDATE,
        PERMISSION_USER_DELETE_HARD,
        PERMISSION_USER_DELETE_TRANSFER,
        PERMISSION_ROLE_MANAGE,
        PERMISSION_ROLE_REVOKE,
    ]
}

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

#[cfg(test)]
pub async fn user_holds_role(
    db: &DbHandle,
    user_id: &str,
    role_name: &str,
) -> Result<bool, DbError> {
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

pub async fn users_holding_role(db: &DbHandle, role_name: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(role_db_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(Vec::new());
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .to(role_db_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut users = Vec::new();
    for edge in &edges.elements {
        if let Some(row) = read_node_sync::<IdRow>(&guard, edge.from)? {
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
    let Some(user_db_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(false);
    };
    let Some(permission_db_id) =
        resolve_node_id_sync(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
    else {
        return Ok(false);
    };
    let held_roles = guard.exec(
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
    for role_edge in &held_roles.elements {
        let grants = guard.exec(
            QueryBuilder::search()
                .from(role_edge.to)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_ROLE_GRANT_PERMISSION)
                .query(),
        )?;
        if grants
            .elements
            .iter()
            .any(|grant| grant.to == permission_db_id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleView {
    pub role_name: String,
    pub permissions: Vec<String>,
}

pub async fn read_role(db: &DbHandle, role_name: &str) -> Result<Option<RoleView>, DbError> {
    let guard = db.read().await;
    let Some(role_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(None);
    };
    Ok(Some(read_role_view_sync(&guard, role_id)?))
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

pub async fn revoke_permission_from_role(
    db: &DbHandle,
    role_name: &str,
    permission_name: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let role_id = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    let permission_id = resolve_node_id_sync(&guard, ENTITY_TYPE_PERMISSION, permission_name)?
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
    let user_db_id = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)?
        .ok_or_else(|| not_found(ENTITY_TYPE_USER, user_id))?;
    let role_db_id = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, role_name))?;
    remove_outgoing_edge(&mut guard, user_db_id, role_db_id, EDGE_USER_HOLD_ROLE)?;
    Ok(())
}

pub async fn delete_role(db: &DbHandle, role_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    if let Some(role_id) = resolve_node_id_sync(&guard, ENTITY_TYPE_ROLE, role_name)? {
        guard.exec_mut(QueryBuilder::remove().ids([role_id]).query())?;
    }
    Ok(())
}

fn read_role_view_sync(db: &agdb::DbAny, role_id: agdb::DbId) -> Result<RoleView, DbError> {
    let row = read_node_sync::<RoleRow>(db, role_id)?
        .ok_or_else(|| not_found(ENTITY_TYPE_ROLE, "row"))?;
    let mut role = RoleView {
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
    let edges = db.exec(
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
    read_rows_sync::<T>(db, &ids)
}

fn remove_outgoing_edge(
    db: &mut agdb::DbAny,
    from: agdb::DbId,
    to: agdb::DbId,
    edge_type: &str,
) -> Result<(), DbError> {
    let edges = db.exec(
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
    if let Some(edge) = edges.elements.iter().find(|edge| edge.to == to) {
        db.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

fn not_found(kind: &str, name: &str) -> DbError {
    DbError::query(DbErrorType::NotFound, format!("{kind} {name}"))
}
