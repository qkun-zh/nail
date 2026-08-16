use crate::infrastructure::state::AppState;
use crate::logic::authorize::authorize;
use crate::logic::error::{LogicError, database_error};
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_ROLE_MANAGE, PERMISSION_ROLE_REVOKE, REQUIRED_ROLES, ROLE_ADMIN,
    RoleView as RepositoryRoleView, create_role as create_role_node,
    delete_role as delete_role_node, grant_permission_to_role, hold_role,
    read_role as read_role_node, read_role_members, read_roles as read_role_nodes,
    revoke_permission_from_role, unhold_role,
};
use nail_common::response::role::{RoleListItem, RoleListPage, RoleNameView, RoleView};

fn admin_console() -> Resource {
    Resource::Virtual("admin-console".to_string())
}

pub struct RoleUpdate<'a> {
    pub permissions_add: &'a [String],
    pub permissions_remove: &'a [String],
    pub users_add: &'a [String],
    pub users_remove: &'a [String],
}

async fn require_role_manage(state: &AppState, actor_id: &str) -> Result<(), LogicError> {
    authorize(state, actor_id, PERMISSION_ROLE_MANAGE, &admin_console()).await
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
) -> Result<String, LogicError> {
    require_role_manage(state, actor_id).await?;
    let name = validate_role_name(raw_name)?;
    if read_role_node(&state.graph, &name)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return Err(LogicError::bad_request("role already exists"));
    }
    create_role_node(&state.graph, &name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to create role: {error}")))?;
    Ok(name)
}

pub async fn read_roles(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<RoleListPage, LogicError> {
    require_role_manage(state, actor_id).await?;
    let roles = read_role_nodes(&state.graph)
        .await
        .map_err(database_error)?;
    let total = roles.len() as u64;
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let page_roles: Vec<RepositoryRoleView> = roles
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect();

    let mut role_list = Vec::with_capacity(page_roles.len());
    for role in &page_roles {
        let member_count = read_role_members(&state.graph, &role.role_name)
            .await
            .map_err(database_error)?
            .len() as u64;
        role_list.push(RoleListItem {
            name: role.role_name.clone(),
            permissions: role.permissions.clone(),
            member_count,
        });
    }
    let has_next = page < total.div_ceil(limit);
    Ok(RoleListPage {
        role_list,
        has_next,
        total,
    })
}

pub async fn read_role(
    state: &AppState,
    actor_id: &str,
    name: &str,
) -> Result<RoleView, LogicError> {
    require_role_manage(state, actor_id).await?;
    let role = read_role_node(&state.graph, name)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("role not found"))?;
    let members = read_role_members(&state.graph, name)
        .await
        .map_err(database_error)?;
    Ok(RoleView {
        name: role.role_name,
        permissions: role.permissions,
        members,
    })
}

pub async fn update_role(
    state: &AppState,
    actor_id: &str,
    name: &str,
    update: RoleUpdate<'_>,
) -> Result<String, LogicError> {
    let RoleUpdate {
        permissions_add,
        permissions_remove,
        users_add,
        users_remove,
    } = update;
    let has_adds = !permissions_add.is_empty() || !users_add.is_empty();
    let has_removes = !permissions_remove.is_empty() || !users_remove.is_empty();
    if has_adds {
        require_role_manage(state, actor_id).await?;
    }
    if has_removes {
        authorize(
            state,
            actor_id,
            PERMISSION_ROLE_REVOKE,
            &Resource::Role(name.to_string()),
        )
        .await?;
    }
    if read_role_node(&state.graph, name)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(LogicError::not_found("role not found"));
    }
    if REQUIRED_ROLES.contains(&name) && name != ROLE_ADMIN {
        let destructive = !permissions_remove.is_empty() || !users_remove.is_empty();
        if destructive {
            return Err(LogicError::bad_request(format!(
                "role {name} is a required role and cannot be modified destructively"
            )));
        }
    }
    for permission in permissions_add {
        grant_permission_to_role(&state.graph, name, permission)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to grant {permission}: {error}"))
            })?;
    }
    for permission in permissions_remove {
        revoke_permission_from_role(&state.graph, name, permission)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to revoke {permission}: {error}"))
            })?;
    }
    for user in users_add {
        hold_role(&state.graph, user, name).await.map_err(|error| {
            LogicError::internal(format!("failed to hold role for {user}: {error}"))
        })?;
    }
    for user in users_remove {
        unhold_role(&state.graph, user, name)
            .await
            .map_err(|error| {
                LogicError::internal(format!("failed to unhold role for {user}: {error}"))
            })?;
    }
    Ok(name.to_string())
}

pub async fn delete_role(
    state: &AppState,
    actor_id: &str,
    name: &str,
) -> Result<RoleNameView, LogicError> {
    require_role_manage(state, actor_id).await?;
    if REQUIRED_ROLES.contains(&name) {
        return Err(LogicError::bad_request(format!(
            "role {name} is a required role and cannot be deleted"
        )));
    }
    delete_role_node(&state.graph, name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete role: {error}")))?;
    Ok(RoleNameView {
        name: name.to_string(),
    })
}
