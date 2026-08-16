use nail_common::pow::Pow;
use nail_common::request::{DeleteMode, UserDeleteRequest, UserUpdateRequest};
use nail_common::response::EmptyView;
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::{UserIdView, UserNameView, UserView};

use crate::infrastructure::state::AppState;
use crate::logic::authorize::authorize;
use crate::logic::error::{LogicError, database_error};
use crate::logic::pow::verify_issued_pow;
use crate::logic::search::{sync_all_best_effort, sync_article_best_effort, sync_user_best_effort};
use crate::logic::session::normalize_token;
use crate::repository::authorization::Resource;
use crate::repository::cache::token_key;
use crate::repository::role::{
    PERMISSION_USER_DELETE_HARD, PERMISSION_USER_READ, PERMISSION_USER_UPDATE, ROLE_MEMBER,
};
use crate::repository::transfer::TransferError;
use crate::repository::user::{UserWriteError, read_user as read_user_node, update_user_name};

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum UserUpdateView {
    Name(UserNameView),
    SessionToken(SessionTokenView),
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum UserDeleteView {
    Empty(EmptyView),
    UserId(UserIdView),
}

pub async fn create_user(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;
    let token = normalize_token(&pow.payload)
        .ok_or_else(|| LogicError::bad_request("invalid or expired token"))?;

    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let entry = state
        .caches
        .create_user
        .consume(&key)
        .ok_or_else(|| LogicError::bad_request("invalid or expired token"))?;

    let user_id =
        match crate::repository::user::create_user(&state.graph, &entry.email_address_hash).await {
            Ok(user_id) => user_id,
            Err(error) => {
                state.caches.create_user.insert(&key, entry);
                return Err(database_error(error));
            }
        };

    crate::repository::role::hold_role(&state.graph, &user_id, ROLE_MEMBER)
        .await
        .map_err(|error| LogicError::internal(format!("failed to grant member role: {error}")))?;

    tracing::info!(user_id = %user_id, "user created from email token");
    Ok(user_id)
}

pub async fn read_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    name_requested: bool,
    email_hash_requested: bool,
) -> Result<UserView, LogicError> {
    let mut view = UserView::default();
    if target_id == actor_id {
        if name_requested || email_hash_requested {
            let entry = read_user_node(&state.graph, actor_id)
                .await
                .map_err(database_error)?
                .ok_or_else(|| LogicError::unauthorized("user not found"))?;
            if name_requested {
                view.name = Some(entry.name);
            }
            if email_hash_requested {
                view.email_hash = Some(entry.email_address_hash);
            }
        }
        return Ok(view);
    }

    authorize(state, actor_id, PERMISSION_USER_READ, &admin_console()).await?;
    let entry = read_user_node(&state.graph, target_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("user not found"))?;
    view.id = Some(target_id.to_string());
    if name_requested {
        view.name = Some(entry.name);
    }
    if email_hash_requested {
        view.email_hash = Some(entry.email_address_hash);
    }
    Ok(view)
}

pub async fn update_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    request: UserUpdateRequest,
) -> Result<UserUpdateView, LogicError> {
    match (request.old_email_token, request.new_email_token) {
        (Some(old_token), Some(new_token)) => {
            let pow = request.pow.ok_or_else(|| {
                LogicError::bad_request("pow is required to confirm the email update")
            })?;
            let new_session_token = crate::logic::email::update_user_email(
                state, actor_id, &pow, &old_token, &new_token,
            )
            .await?;
            return Ok(UserUpdateView::SessionToken(SessionTokenView {
                session_token: new_session_token,
            }));
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
        sync_user_best_effort(state, target_id).await;
        return Ok(UserUpdateView::Name(UserNameView { name }));
    }

    let pow = request
        .pow
        .ok_or_else(|| LogicError::bad_request("pow is required"))?;
    let name = handle_update_name(state, actor_id, &pow).await?;
    sync_user_best_effort(state, actor_id).await;
    Ok(UserUpdateView::Name(UserNameView { name }))
}

pub async fn delete_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    request: UserDeleteRequest,
) -> Result<UserDeleteView, LogicError> {
    match request.mode {
        Some(DeleteMode::Transfer) => {
            handle_delete_user_transfer(state, actor_id, &request.pow).await?;
            Ok(UserDeleteView::Empty(EmptyView {}))
        }
        Some(DeleteMode::Hard) => {
            handle_delete_user_hard(state, actor_id, target_id).await?;
            Ok(UserDeleteView::UserId(UserIdView {
                user_id: target_id.to_string(),
            }))
        }
        Some(DeleteMode::Soft) => Err(LogicError::bad_request(
            "user delete only supports mode \"transfer\" or \"hard\"",
        )),
        None => Err(LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\" or \"hard\")",
        )),
    }
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
    authorize(state, actor_id, PERMISSION_USER_UPDATE, &admin_console()).await?;
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

async fn handle_delete_user_transfer(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<(), LogicError> {
    verify_issued_pow(state, pow)?;
    let token = normalize_token(&pow.payload)
        .ok_or_else(|| LogicError::bad_request("invalid delete token"))?;
    let token_hash = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash delete token: {error}")))?;

    let Some(entry) = state.caches.delete_user.read(&token_hash) else {
        let user_exists = read_user_node(&state.graph, actor_id)
            .await
            .map_err(database_error)?
            .is_some();
        if user_exists {
            return Err(LogicError::bad_request("invalid or expired delete token"));
        }
        state.caches.session.delete_by_reverse_key(actor_id);
        return Ok(());
    };
    if entry.user_id != actor_id {
        return Err(LogicError::bad_request(
            "delete token does not match your account",
        ));
    }

    let outcome = crate::repository::transfer::transfer_account_assets(&state.graph, actor_id)
        .await
        .map_err(|error| match error {
            TransferError::NoRecycler => LogicError::internal("no recycler available"),
            TransferError::Db(error) => {
                LogicError::internal(format!("failed to transfer account assets: {error}"))
            }
        })?;

    let email_address_hash = entry.email_address_hash;
    state.caches.delete_user.consume(&token_hash);
    state.caches.session.delete_by_reverse_key(actor_id);
    state.caches.email_update.delete(actor_id);
    state.caches.delete_user.delete_by_reverse_key(actor_id);
    state
        .caches
        .create_user
        .delete_by_reverse_key(&email_address_hash);

    for article_id in &outcome.transferred_article_ids {
        sync_article_best_effort(state, article_id).await;
    }
    tracing::info!(user_id = %actor_id, "user deleted, assets transferred");
    Ok(())
}

async fn handle_delete_user_hard(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
) -> Result<(), LogicError> {
    authorize(
        state,
        actor_id,
        PERMISSION_USER_DELETE_HARD,
        &admin_console(),
    )
    .await?;
    let outcome = crate::repository::delete::delete_user(&state.graph, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete user: {error}")))?;
    crate::logic::version::remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
    sync_all_best_effort(state).await;
    Ok(())
}

fn name_update_error(error: UserWriteError) -> LogicError {
    match error {
        UserWriteError::AlreadyTaken => LogicError::bad_request("name already taken"),
        UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
        UserWriteError::EmailMismatch => LogicError::internal("unexpected email mismatch"),
        UserWriteError::Db(error) => {
            LogicError::internal(format!("failed to update name: {error}"))
        }
    }
}

fn admin_console() -> Resource {
    Resource::Virtual("admin-console".to_string())
}
