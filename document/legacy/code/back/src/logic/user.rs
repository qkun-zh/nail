
use common::hash;
use common::pow::Pow;
use uuid::Uuid;

use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{authorize};
use crate::logic::authenticate::{
    authenticate_session, normalize_email, normalize_token, verify_issued_pow,
};
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo;

pub async fn handle_logout(
    state: &AppState,
    pow: &Pow,
    session_token: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    verify_issued_pow(state, pow)?;
    repo::token::session::delete_session_token(&state.cache, session_token);
    tracing::info!(user_id = %user_id, "user logged out");
    Ok(())
}

pub async fn handle_deregister_request(
    state: &AppState,
    pow: &Pow,
    session_token: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    verify_issued_pow(state, pow)?;

    let email = normalize_email(&pow.payload);
    let user_entry = repo::user::read_user(&state.db, &user_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;

    if user_entry.email_address_hash != hash::email(&email) {
        return Err(LogicError::bad_request("email does not match your account"));
    }

    let token = Uuid::now_v7().to_string();

    let email_subject = Uuid::now_v7().to_string();
    match state.email.send_email(&email, &email_subject, &token).await {
        Ok(()) => {}
        Err(crate::other::email::SendEmailError::RateLimited) => {
            return Err(LogicError::bad_request(
                "email already sent recently, check your inbox",
            ));
        }
        Err(crate::other::email::SendEmailError::Smtp(e)) => {
            tracing::warn!(target: "email", error = %e, "failed to send deregister confirmation email");
            return Err(LogicError::internal("failed to send confirmation email"));
        }
    }

    repo::token::deregister::create_deregister_token(
        &state.cache,
        &token,
        &user_id,
        &user_entry.email_address_hash,
    );

    tracing::info!(email_hash = %user_entry.email_address_hash, "deregister confirmation email sent");

    Ok(email_subject)
}

pub async fn handle_deregister_confirm(
    state: &AppState,
    pow: &Pow,
    session_token: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    verify_issued_pow(state, pow)?;
    let token = normalize_token(&pow.payload)
        .ok_or_else(|| LogicError::bad_request("invalid deregister token"))?;

    let Some(entry_user_id) =
        repo::token::deregister::find_user_id_by_deregister_token(&state.cache, &token)
    else {
        let user_exists = repo::user::read_user(&state.db, &user_id)
            .await
            .ok()
            .flatten()
            .is_some();
        if !user_exists {
            repo::token::session::delete_session_tokens_by_user_id(&state.cache, &user_id);
            return Ok(());
        }
        return Err(LogicError::bad_request(
            "invalid or expired deregister token",
        ));
    };
    if entry_user_id != user_id {
        return Err(LogicError::bad_request(
            "deregister token does not match your account",
        ));
    }

    let transferred_article_ids = repo::search::article_ids_of_user(&state.db, &user_id)
        .await
        .unwrap_or_default();

    let outcome = repo::article::transfer_account_assets(&state.db, &user_id)
        .await
        .map_err(|e| {
            LogicError::internal(format!("failed to transfer account assets: {e}"))
        })?;
    tracing::info!(
        user_id = %user_id,
        article_edges = outcome.transferred_article_edges,
        comment_edges = outcome.transferred_comment_edges,
        "account assets transferred to recycler"
    );

    let email_address_hash =
        match repo::token::deregister::consume_deregister_token(&state.cache, &token) {
            Some(entry) => entry.email_address_hash,
            None => {
                repo::token::session::delete_session_tokens_by_user_id(&state.cache, &user_id);
                repo::token::email_update::delete_email_update_token(&state.cache, &user_id);
                repo::token::deregister::delete_deregister_tokens_by_user_id(
                    &state.cache,
                    &user_id,
                );
                return Ok(());
            }
        };

    repo::token::session::delete_session_tokens_by_user_id(&state.cache, &user_id);
    repo::token::email_update::delete_email_update_token(&state.cache, &user_id);
    repo::token::deregister::delete_deregister_tokens_by_user_id(&state.cache, &user_id);
    repo::token::authenticate::delete_authenticate_tokens_by_email_address_hash(
        &state.cache,
        &email_address_hash,
    );

    for article_id in &transferred_article_ids {
        if let Err(e) = repo::search::sync_article(&state.search, &state.db, article_id).await {
            tracing::warn!(article_id = %article_id, error = %e, "search index sync after deregister failed");
        }
    }

    tracing::info!(user_id = %user_id, "account deregistered, assets transferred");

    Ok(())
}

pub async fn handle_read_name(state: &AppState, session_token: &str) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    let user_entry = repo::user::read_user(&state.db, &user_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    Ok(user_entry.name)
}

pub async fn read_author_names_by_user(
    state: &AppState,
    user_ids: &[String],
) -> std::collections::HashMap<String, String> {
    match repo::user::read_user_names_by_ids(&state.db, user_ids).await {
        Ok(rows) => rows.into_iter().collect(),
        Err(e) => {
            tracing::warn!(target: "user", error = %e, "failed to load author names");
            std::collections::HashMap::new()
        }
    }
}

pub async fn handle_update_name(
    state: &AppState,
    pow: &Pow,
    session_token: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    verify_issued_pow(state, pow)?;

    let name = common::name::validate_name(&pow.payload)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;

    let updated = repo::user::update_user_name(&state.db, &user_id, &name)
        .await
        .map_err(|e| match e {
            repo::user::UserWriteError::AlreadyTaken => {
                LogicError::bad_request("name already taken")
            }
            repo::user::UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
            repo::user::UserWriteError::Db(e) => {
                LogicError::internal(format!("failed to update name: {e}"))
            }
        })?;
    if !updated {
        return Err(LogicError::unauthorized("user not found"));
    }
    if let Err(e) = repo::search::sync_articles_of_user(&state.search, &state.db, &user_id).await {
        tracing::warn!(user_id = %user_id, error = %e, "search index sync after name update failed");
    }
    Ok(name)
}

fn admin_console() -> Resource {
    Resource::System("admin-console".to_string())
}

pub async fn handle_list_users(
    state: &AppState,
    session_token: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<(String, String, String)>, u64), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    authorize(state, &user_id, "User::Read", &admin_console()).await?;
    let (items, total) = repo::user::list_users(&state.db, limit, offset)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?;
    Ok((
        items
            .into_iter()
            .map(|u| (u.id, u.name, u.email_address_hash))
            .collect(),
        total,
    ))
}

