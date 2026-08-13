use email_address::{EmailAddress, Options};
use nail_common::pow::Pow;
use nail_common::request::{EmailReadIntent, EmailReadRequest};
use uuid::Uuid;

use crate::infrastructure::email::SendEmailError;
use crate::infrastructure::state::AppState;
use crate::logic::authenticate::{authenticate_session, normalize_token};
use crate::logic::error::LogicError;
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::{
    AuthenticateTokenEntry, DeregisterTokenEntry, EmailUpdateTokenEntry, SessionTokenEntry, token_key,
};
use crate::repository::user::{
    UserWriteError, find_user_by_email_address_hash, read_user, update_user_email,
};

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub fn parse_intent(value: &str) -> Option<EmailReadIntent> {
    match value {
        "authenticate" => Some(EmailReadIntent::Authenticate),
        "change_email" => Some(EmailReadIntent::ChangeEmail),
        "deregister" => Some(EmailReadIntent::Deregister),
        _ => None,
    }
}

pub fn validate_email(email: &str, allowed_domains: &[String]) -> bool {
    if email.len() > 254 {
        return false;
    }
    let Ok(parsed) = EmailAddress::parse_with_options(email, Options::default()) else {
        return false;
    };
    let domain = parsed.domain().to_lowercase();
    allowed_domains.iter().any(|allowed| allowed == &domain)
}

pub async fn handle_email_read(
    state: &AppState,
    intent: EmailReadIntent,
    request: EmailReadRequest,
    session_token: Option<String>,
) -> Result<serde_json::Value, LogicError> {
    match intent {
        EmailReadIntent::Authenticate => {
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required"))?;
            let email_subject = handle_email_auth_request(state, &pow).await?;
            Ok(serde_json::json!({ "email_subject": email_subject }))
        }
        EmailReadIntent::ChangeEmail => {
            let user_id = require_session_user(state, session_token)?;
            let old_email_pow = request
                .old_email_pow
                .ok_or_else(|| LogicError::bad_request("old_email_pow is required"))?;
            let new_email_pow = request
                .new_email_pow
                .ok_or_else(|| LogicError::bad_request("new_email_pow is required"))?;
            let (old_email_subject, new_email_subject) =
                handle_email_update_send(state, &user_id, &old_email_pow, &new_email_pow).await?;
            Ok(serde_json::json!({
                "old_email_subject": old_email_subject,
                "new_email_subject": new_email_subject,
            }))
        }
        EmailReadIntent::Deregister => {
            let user_id = require_session_user(state, session_token)?;
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required"))?;
            let email_subject = handle_deregister_request(state, &user_id, &pow).await?;
            Ok(serde_json::json!({ "email_subject": email_subject }))
        }
    }
}

fn require_session_user(
    state: &AppState,
    session_token: Option<String>,
) -> Result<String, LogicError> {
    let token =
        session_token.ok_or_else(|| LogicError::unauthorized("missing session-token header"))?;
    authenticate_session(state, &token)
}

async fn handle_email_auth_request(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    let email = normalize_email(&pow.payload);
    if !validate_email(&email, &state.config.email.allowed_domains) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }
    verify_issued_pow(state, pow)?;

    let token = Uuid::now_v7().to_string();
    let email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &email, &email_subject, &token).await?;

    let email_address_hash = nail_common::hash::email(&email);
    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash auth token: {error}")))?;
    state.caches.authenticate.insert(
        &key,
        AuthenticateTokenEntry {
            email_address_hash: email_address_hash.clone(),
            email_subject: email_subject.clone(),
        },
    );
    tracing::info!(email_hash = %email_address_hash, "auth email sent");
    Ok(email_subject)
}

