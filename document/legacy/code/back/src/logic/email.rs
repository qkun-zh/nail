
use common::hash;
use common::pow::Pow;
use uuid::Uuid;

use crate::logic::authenticate::{authenticate_session, verify_issued_pow};
use crate::logic::authenticate::{normalize_email, normalize_token, validate_email};
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo;

pub async fn handle_email_update_send(
    state: &AppState,
    old_email_pow: &Pow,
    new_email_pow: &Pow,
    session_token: &str,
) -> Result<(String, String), LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    verify_issued_pow(state, old_email_pow)?;
    verify_issued_pow(state, new_email_pow)?;

    let old_email = normalize_email(&old_email_pow.payload);
    let new_email = normalize_email(&new_email_pow.payload);
    if old_email == new_email {
        return Err(LogicError::bad_request(
            "new email must be different from old email",
        ));
    }

    let user_entry = repo::user::read_user(&state.db, &user_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    if user_entry.email_address_hash != hash::email(&old_email) {
        return Err(LogicError::bad_request(
            "old email does not match your current email",
        ));
    }

    let allowed_domains = &state.config.email.allowed_domains;
    if !validate_email(&old_email, allowed_domains) || !validate_email(&new_email, allowed_domains)
    {
        return Err(LogicError::bad_request("email domain not allowed"));
    }

    let new_email_address_hash = hash::email(&new_email);
    if let Some(existing_user_id) =
        repo::user::find_user_by_email_address_hash(&state.db, &new_email_address_hash)
            .await
            .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();
    let old_email_address_hash = user_entry.email_address_hash;

    let old_email_subject = Uuid::now_v7().to_string();
    match state
        .email
        .send_email(&old_email, &old_email_subject, &old_token)
        .await
    {
        Ok(()) => {}
        Err(crate::other::email::SendEmailError::RateLimited) => {
            return Err(LogicError::bad_request(
                "email already sent recently, check your inbox",
            ));
        }
        Err(crate::other::email::SendEmailError::Smtp(e)) => {
            tracing::warn!(error = %e, "failed to send confirmation to old email");
            return Err(LogicError::internal(
                "failed to send confirmation to old email",
            ));
        }
    }

    let new_email_subject = Uuid::now_v7().to_string();
    match state
        .email
        .send_email(&new_email, &new_email_subject, &new_token)
        .await
    {
        Ok(()) => {}
        Err(crate::other::email::SendEmailError::RateLimited) => {
            return Err(LogicError::bad_request(
                "email already sent recently, check your inbox",
            ));
        }
        Err(crate::other::email::SendEmailError::Smtp(e)) => {
            tracing::warn!(error = %e, "failed to send confirmation to new email");
            return Err(LogicError::internal(
                "failed to send confirmation to new email",
            ));
        }
    }

    repo::token::email_update::create_email_update_token(
        &state.cache,
        &user_id,
        &old_email_address_hash,
        &new_email_address_hash,
        &hash::token(&old_token),
        &hash::token(&new_token),
    );

    tracing::info!(
        old_hash = %old_email_address_hash,
        new_hash = %new_email_address_hash,
        "email update requested, confirmation emails sent"
    );

    Ok((old_email_subject, new_email_subject))
}

pub async fn handle_email_update_confirm(
    state: &AppState,
    pow: &Pow,
    raw_old_email_token: &str,
    raw_new_email_token: &str,
    session_token: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    verify_issued_pow(state, pow)?;

    let raw_old = raw_old_email_token.trim();
    let raw_new = raw_new_email_token.trim();
    let old_email_token = normalize_token(raw_old)
        .ok_or_else(|| LogicError::bad_request("invalid old email token"))?;
    let new_email_token = normalize_token(raw_new)
        .ok_or_else(|| LogicError::bad_request("invalid new email token"))?;

    let expected_canonical = format!("{}\n{}", old_email_token, new_email_token);
    let expected_raw = format!("{}\n{}", raw_old, raw_new);
    if pow.payload != expected_canonical && pow.payload != expected_raw {
        return Err(LogicError::bad_request("PoW payload does not match token"));
    }
    if old_email_token == new_email_token {
        return Err(LogicError::bad_request(
            "old token and new token must be different",
        ));
    }

    let entry = repo::token::email_update::read_email_update_token(&state.cache, &user_id)
        .ok_or_else(|| LogicError::bad_request("invalid or expired email update request"))?;

    if entry.token_from_old_email_hash != hash::token(&old_email_token)
        || entry.token_from_new_email_hash != hash::token(&new_email_token)
    {
        return Err(LogicError::bad_request("token mismatch"));
    }


    let old_email_address_hash = entry.old_email_address_hash;
    let new_email_address_hash = entry.new_email_address_hash;
    if let Some(existing_user_id) =
        repo::user::find_user_by_email_address_hash(&state.db, &new_email_address_hash)
            .await
            .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    let updated = repo::user::update_user_email(
        &state.db,
        &user_id,
        &old_email_address_hash,
        &new_email_address_hash,
    )
    .await
    .map_err(|e| match e {
        repo::user::UserWriteError::AlreadyTaken => {
            LogicError::bad_request("new email is already used by another account")
        }
        repo::user::UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
        repo::user::UserWriteError::Db(e) => {
            LogicError::internal(format!("failed to update email: {e}"))
        }
    })?;
    if !updated {
        return Err(LogicError::bad_request("email has already been changed"));
    }

    repo::token::email_update::consume_email_update_token_if_matches(
        &state.cache,
        &user_id,
        &hash::token(&old_email_token),
        &hash::token(&new_email_token),
    );
    repo::token::session::delete_session_tokens_by_user_id(&state.cache, &user_id);
    repo::token::authenticate::delete_authenticate_tokens_by_email_address_hash(
        &state.cache,
        &old_email_address_hash,
    );
    repo::token::deregister::delete_deregister_tokens_by_user_id(&state.cache, &user_id);

    let new_session_token = Uuid::now_v7().to_string();
    repo::token::session::create_session_token(&state.cache, &new_session_token, &user_id);

    tracing::info!(user_id = %user_id, "email updated");

    Ok(new_session_token)
}
