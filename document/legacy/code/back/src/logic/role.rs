
use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{authorize};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo::authorization::{
    ROLE_ADMIN, ROLE_RECYCLER, REQUIRED_ROLES,
    apply_tag_to_role, create_role, delete_role, grant_permission_to_role, hold_role,
    list_roles, read_role_authorization, read_role_members, remove_tag_from_role,
    revoke_permission_from_role, unhold_role,
};

fn admin_console() -> Resource {
    Resource::System("admin-console".to_string())
}

async fn gate_role_manage(state: &AppState, session_token: &str) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(state, &user_id, "Role::Manage", &admin_console()).await?;
    Ok(user_id)
}

#[derive(serde::Serialize)]
pub struct RoleListItem {
    pub name: String,
    pub permissions: Vec<String>,
    pub scopes: Vec<String>,
    pub member_count: u64,
}

pub async fn handle_create_role(
    state: &AppState,
    session_token: &str,
    name: &str,
) -> Result<String, LogicError> {
    gate_role_manage(state, session_token).await?;
    let name = validate_role_name(name)?;
    create_role(&state.db, &name)
        .await
        .map_err(|e| LogicError::internal(format!("failed to create role: {e}")))?;
    Ok(name)
}

pub async fn handle_read_roles(
    state: &AppState,
    session_token: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<RoleListItem>, u64), LogicError> {
    gate_role_manage(state, session_token).await?;
    let roles = list_roles(&state.db)
        .await
        .map_err(|e| LogicError::internal(format!("failed to list roles: {e}")))?;
    let total = roles.len() as u64;
    let page: Vec<RoleListItem> = roles
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|r| RoleListItem {
            name: r.role_name,
            permissions: r.permissions,
            scopes: r.scopes,
            member_count: 0,
        })
        .collect();
    Ok((page, total))
}

pub async fn handle_read_role(
    state: &AppState,
    session_token: &str,
    name: &str,
) -> Result<serde_json::Value, LogicError> {
    gate_role_manage(state, session_token).await?;
    let Some(role) = read_role_authorization(&state.db, name)
        .await
        .map_err(|e| LogicError::internal(format!("failed to read role: {e}")))?
    else {
        return Err(LogicError::not_found("role not found"));
    };
    let members = read_role_members(&state.db, name)
        .await
        .map_err(|e| LogicError::internal(format!("failed to read role members: {e}")))?;
    Ok(serde_json::json!({
        "name": role.role_name,
        "permissions": role.permissions,
        "scopes": role.scopes,
        "members": members,
    }))
}

pub async fn handle_update_role(
    state: &AppState,
    session_token: &str,
    name: &str,
    permissions_add: &[String],
    permissions_remove: &[String],
    tags_add: &[String],
    tags_remove: &[String],
    users_add: &[String],
    users_remove: &[String],
) -> Result<String, LogicError> {
    gate_role_manage(state, session_token).await?;
    if REQUIRED_ROLES.contains(&name) {
        let protected = !permissions_remove.is_empty()
            || !tags_remove.is_empty()
            || !users_remove.is_empty();
        if protected {
            return Err(LogicError::bad_request(format!(
                "role {name} is a required role and cannot be modified destructively"
            )));
        }
    }
    for p in permissions_add {
        grant_permission_to_role(&state.db, name, p)
            .await
            .map_err(|e| LogicError::internal(format!("failed to grant {p}: {e}")))?;
    }
    for p in permissions_remove {
        revoke_permission_from_role(&state.db, name, p)
            .await
            .map_err(|e| LogicError::internal(format!("failed to revoke {p}: {e}")))?;
    }
    for t in tags_add {
        apply_tag_to_role(&state.db, name, t)
            .await
            .map_err(|e| LogicError::internal(format!("failed to apply tag {t}: {e}")))?;
    }
    for t in tags_remove {
        remove_tag_from_role(&state.db, name, t)
            .await
            .map_err(|e| LogicError::internal(format!("failed to remove tag {t}: {e}")))?;
    }
    for u in users_add {
        hold_role(&state.db, u, name)
            .await
            .map_err(|e| LogicError::internal(format!("failed to hold role for {u}: {e}")))?;
    }
    for u in users_remove {
        unhold_role(&state.db, u, name)
            .await
            .map_err(|e| LogicError::internal(format!("failed to unhold role for {u}: {e}")))?;
    }
    Ok(name.to_string())
}

pub async fn handle_delete_role(
    state: &AppState,
    session_token: &str,
    name: &str,
) -> Result<(), LogicError> {
    gate_role_manage(state, session_token).await?;
    if name == ROLE_ADMIN || name == ROLE_RECYCLER {
        return Err(LogicError::bad_request(format!(
            "role {name} is a required role and cannot be deleted"
        )));
    }
    delete_role(&state.db, name)
        .await
        .map_err(|e| LogicError::internal(format!("failed to delete role: {e}")))?;
    Ok(())
}

fn validate_role_name(name: &str) -> Result<String, LogicError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(LogicError::bad_request("invalid role name"));
    }
    if !trimmed
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(LogicError::bad_request("invalid role name"));
    }
    Ok(trimmed.to_string())
}