pub async fn handle_email_update_send(
    state: &AppState,
    user_id: &str,
    old_email_pow: &Pow,
    new_email_pow: &Pow,
) -> Result<(String, String), LogicError> {
    verify_issued_pow(state, old_email_pow)?;
    verify_issued_pow(state, new_email_pow)?;

    let old_email = normalize_email(&old_email_pow.payload);
    let new_email = normalize_email(&new_email_pow.payload);
    if old_email == new_email {
        return Err(LogicError::bad_request(
            "new email must be different from old email",
        ));
    }

    let user_entry = read_user(&state.graph, user_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    let old_email_address_hash = nail_common::hash::email(&old_email);
    if user_entry.email_address_hash != old_email_address_hash {
        return Err(LogicError::bad_request(
            "old email does not match your current email",
        ));
    }

    let allowed_domains = &state.config.email.allowed_domains;
    if !validate_email(&old_email, allowed_domains) || !validate_email(&new_email, allowed_domains) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }

    let new_email_address_hash = nail_common::hash::email(&new_email);
    if let Some(existing_user_id) =
        find_user_by_email_address_hash(&state.graph, &new_email_address_hash)
            .await
            .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();

    let old_email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &old_email, &old_email_subject, &old_token).await?;
    let new_email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &new_email, &new_email_subject, &new_token).await?;

    let token_from_old_email_hash = token_key(&old_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let token_from_new_email_hash = token_key(&new_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    state.caches.email_update.insert(
        user_id,
        EmailUpdateTokenEntry {
            old_email_address_hash: old_email_address_hash.clone(),
            new_email_address_hash: new_email_address_hash.clone(),
            token_from_old_email_hash,
            token_from_new_email_hash,
        },
    );

    tracing::info!(
        old_hash = %old_email_address_hash,
        new_hash = %new_email_address_hash,
        "email update requested"
    );
    Ok((old_email_subject, new_email_subject))
}

pub async fn handle_email_update_confirm(
    state: &AppState,
    user_id: &str,
    pow: &Pow,
    raw_old_email_token: &str,
    raw_new_email_token: &str,
) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;

    let raw_old = raw_old_email_token.trim();
    let raw_new = raw_new_email_token.trim();
    let old_email_token = normalize_token(raw_old)
        .ok_or_else(|| LogicError::bad_request("invalid old email token"))?;
    let new_email_token = normalize_token(raw_new)
        .ok_or_else(|| LogicError::bad_request("invalid new email token"))?;

    let expected_canonical = format!("{old_email_token}\n{new_email_token}");
    let expected_raw = format!("{raw_old}\n{raw_new}");
    if pow.payload != expected_canonical && pow.payload != expected_raw {
        return Err(LogicError::bad_request("PoW payload does not match token"));
    }
    if old_email_token == new_email_token {
        return Err(LogicError::bad_request(
            "old token and new token must be different",
        ));
    }

    let entry = state
        .caches
        .email_update
        .read(user_id)
        .ok_or_else(|| LogicError::bad_request("invalid or expired email update request"))?;

    let old_token_hash = token_key(&old_email_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let new_token_hash = token_key(&new_email_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    if entry.token_from_old_email_hash != old_token_hash
        || entry.token_from_new_email_hash != new_token_hash
    {
        return Err(LogicError::bad_request("token mismatch"));
    }

    let old_email_address_hash = entry.old_email_address_hash;
    let new_email_address_hash = entry.new_email_address_hash;
    if let Some(existing_user_id) =
        find_user_by_email_address_hash(&state.graph, &new_email_address_hash)
            .await
            .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    update_user_email(
        &state.graph,
        user_id,
        &old_email_address_hash,
        &new_email_address_hash,
    )
    .await
    .map_err(|error| match error {
        UserWriteError::AlreadyTaken => {
            LogicError::bad_request("new email is already used by another account")
        }
        UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
        UserWriteError::EmailMismatch => LogicError::bad_request("email has already been changed"),
        UserWriteError::Db(error) => LogicError::internal(format!("failed to update email: {error}")),
    })?;

    state.caches.email_update.consume_if(user_id, |current| {
        current.token_from_old_email_hash == old_token_hash
            && current.token_from_new_email_hash == new_token_hash
    });
    state.caches.session.delete_by_reverse_key(user_id);
    state
        .caches
        .authenticate
        .delete_by_reverse_key(&old_email_address_hash);
    state.caches.deregister.delete_by_reverse_key(user_id);

    let new_session_token = Uuid::now_v7().to_string();
    let session_key = token_key(&new_session_token)
        .map_err(|error| LogicError::internal(format!("failed to hash session token: {error}")))?;
    state.caches.session.insert(
        &session_key,
        SessionTokenEntry {
            user_id: user_id.to_string(),
        },
    );

    tracing::info!(user_id = %user_id, "email updated");
    Ok(new_session_token)
}

pub async fn handle_deregister_request(
    state: &AppState,
    user_id: &str,
    pow: &Pow,
) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;

    let email = normalize_email(&pow.payload);
    let user_entry = read_user(&state.graph, user_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    if user_entry.email_address_hash != nail_common::hash::email(&email) {
        return Err(LogicError::bad_request("email does not match your account"));
    }

    let token = Uuid::now_v7().to_string();
    let email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &email, &email_subject, &token).await?;

    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash deregister token: {error}")))?;
    state.caches.deregister.insert(
        &key,
        DeregisterTokenEntry {
            user_id: user_id.to_string(),
            email_address_hash: user_entry.email_address_hash,
        },
    );

    tracing::info!(user_id = %user_id, "deregister confirmation email sent");
    Ok(email_subject)
}

async fn send_confirmation_email(
    state: &AppState,
    email: &str,
    subject: &str,
    token: &str,
) -> Result<(), LogicError> {
    match state.email.send_email(email, subject, token).await {
        Ok(()) => Ok(()),
        Err(SendEmailError::RateLimited) => Err(LogicError::bad_request(
            "email already sent recently, check your inbox",
        )),
        Err(SendEmailError::Transport(error)) => {
            tracing::warn!(target: "email", error = %error, "failed to send email");
            Err(LogicError::internal("failed to send email"))
        }
    }
}