pub async fn handle_read_self_email_hash(
    state: &AppState,
    user_id: &str,
) -> Result<Option<String>, LogicError> {
    Ok(repo::user::read_user(&state.db, user_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .map(|u| u.email_address_hash))
}

pub async fn handle_read_user_manage(
    state: &AppState,
    session_token: &str,
    user_id: &str,
) -> Result<serde_json::Value, LogicError> {
    let actor = authenticate_session(state, session_token)?;
    authorize(state, &actor, "User::Read", &admin_console()).await?;
    let entry = repo::user::read_user(&state.db, user_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("user not found"))?;
    Ok(serde_json::json!({
        "id": user_id,
        "name": entry.name,
        "email_hash": entry.email_address_hash,
    }))
}

pub async fn handle_admin_update_name(
    state: &AppState,
    session_token: &str,
    target_user_id: &str,
    raw_name: &str,
) -> Result<String, LogicError> {
    let actor = authenticate_session(state, session_token)?;
    authorize(state, &actor, "User::Update", &admin_console()).await?;
    let name = common::name::validate_name(raw_name)
        .map_err(|e| LogicError::bad_request(e.to_string()))?;
    let updated = repo::user::update_user_name(&state.db, target_user_id, &name)
        .await
        .map_err(|e| match e {
            repo::user::UserWriteError::AlreadyTaken => {
                LogicError::bad_request("name already taken")
            }
            repo::user::UserWriteError::UserMissing => LogicError::not_found("user not found"),
            repo::user::UserWriteError::Db(e) => {
                LogicError::internal(format!("failed to update name: {e}"))
            }
        })?;
    if !updated {
        return Err(LogicError::not_found("user not found"));
    }
    if let Err(e) =
        repo::search::sync_articles_of_user(&state.search, &state.db, target_user_id).await
    {
        tracing::warn!(user_id = %target_user_id, error = %e, "search index sync after admin name update failed");
    }
    Ok(name)
}

pub async fn handle_hard_delete_user(
    state: &AppState,
    session_token: &str,
    target_user_id: &str,
) -> Result<(), LogicError> {
    let actor = authenticate_session(state, session_token)?;
    authorize(state, &actor, "User::Delete", &admin_console()).await?;
    let outcome = repo::hard_delete::hard_delete_user(&state.db, target_user_id)
        .await
        .map_err(|e| LogicError::internal(format!("failed to delete user: {e}")))?;
    crate::logic::version::cleanup_pdf_files_by_hashes(state, &outcome.removed_pdf_hashes).await;
    if let Err(e) = repo::search::rebuild_index(&state.search, &state.db).await {
        tracing::warn!(user_id = %target_user_id, error = %e, "search index rebuild after user hard delete failed");
    }
    Ok(())
}
