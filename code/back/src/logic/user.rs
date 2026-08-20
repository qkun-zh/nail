use nail_common::request::{DeleteMode, UserDeleteQuery, UserUpdateRequest};
use nail_common::response::EmptyView;
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::{UserIdView, UserListItem, UserNameView, UserView};

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{
    EntityRef, authorize_anonymous, authorize_entity, authorize_entity_or, authorize_global,
    require_entity_readable,
};
use crate::logic::error::LogicError;
use crate::logic::pagination::paginate;
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

pub async fn create_user(state: &AppState, raw_token: &str) -> Result<String, LogicError> {
    authorize_anonymous(
        state,
        PERMISSION_USER_CREATE,
        &Resource::Virtual("any".to_string()),
    )
    .await?;
    let key = hash_token(
        raw_token,
        LogicError::bad_request("invalid or expired token"),
    )?;

    let entry = state
        .cache
        .user_creation
        .delete(&key)
        .ok_or_else(|| LogicError::bad_request("invalid or expired token"))?;

    let user_id = match crate::repository::user::create_user(&state.database, entry.as_str()).await
    {
        Ok(user_id) => user_id,
        Err(error) => {
            state.cache.user_creation.insert(&key, entry);
            return Err(error.into());
        }
    };

    if crate::repository::delete::is_soft_deleted(&state.database, "user", &user_id).await? {
        return Err(LogicError::bad_request("email address is deactivated"));
    }

    crate::repository::role::hold_role(&state.database, &user_id, ROLE_MEMBER)
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
        let entry = read_user_node(&state.database, target_id)
            .await?
            .ok_or_else(|| LogicError::not_found("user not found"))?;
        if name_requested {
            view.name = Some(entry.name);
        }
        if email_hash_requested {
            view.email_hash = Some(entry.email_address_hash);
        }
    }
    let roles = crate::repository::role::roles_of_user(&state.database, target_id).await?;
    view.roles = Some(roles);
    let articles = crate::repository::article::articles_of_user(&state.database, target_id).await?;
    view.articles = Some(articles);
    Ok(view)
}

pub async fn read_users(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<nail_common::response::ListPage<UserListItem>, LogicError> {
    authorize_global(state, actor_id, PERMISSION_USER_READ).await?;
    let users = read_user_nodes(&state.database).await?;
    let total = users.len() as u64;
    let (page_users, has_next) = paginate(users, page, limit);

    let mut items = Vec::with_capacity(page_users.len());
    for user in &page_users {
        let roles = crate::repository::role::roles_of_user(&state.database, &user.id).await?;
        items.push(UserListItem {
            id: user.id.clone(),
            name: user.name.clone(),
            roles,
        });
    }
    Ok(nail_common::response::ListPage {
        items,
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
            authorize_entity(
                state,
                actor_id,
                PERMISSION_USER_UPDATE,
                EntityRef::User(actor_id),
            )
            .await?;
            let new_session_token =
                crate::logic::email::update_user_email(state, actor_id, &old_token, &new_token)
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

    Err(LogicError::bad_request(
        "name, old_email_token or new_email_token is required",
    ))
}

pub async fn delete_user(
    state: &AppState,
    actor_id: &str,
    target_id: &str,
    query: UserDeleteQuery,
) -> Result<UserDeleteView, LogicError> {
    match query.mode {
        Some(DeleteMode::Transfer) => {
            let token = query
                .token
                .ok_or_else(|| LogicError::bad_request("token is required"))?;
            handle_delete_user_transfer(state, actor_id, &token).await?;
            Ok(UserDeleteView::Empty(EmptyView {}))
        }
        Some(DeleteMode::Soft) => {
            let token = query
                .token
                .ok_or_else(|| LogicError::bad_request("token is required"))?;
            handle_delete_user_soft(state, actor_id, &token).await?;
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
    update_user_name(&state.database, target_id, &name)
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
    raw_token: &str,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_DELETE_TRANSFER,
        EntityRef::User(actor_id),
    )
    .await?;
    let token_hash = hash_token(raw_token, LogicError::bad_request("invalid delete token"))?;

    let Some(entry) = state.cache.user_deletion.read(&token_hash) else {
        let user_exists = read_user_node(&state.database, actor_id).await?.is_some();
        if user_exists {
            return Err(LogicError::bad_request("invalid or expired delete token"));
        }
        let _ = state.cache.session.delete_by_reverse_key(actor_id);
        return Ok(());
    };
    if entry.user_id.as_str() != actor_id {
        return Err(LogicError::bad_request(
            "delete token does not match your account",
        ));
    }

    let outcome =
        crate::repository::transfer::transfer_account_assets(&state.database, actor_id).await?;

    let email_address_hash = entry.email_address_hash.as_str();
    let _ = state.cache.user_deletion.delete(&token_hash);
    let _ = state.cache.session.delete_by_reverse_key(actor_id);
    let _ = state.cache.email_update.delete(actor_id);
    let _ = state.cache.user_deletion.delete_by_reverse_key(actor_id);
    let _ = state
        .cache
        .user_creation
        .delete_by_reverse_key(email_address_hash);

    for article_id in &outcome.transferred_article_ids {
        sync_article_best_effort(state, article_id).await;
    }
    tracing::info!(user_id = %actor_id, "user deleted, assets transferred");
    Ok(())
}

async fn handle_delete_user_soft(
    state: &AppState,
    actor_id: &str,
    raw_token: &str,
) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_USER_DELETE_SOFT,
        EntityRef::User(actor_id),
    )
    .await?;
    let token_hash = hash_token(raw_token, LogicError::bad_request("invalid delete token"))?;

    let Some(entry) = state.cache.user_deletion.read(&token_hash) else {
        let user_exists = read_user_node(&state.database, actor_id).await?.is_some();
        if user_exists {
            return Err(LogicError::bad_request("invalid or expired delete token"));
        }
        let _ = state.cache.session.delete_by_reverse_key(actor_id);
        return Ok(());
    };
    if entry.user_id.as_str() != actor_id {
        return Err(LogicError::bad_request(
            "delete token does not match your account",
        ));
    }

    crate::repository::delete::soft_delete_user(&state.database, actor_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to soft-delete user: {error}")))?;

    let email_address_hash = entry.email_address_hash.as_str();
    let _ = state.cache.user_deletion.delete(&token_hash);
    let _ = state.cache.session.delete_by_reverse_key(actor_id);
    let _ = state.cache.email_update.delete(actor_id);
    let _ = state.cache.user_deletion.delete_by_reverse_key(actor_id);
    let _ = state
        .cache
        .user_creation
        .delete_by_reverse_key(email_address_hash);

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
    crate::repository::delete::undelete_soft_user(&state.database, target_id)
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
    let outcome = crate::repository::delete::delete_user(&state.database, target_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete user: {error}")))?;
    crate::logic::version::remove_orphaned_pdfs(state, &outcome.removed_pdf_hashes).await;
    sync_all_best_effort(state).await;
    Ok(())
}
