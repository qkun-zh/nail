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
use nail_common::response::role::{RoleListItem, RoleNameView, RoleView};

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

pub async fn create_role(
    state: &AppState,
    actor_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize_global(state, actor_id, PERMISSION_ROLE_CREATE).await?;
    let name = validate_role_name(raw_name)?;
    if read_role_node(&state.graph, &name).await?.is_some() {
        return Err(LogicError::bad_request("role already exists"));
    }
    let role_id = create_role_node(&state.graph, &name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to create role: {error}")))?;
    Ok((role_id, name))
}

pub async fn read_roles(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<nail_common::response::ListPage<RoleListItem>, LogicError> {
    authorize_global(state, actor_id, PERMISSION_ROLE_READ).await?;
    let roles = read_role_nodes(&state.graph).await?;
    let total = roles.len() as u64;
    let (page_roles, has_next) = paginate(roles, page, limit);

    let mut items = Vec::with_capacity(page_roles.len());
    for role in &page_roles {
        let member_count = read_role_members(&state.graph, &role.role_name)
            .await?
            .len() as u64;
        items.push(RoleListItem {
            id: role.id.clone(),
            name: role.role_name.clone(),
            permissions: role.permissions.clone(),
            member_count,
        });
    }
    Ok(nail_common::response::ListPage {
        items,
        has_next,
        total,
    })
}

pub async fn read_role(
    state: &AppState,
    actor_id: &str,
    role_id: &str,
) -> Result<RoleView, LogicError> {
    let role = read_role_node_by_id(&state.graph, role_id)
        .await?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ROLE_READ,
        EntityRef::Role(role.role_name.as_str()),
    )
    .await?;
    let members = read_role_members(&state.graph, &role.role_name).await?;
    Ok(RoleView {
        id: role.id,
        name: role.role_name,
        permissions: role.permissions,
        members,
    })
}

pub async fn update_role(
    state: &AppState,
    actor_id: &str,
    role_id: &str,
    update: RoleUpdate<'_>,
) -> Result<RoleNameView, LogicError> {
    let RoleUpdate {
        permissions_add,
        permissions_remove,
        users_add,
        users_remove,
    } = update;
    let role = read_role_node_by_id(&state.graph, role_id)
        .await?
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
        )
        .await?;
    }
    if has_adds {
        authorize_entity(
            state,
            actor_id,
            PERMISSION_ROLE_GRANT,
            EntityRef::Role(&name),
        )
        .await?;
    }
    if has_removes {
        authorize_entity(
            state,
            actor_id,
            PERMISSION_ROLE_REVOKE,
            EntityRef::Role(&name),
        )
        .await?;
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
        grant_permission_to_role(&state.graph, &name, permission)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to grant {permission}: {error}"))
            })?;
    }
    for permission in permissions_remove {
        revoke_permission_from_role(&state.graph, &name, permission)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to revoke {permission}: {error}"))
            })?;
    }
    for user in users_add {
        hold_role(&state.graph, user, &name)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to hold role for {user}: {error}"))
            })?;
    }
    for user in users_remove {
        unhold_role(&state.graph, user, &name)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to unhold role for {user}: {error}"))
            })?;
    }
    Ok(RoleNameView {
        id: role_id.to_string(),
        name,
    })
}

pub async fn delete_role(
    state: &AppState,
    actor_id: &str,
    role_id: &str,
) -> Result<RoleNameView, LogicError> {
    let role = read_role_node_by_id(&state.graph, role_id)
        .await?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    let name = role.role_name;
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_ROLE_DELETE,
        EntityRef::Role(&name),
    )
    .await?;
    if REQUIRED_ROLES.contains(&name.as_str()) {
        return Err(LogicError::bad_request(format!(
            "role {name} is a required role and cannot be deleted"
        )));
    }
    delete_role_node(&state.graph, &name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete role: {error}")))?;
    Ok(RoleNameView {
        id: role_id.to_string(),
        name,
    })
}
