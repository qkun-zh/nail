
use common::hash;
use common::pow::Challenge;
use common::pow::{Pow, verify};
use email_address::{EmailAddress, Options};
use uuid::Uuid;

use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::other::conf::ServerConfig;
use crate::repo;

pub fn generate_challenge(config: &ServerConfig, caches: &repo::TokenCaches) -> Challenge {
    let id = Uuid::now_v7();
    repo::token::challenge::create_challenge(caches, &id.to_string());
    tracing::info!(
        challenge_id = %id,
        difficulty = config.pow_difficulty_iterations,
        "challenge issued"
    );
    Challenge {
        id,
        difficulty: config.pow_difficulty_iterations,
    }
}

pub fn verify_issued_pow(state: &AppState, pow: &Pow) -> Result<(), LogicError> {
    if !repo::token::challenge::consume_challenge(&state.cache, &pow.challenge.id.to_string()) {
        tracing::warn!(
            challenge_id = %pow.challenge.id,
            "challenge not issued, expired, or already used"
        );
        return Err(LogicError::bad_request(
            "challenge not issued, expired, or already used",
        ));
    }
    if !verify(pow, state.config.server.pow_difficulty_iterations) {
        tracing::warn!(challenge_id = %pow.challenge.id, "PoW verification failed");
        return Err(LogicError::bad_request("PoW verification failed"));
    }
    Ok(())
}

pub(crate) fn validate_email(email: &str, allowed_domains: &[String]) -> bool {
    if email.len() > 254 {
        return false;
    }
    let Ok(parsed) = EmailAddress::parse_with_options(email, Options::default()) else {
        return false;
    };
    let domain = parsed.domain().to_lowercase();
    allowed_domains
        .iter()
        .any(|d| d.as_str() == domain.as_str())
}

pub async fn handle_email_auth_request(
    state: &AppState,
    payload: Pow,
) -> Result<String, LogicError> {
    let email = normalize_email(&payload.payload);

    if !validate_email(&email, &state.config.email.allowed_domains) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }
    verify_issued_pow(state, &payload)?;

    let email_address_hash = hash::email(&email);

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
            tracing::warn!(target: "email", error = %e, "failed to send authenticate email");
            return Err(LogicError::internal(format!(
                "failed to send authenticate email: {e}"
            )));
        }
    }
    repo::token::authenticate::create_authenticate_token(
        &state.cache,
        &token,
        &email_address_hash,
        &email_subject,
    );

    tracing::info!(email_hash = %email_address_hash, "auth email sent");

    Ok(email_subject)
}

pub async fn handle_token_exchange(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;
    let token =
        normalize_token(&pow.payload).ok_or_else(|| LogicError::bad_request("invalid token"))?;

    let entry = repo::token::authenticate::consume_authenticate_token(&state.cache, &token)
        .ok_or_else(|| {
            tracing::warn!(token_hash = %hash::token(&token), "invalid or expired email token");
            LogicError::bad_request("invalid or expired token")
        })?;

    let user_id = match repo::user::find_or_create_user(&state.db, &entry.email_address_hash).await
    {
        Ok(user_id) => user_id,
        Err(e) => {
            repo::token::authenticate::create_authenticate_token(
                &state.cache,
                &token,
                &entry.email_address_hash,
                &entry.email_subject,
            );
            return Err(LogicError::internal(format!("database query failed: {e}")));
        }
    };

    crate::repo::authorization::hold_role(
        &state.db,
        &user_id,
        crate::repo::authorization::ROLE_MEMBER,
    )
    .await
    .map_err(|e| LogicError::internal(format!("failed to grant member role: {e}")))?;

    let session_token = Uuid::now_v7().to_string();
    repo::token::session::create_session_token(&state.cache, &session_token, &user_id);

    tracing::info!(user_id = %user_id, "session created after email token exchange");

    Ok(session_token)
}

pub fn authenticate_session(state: &AppState, raw_token: &str) -> Result<String, LogicError> {
    let token = normalize_token(raw_token)
        .ok_or_else(|| LogicError::bad_request("invalid session token"))?;

    repo::token::session::find_user_id_by_session_token(&state.cache, &token).ok_or_else(|| {
        tracing::warn!(session_hash = %hash::token(&token), "invalid or expired session");
        LogicError::unauthorized("invalid session")
    })
}

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub fn normalize_token(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    Uuid::parse_str(&cleaned).ok().map(|uuid| uuid.to_string())
}
