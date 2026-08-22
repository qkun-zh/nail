use database::{Database, EdgeKind, Error, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::schema::{
    IdRow, KEY_PERMISSION_NAME, KEY_ROLE_NAME, PermissionRow, RoleRow,
};

pub use authorizer::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_DELETE_HARD, PERMISSION_ARTICLE_DELETE_SOFT,
    PERMISSION_ARTICLE_DELETE_TRANSFER, PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UNDELETE_SOFT,
    PERMISSION_ARTICLE_UPDATE, PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_DELETE_HARD,
    PERMISSION_COMMENT_DELETE_SOFT, PERMISSION_COMMENT_DELETE_TRANSFER, PERMISSION_COMMENT_READ,
    PERMISSION_COMMENT_UNDELETE_SOFT, PERMISSION_COMMENT_UPDATE, PERMISSION_ROLE_CREATE,
    PERMISSION_ROLE_DELETE, PERMISSION_ROLE_GRANT, PERMISSION_ROLE_READ, PERMISSION_ROLE_REVOKE,
    PERMISSION_ROLE_UPDATE, PERMISSION_TAG_APPLY, PERMISSION_TAG_CREATE, PERMISSION_TAG_DELETE,
    PERMISSION_TAG_READ, PERMISSION_TAG_UNAPPLY, PERMISSION_TAG_UPDATE, PERMISSION_USER_CREATE,
    PERMISSION_USER_DELETE_HARD, PERMISSION_USER_DELETE_SOFT, PERMISSION_USER_DELETE_TRANSFER,
    PERMISSION_USER_READ, PERMISSION_USER_UNDELETE_SOFT, PERMISSION_USER_UPDATE,
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE_HARD, PERMISSION_VERSION_DELETE_SOFT,
    PERMISSION_VERSION_READ, PERMISSION_VERSION_UNDELETE_SOFT, PERMISSION_VERSION_UPDATE,
};

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

pub fn create_role(db: &Database, name: &str) -> Result<String, Error> {
    db.write(|scope| {
        if let Some(existing) = read_role_by_name_sync(scope, name)? {
            return Ok(existing.id);
        }
        let role_id = uuid::Uuid::now_v7().to_string();
        scope.insert_node(&RoleRow {
            id: role_id.clone(),
            role_name: name.to_string(),
        })?;
        Ok(role_id)
    })
}

pub fn create_permission(db: &Database, name: &str) -> Result<(), Error> {
    db.write(|scope| {
        if scope.find_by_key(KEY_PERMISSION_NAME, name)?.is_some() {
            return Ok(());
        }
        scope.insert_node(&PermissionRow {
            permission_name: name.to_string(),
        })?;
        Ok(())
    })
}

pub fn grant_permission_to_role(
    db: &Database,
    role_name: &str,
    permission_name: &str,
) -> Result<(), Error> {
    db.write(|scope| {
        let role_id = resolve_role_id_by_name_sync(scope, role_name)?
            .ok_or_else(|| not_found(NodeKind::Role, role_name))?;
        let permission_id = scope
            .resolve(NodeKind::Permission, permission_name)?
            .ok_or_else(|| not_found(NodeKind::Permission, permission_name))?;
        scope.insert_edge(
            NodeKind::Role,
            role_id,
            EdgeKind::RoleGrantPermission,
            NodeKind::Permission,
            permission_id,
        )?;
        Ok(())
    })
}

pub fn hold_role(db: &Database, user_id: &str, role_name: &str) -> Result<(), Error> {
    db.write(|scope| {
        let user_db_id = scope
            .resolve(NodeKind::User, user_id)?
            .ok_or_else(|| not_found(NodeKind::User, user_id))?;
        let role_db_id = resolve_role_id_by_name_sync(scope, role_name)?
            .ok_or_else(|| not_found(NodeKind::Role, role_name))?;
        scope.insert_edge(
            NodeKind::User,
            user_db_id,
            EdgeKind::UserHoldRole,
            NodeKind::Role,
            role_db_id,
        )?;
        Ok(())
    })
}

#[cfg(test)]
pub fn user_holds_role(db: &Database, user_id: &str, role_name: &str) -> Result<bool, Error> {
    db.read(|scope| {
        let Some(user_db_id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(false);
        };
        let Some(role_db_id) = resolve_role_id_by_name_sync(scope, role_name)? else {
            return Ok(false);
        };
        Ok(scope
            .outgoing(user_db_id, EdgeKind::UserHoldRole)?
            .contains(&role_db_id))
    })
}

pub fn users_holding_role(db: &Database, role_name: &str) -> Result<Vec<String>, Error> {
    db.read(|scope| {
        let Some(role_db_id) = resolve_role_id_by_name_sync(scope, role_name)? else {
            return Ok(Vec::new());
        };
        let holders = scope.incoming(role_db_id, EdgeKind::UserHoldRole)?;
        let rows = scope.scope_read_nodes::<IdRow>(&holders)?;
        Ok(rows.into_iter().map(|row| row.id).collect())
    })
}

