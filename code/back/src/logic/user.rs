use nail_common::pow::Pow;
use nail_common::request::{DeleteMode, UserDeleteRequest, UserUpdateRequest};
use nail_common::response::EmptyView;
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::{UserIdView, UserListPage, UserNameView, UserView};

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{
    EntityRef, authorize_anonymous, authorize_entity, authorize_entity_or, authorize_global,
    require_entity_readable,
};
use crate::logic::error::LogicError;
use crate::logic::pagination::paginate;
use crate::logic::pow::verify_issued_pow;
use crate::logic::search::{sync_all_best_effort, sync_article_best_effort, sync_user_best_effort};
use crate::logic::session::hash_token;
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_USER_CREATE, PERMISSION_USER_DELETE_HARD, PERMISSION_USER_DELETE_SOFT,
    PERMISSION_USER_DELETE_TRANSFER, PERMISSION_USER_READ, PERMISSION_USER_UNDELETE_SOFT,
    PERMISSION_USER_UPDATE, ROLE_MEMBER,
};
use crate::repository::user::{
    UserWriteError, read_user as read_user_node, read_users as read_user_nodes, update_user_name,
};

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
    authorize_anonymous(
        state,
        PERMISSION_USER_CREATE,
        &Resource::Virtual("any".to_string()),
    )
    .await?;
    verify_issued_pow(state, pow)?;
    let key = hash_token(
        &pow.payload,
        LogicError::bad_request("invalid or expired token"),
    )?;

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
                return Err(error.into());
            }
        };

    if crate::repository::delete::is_soft_deleted(&state.graph, "user", &user_id).await? {
        return Err(LogicError::bad_request("email address is deactivated"));
    }

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
    require_entity_readable(state, actor_id, EntityRef::User(target_id)).await?;

    let mut view = UserView {
        id: Some(target_id.to_string()),
        ..UserView::default()
    };
    if name_requested || email_hash_requested {
        let entry = read_user_node(&state.graph, target_id)
            .await?
            .ok_or_else(|| LogicError::not_found("user not found"))?;
        if name_requested {
            view.name = Some(entry.name);
        }
        if email_hash_requested {
            view.email_hash = Some(entry.email_address_hash);
        }
    }
    let roles = crate::repository::role::roles_of_user(&state.graph, target_id).await?;
    view.roles = Some(roles);
    let articles = crate::repository::article::articles_of_user(&state.graph, target_id).await?;
    view.articles = Some(articles);
    Ok(view)
}

pub async fn read_users(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<UserListPage, LogicError> {
    authorize_global(state, actor_id, PERMISSION_USER_READ).await?;
    let users = read_user_nodes(&state.graph).await?;
    let total = users.len() as u64;
    let (page_users, has_next) = paginate(users, page, limit);

    let mut user_list = Vec::with_capacity(page_users.len());
    for user in &page_users {
        let roles = crate::repository::role::roles_of_user(&state.graph, &user.id).await?;
        user_list.push(nail_common::response::user::UserListItem {
            id: user.id.clone(),
            name: user.name.clone(),
            roles,
        });
    }
    Ok(UserListPage {
        user_list,
        has_next,
        total,
    })
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
            authorize_entity(
                state,
                actor_id,
                PERMISSION_USER_UPDATE,
                EntityRef::User(actor_id),
            )
            .await?;
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
        Some(DeleteMode::Soft) => {
            handle_delete_user_soft(state, actor_id, &request.pow).await?;
            Ok(UserDeleteView::Empty(EmptyView {}))
        }
        Some(DeleteMode::Hard) => {
            handle_delete_user_hard(state, actor_id, target_id).await?;
            Ok(UserDeleteView::UserId(UserIdView {
                user_id: target_id.to_string(),
            }))
        }
        None => Err(LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\", \"soft\" or \"hard\")",
        )),
    }
}

async fn handle_update_name(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<String, LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_UPDATE,
        EntityRef::User(actor_id),
    )
    .await?;
    verify_issued_pow(state, pow)?;
    let name = nail_common::name::validate_name(&pow.payload)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    update_user_name(&state.graph, actor_id, &name).await?;
    Ok(name)
}

async fn handle_admin_update_name(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    raw_name: &str,
) -> Result<String, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_USER_UPDATE,
        EntityRef::User(target_id),
    )
    .await?;
    let name = nail_common::name::validate_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    update_user_name(&state.graph, target_id, &name)
        .await
        .map_err(|error| match error {
            UserWriteError::UserMissing => LogicError::not_found("user not found"),
            other => other.into(),
        })?;
    Ok(name)
}

async fn handle_delete_user_transfer(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_DELETE_TRANSFER,
        EntityRef::User(actor_id),
    )
    .await?;
    verify_issued_pow(state, pow)?;
    let token_hash = hash_token(
        &pow.payload,
        LogicError::bad_request("invalid delete token"),
    )?;

    let Some(entry) = state.caches.delete_user.read(&token_hash) else {
        let user_exists = read_user_node(&state.graph, actor_id).await?.is_some();
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

    let outcome =
        crate::repository::transfer::transfer_account_assets(&state.graph, actor_id).await?;

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

async fn handle_delete_user_soft(
    state: &AppState,
    actor_id: &str,
    pow: &Pow,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_DELETE_SOFT,
        EntityRef::User(actor_id),
    )
    .await?;
    verify_issued_pow(state, pow)?;
    let token_hash = hash_token(
        &pow.payload,
        LogicError::bad_request("invalid delete token"),
    )?;

    let Some(entry) = state.caches.delete_user.read(&token_hash) else {
        let user_exists = read_user_node(&state.graph, actor_id).await?.is_some();
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

    crate::repository::delete::soft_delete_user(&state.graph, actor_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to soft-delete user: {error}")))?;

    let email_address_hash = entry.email_address_hash;
    state.caches.delete_user.consume(&token_hash);
    state.caches.session.delete_by_reverse_key(actor_id);
    state.caches.email_update.delete(actor_id);
    state.caches.delete_user.delete_by_reverse_key(actor_id);
    state
        .caches
        .create_user
        .delete_by_reverse_key(&email_address_hash);

    sync_all_best_effort(state).await;
    tracing::info!(user_id = %actor_id, "user soft-deleted");
    Ok(())
}

pub async fn undelete_soft_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_UNDELETE_SOFT,
        EntityRef::User(target_id),
    )
    .await?;
    crate::repository::delete::undelete_soft_user(&state.graph, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to undelete user: {error}")))?;
    sync_all_best_effort(state).await;
    Ok(())
}

async fn handle_delete_user_hard(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_DELETE_HARD,
        EntityRef::User(target_id),
    )
    .await?;
    let outcome = crate::repository::delete::delete_user(&state.graph, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete user: {error}")))?;
    crate::logic::version::remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
    sync_all_best_effort(state).await;
    Ok(())
}
