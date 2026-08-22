use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity, authorize_entity_or, authorize_global};
use crate::logic::error::LogicError;
use crate::logic::pagination::paginate;
use crate::repository::role::{
    PERMISSION_ROLE_CREATE, PERMISSION_ROLE_DELETE, PERMISSION_ROLE_GRANT, PERMISSION_ROLE_READ,
    PERMISSION_ROLE_REVOKE, PERMISSION_ROLE_UPDATE, REQUIRED_ROLES, ROLE_ADMIN,
    create_role as create_role_node, delete_role as delete_role_node, grant_permission_to_role,
    hold_role, read_role as read_role_node, read_role_by_id as read_role_node_by_id,
    read_role_members, read_roles as read_role_nodes, revoke_permission_from_role, unhold_role,
};
use common::response::NamedRef;
use common::response::role::{RoleListItem, RoleView};

pub struct RoleUpdate<'a> {
    pub permissions_add: &'a [String],
    pub permissions_remove: &'a [String],
    pub users_add: &'a [String],
    pub users_remove: &'a [String],
}

pub fn validate_role_name(raw: &str) -> Result<String, LogicError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(LogicError::bad_request("invalid role name"));
    }
    if !trimmed
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(LogicError::bad_request("invalid role name"));
    }
    Ok(trimmed.to_string())
}

pub fn create_role(
    state: &AppState,
    actor_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize_global(state, actor_id, PERMISSION_ROLE_CREATE)?;
    let name = validate_role_name(raw_name)?;
    if read_role_node(&state.database, &name)?.is_some() {
        return Err(LogicError::bad_request("role already exists"));
    }
    let role_id = create_role_node(&state.database, &name)
        .map_err(|error| LogicError::internal(format!("failed to create role: {error}")))?;
    Ok((role_id, name))
}

pub fn read_roles(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<common::response::ListPage<RoleListItem>, LogicError> {
    authorize_global(state, actor_id, PERMISSION_ROLE_READ)?;
    let roles = read_role_nodes(&state.database)?;
    let total = roles.len() as u64;
    let (page_roles, has_next) = paginate(roles, page, limit);

    let mut items = Vec::with_capacity(page_roles.len());
    for role in &page_roles {
        let member_count = read_role_members(&state.database, &role.role_name)?.len() as u64;
        items.push(RoleListItem {
            id: role.id.clone(),
            name: role.role_name.clone(),
            permissions: role.permissions.clone(),
            member_count,
        });
    }
    Ok(common::response::ListPage {
        items,
        has_next,
        total,
    })
}

pub fn read_role(state: &AppState, actor_id: &str, role_id: &str) -> Result<RoleView, LogicError> {
    let role = read_role_node_by_id(&state.database, role_id)?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ROLE_READ,
        EntityRef::Role(role.role_name.as_str()),
    )?;
    let members = read_role_members(&state.database, &role.role_name)?;
    Ok(RoleView {
        id: role.id,
        name: role.role_name,
        permissions: role.permissions,
        members,
    })
}

pub fn update_role(
    state: &AppState,
    actor_id: &str,
    role_id: &str,
    update: &RoleUpdate<'_>,
) -> Result<RoleView, LogicError> {
    let &RoleUpdate {
        permissions_add,
        permissions_remove,
        users_add,
        users_remove,
    } = update;
    let role = read_role_node_by_id(&state.database, role_id)?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    let name = role.role_name;
    let has_adds = !permissions_add.is_empty() || !users_add.is_empty();
    let has_removes = !permissions_remove.is_empty() || !users_remove.is_empty();
    if has_adds || has_removes {
        authorize_entity(
            state,
            actor_id,
            PERMISSION_ROLE_UPDATE,
            EntityRef::Role(&name),
        )?;
    }
    if has_adds {
        authorize_entity(
            state,
            actor_id,
            PERMISSION_ROLE_GRANT,
            EntityRef::Role(&name),
        )?;
    }
    if has_removes {
        authorize_entity(
            state,
            actor_id,
            PERMISSION_ROLE_REVOKE,
            EntityRef::Role(&name),
        )?;
    }
    if REQUIRED_ROLES.contains(&name.as_str()) && name != ROLE_ADMIN {
        let destructive = !permissions_remove.is_empty() || !users_remove.is_empty();
        if destructive {
            return Err(LogicError::bad_request(format!(
                "role {name} is a required role and cannot be modified destructively"
            )));
        }
    }
    for permission in permissions_add {
        grant_permission_to_role(&state.database, &name, permission).map_err(|error| {
            LogicError::internal(format!("failed to grant {permission}: {error}"))
        })?;
    }
    for permission in permissions_remove {
        revoke_permission_from_role(&state.database, &name, permission).map_err(|error| {
            LogicError::internal(format!("failed to revoke {permission}: {error}"))
        })?;
    }
    for user in users_add {
        hold_role(&state.database, user, &name).map_err(|error| {
            LogicError::internal(format!("failed to hold role for {user}: {error}"))
        })?;
    }
    for user in users_remove {
        unhold_role(&state.database, user, &name).map_err(|error| {
            LogicError::internal(format!("failed to unhold role for {user}: {error}"))
        })?;
    }
    let role = read_role_node_by_id(&state.database, role_id)?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    let members = read_role_members(&state.database, &role.role_name)?;
    Ok(RoleView {
        id: role.id,
        name: role.role_name,
        permissions: role.permissions,
        members,
    })
}

pub fn delete_role(
    state: &AppState,
    actor_id: &str,
    role_id: &str,
) -> Result<NamedRef, LogicError> {
    let role = read_role_node_by_id(&state.database, role_id)?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    let name = role.role_name;
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ROLE_DELETE,
        EntityRef::Role(&name),
    )?;
    if REQUIRED_ROLES.contains(&name.as_str()) {
        return Err(LogicError::bad_request(format!(
            "role {name} is a required role and cannot be deleted"
        )));
    }
    delete_role_node(&state.database, &name)
        .map_err(|error| LogicError::internal(format!("failed to delete role: {error}")))?;
    Ok(NamedRef {
        id: role_id.to_string(),
        name,
    })
}