#[cfg(test)]
pub fn user_holds_permission(
    db: &Database,
    user_id: &str,
    permission_name: &str,
) -> Result<bool, Error> {
    db.read(|scope| {
        let Some(user_db_id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(false);
        };
        let Some(permission_db_id) = scope.resolve(NodeKind::Permission, permission_name)? else {
            return Ok(false);
        };
        for role in scope.outgoing(user_db_id, EdgeKind::UserHoldRole)? {
            if scope
                .outgoing(role, EdgeKind::RoleGrantPermission)?
                .contains(&permission_db_id)
            {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleView {
    pub id: String,
    pub role_name: String,
    pub permissions: Vec<String>,
}

pub fn read_role_by_id(db: &Database, role_id: &str) -> Result<Option<RoleView>, Error> {
    db.read(|scope| {
        let Some(role_db_id) = scope.resolve(NodeKind::Role, role_id)? else {
            return Ok(None);
        };
        Ok(Some(read_role_view_sync(scope, role_db_id)?))
    })
}

pub fn read_role(db: &Database, role_name: &str) -> Result<Option<RoleView>, Error> {
    db.read(|scope| read_role_by_name_sync(scope, role_name))
}

fn read_role_by_name_sync(
    scope: &impl GraphRead,
    role_name: &str,
) -> Result<Option<RoleView>, Error> {
    let Some(role_id) = resolve_role_id_by_name_sync(scope, role_name)? else {
        return Ok(None);
    };
    Ok(Some(read_role_view_sync(scope, role_id)?))
}

fn resolve_role_id_by_name_sync(
    scope: &impl GraphRead,
    role_name: &str,
) -> Result<Option<database::NodeId>, Error> {
    scope.scope_find_by_key(KEY_ROLE_NAME, role_name)
}

pub fn read_roles(db: &Database) -> Result<Vec<RoleView>, Error> {
    db.read(|scope| {
        let nodes = scope.all_nodes(NodeKind::Role)?;
        let mut roles = Vec::with_capacity(nodes.len());
        for node in nodes {
            roles.push(read_role_view_sync(scope, node)?);
        }
        roles.sort_by(|left, right| left.role_name.cmp(&right.role_name));
        Ok(roles)
    })
}

pub fn read_role_members(db: &Database, role_name: &str) -> Result<Vec<String>, Error> {
    let mut members = users_holding_role(db, role_name)?;
    members.sort();
    Ok(members)
}

pub fn roles_of_user(db: &Database, user_id: &str) -> Result<Vec<RoleRow>, Error> {
    db.read(|scope| {
        let Some(user_db_id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Vec::new());
        };
        let held = scope.outgoing(user_db_id, EdgeKind::UserHoldRole)?;
        let mut rows = scope.scope_read_nodes::<RoleRow>(&held)?;
        rows.sort_by(|a, b| a.role_name.cmp(&b.role_name));
        Ok(rows)
    })
}

pub fn revoke_permission_from_role(
    db: &Database,
    role_name: &str,
    permission_name: &str,
) -> Result<(), Error> {
    db.write(|scope| {
        let role_id = resolve_role_id_by_name_sync(scope, role_name)?
            .ok_or_else(|| not_found(NodeKind::Role, role_name))?;
        let permission_id = scope
            .resolve(NodeKind::Permission, permission_name)?
            .ok_or_else(|| not_found(NodeKind::Permission, permission_name))?;
        scope.remove_edge(role_id, EdgeKind::RoleGrantPermission, permission_id)?;
        Ok(())
    })
}

pub fn unhold_role(db: &Database, user_id: &str, role_name: &str) -> Result<(), Error> {
    db.write(|scope| {
        let user_db_id = scope
            .resolve(NodeKind::User, user_id)?
            .ok_or_else(|| not_found(NodeKind::User, user_id))?;
        let role_db_id = resolve_role_id_by_name_sync(scope, role_name)?
            .ok_or_else(|| not_found(NodeKind::Role, role_name))?;
        scope.remove_edge(user_db_id, EdgeKind::UserHoldRole, role_db_id)?;
        Ok(())
    })
}

pub fn delete_role(db: &Database, role_name: &str) -> Result<(), Error> {
    db.write(|scope| {
        if let Some(role_id) = resolve_role_id_by_name_sync(scope, role_name)? {
            scope.remove(&[role_id])?;
        }
        Ok(())
    })
}

fn read_role_view_sync(
    scope: &impl GraphRead,
    role_id: database::NodeId,
) -> Result<RoleView, Error> {
    let row = scope
        .scope_read_node::<RoleRow>(role_id)?
        .ok_or_else(|| not_found(NodeKind::Role, "row"))?;
    let mut role = RoleView {
        id: row.id,
        role_name: row.role_name,
        ..Default::default()
    };
    let grants = scope.scope_outgoing(role_id, EdgeKind::RoleGrantPermission)?;
    let permissions = scope.scope_read_nodes::<PermissionRow>(&grants)?;
    role.permissions
        .extend(permissions.into_iter().map(|p| p.permission_name));
    Ok(role)
}

fn not_found(kind: NodeKind, name: &str) -> Error {
    Error::NotFound {
        kind,
        id: name.to_string(),
    }
}
