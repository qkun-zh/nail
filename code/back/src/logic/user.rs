use nail_common::pow::Pow;
use nail_common::request::{DeleteMode, UserDeleteRequest, UserUpdateRequest};

use crate::infrastructure::state::AppState;
use crate::logic::authenticate::normalize_token;
use crate::logic::error::LogicError;
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::token_key;
use crate::repository::role::{
    PERMISSION_USER_DELETE, PERMISSION_USER_READ, PERMISSION_USER_UPDATE, user_holds_permission,
};
use crate::repository::transfer::TransferError;
use crate::repository::user::{UserWriteError, read_user, update_user_name};

pub async fn read_user_profile(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    name_requested: bool,
    email_hash_requested: bool,
) -> Result<serde_json::Value, LogicError> {
    let mut data = serde_json::Map::new();
    if target_id == actor_id {
        if name_requested || email_hash_requested {
            let entry = read_user(&state.graph, actor_id)
                .await
                .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
                .ok_or_else(|| LogicError::unauthorized("user not found"))?;
            if name_requested {
                data.insert("name".to_string(), serde_json::json!(entry.name));
            }
            if email_hash_requested {
                data.insert(
                    "email_hash".to_string(),
                    serde_json::json!(entry.email_address_hash),
                );
            }
        }
        return Ok(serde_json::Value::Object(data));
    }

    require_permission(state, actor_id, PERMISSION_USER_READ).await?;
    let entry = read_user(&state.graph, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::not_found("user not found"))?;
    data.insert("id".to_string(), serde_json::json!(target_id));
    if name_requested {
        data.insert("name".to_string(), serde_json::json!(entry.name));
    }
    if email_hash_requested {
        data.insert(
            "email_hash".to_string(),
            serde_json::json!(entry.email_address_hash),
        );
    }
    Ok(serde_json::Value::Object(data))
}

pub async fn update_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    request: UserUpdateRequest,
) -> Result<serde_json::Value, LogicError> {
    match (request.old_email_token, request.new_email_token) {
        (Some(old_token), Some(new_token)) => {
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required to confirm the email update"))?;
            let new_session_token = crate::logic::email::handle_email_update_confirm(
                state,
                actor_id,
                &pow,
                &old_token,
                &new_token,
            )
            .await?;
            return Ok(serde_json::json!({ "session_token": new_session_token }));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(LogicError::bad_request(
                "old_email_token and new_email_token must both be provided",
            ));
        }
        (None, None) => {}
    }

    if let Some(raw_name) = request.name {
        let name = handle_admin_update_name(state, actor_id, target_id, &raw_name).await?;
        return Ok(serde_json::json!({ "name": name }));
    }

    let pow = request
        .pow
        .ok_or_else(|| LogicError::bad_request("pow is required"))?;
    let name = handle_update_name(state, actor_id, &pow).await?;
    Ok(serde_json::json!({ "name": name }))
}

pub async fn delete_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    request: UserDeleteRequest,
) -> Result<serde_json::Value, LogicError> {
    match request.mode {
        Some(DeleteMode::Transfer) => {
            handle_deregister_confirm(state, actor_id, &request.pow).await?;
            Ok(serde_json::json!({}))
        }
        Some(DeleteMode::Hard) => {
            handle_hard_delete_user(state, actor_id, target_id).await?;
            Ok(serde_json::json!({ "user_id": target_id }))
        }
        None => Err(LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
        )),
    }
}

pub async fn list_users(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<serde_json::Value, LogicError> {
    require_permission(state, actor_id, PERMISSION_USER_READ).await?;
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, total) = crate::repository::user::list_users(&state.graph, limit, offset)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    let user_list: Vec<serde_json::Value> = items
        .into_iter()
        .map(|item| {
            serde_json::json!({
                "id": item.id,
                "name": item.name,
                "email_hash": item.email_address_hash,
            })
        })
        .collect();
    let has_next = page < total.div_ceil(limit);
    Ok(serde_json::json!({
        "user_list": user_list,
        "has_next": has_next,
        "total": total,
    }))
}

async fn handle_update_name(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;
    let name = nail_common::name::validate_name(&pow.payload)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    update_user_name(&state.graph, actor_id, &name)
        .await
        .map_err(name_update_error)?;
    Ok(name)
}

async fn handle_admin_update_name(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    raw_name: &str,
) -> Result<String, LogicError> {
    require_permission(state, actor_id, PERMISSION_USER_UPDATE).await?;
    let name = nail_common::name::validate_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    update_user_name(&state.graph, target_id, &name)
        .await
        .map_err(|error| match error {
            UserWriteError::UserMissing => LogicError::not_found("user not found"),
            other => name_update_error(other),
        })?;
    Ok(name)
}

async fn handle_deregister_confirm(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<(), LogicError> {
    verify_issued_pow(state, pow)?;
    let token = normalize_token(&pow.payload)
        .ok_or_else(|| LogicError::bad_request("invalid deregister token"))?;
    let token_hash = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash deregister token: {error}")))?;

    let Some(entry) = state.caches.deregister.read(&token_hash) else {
        let user_exists = read_user(&state.graph, actor_id)
            .await
            .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
            .is_some();
        if user_exists {
            return Err(LogicError::bad_request("invalid or expired deregister token"));
        }
        state.caches.session.delete_by_reverse_key(actor_id);
        return Ok(());
    };
    if entry.user_id != actor_id {
        return Err(LogicError::bad_request(
            "deregister token does not match your account",
        ));
    }

    crate::repository::transfer::transfer_account_assets(&state.graph, actor_id)
        .await
        .map_err(|error| match error {
            TransferError::NoRecycler => LogicError::internal("no recycler available"),
            TransferError::Db(error) => {
                LogicError::internal(format!("failed to transfer account assets: {error}"))
            }
        })?;

    let email_address_hash = entry.email_address_hash;
    state.caches.deregister.consume(&token_hash);
    state.caches.session.delete_by_reverse_key(actor_id);
    state.caches.email_update.delete(actor_id);
    state.caches.deregister.delete_by_reverse_key(actor_id);
    state
        .caches
        .authenticate
        .delete_by_reverse_key(&email_address_hash);

    tracing::info!(user_id = %actor_id, "account deregistered, assets transferred");
    Ok(())
}

async fn handle_hard_delete_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
) -> Result<(), LogicError> {
    require_permission(state, actor_id, PERMISSION_USER_DELETE).await?;
    crate::repository::delete::hard_delete_user(&state.graph, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete user: {error}")))?;
    Ok(())
}

async fn require_permission(
    state: &AppState,
    actor_id: &str,
    permission: &str,
) -> Result<(), LogicError> {
    let granted = user_holds_permission(&state.graph, actor_id, permission)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    if granted {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

fn name_update_error(error: UserWriteError) -> LogicError {
    match error {
        UserWriteError::AlreadyTaken => LogicError::bad_request("name already taken"),
        UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
        UserWriteError::EmailMismatch => LogicError::internal("unexpected email mismatch"),
        UserWriteError::Db(error) => LogicError::internal(format!("failed to update name: {error}")),
    }
}
