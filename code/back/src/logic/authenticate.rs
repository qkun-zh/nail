use nail_common::pow::Pow;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::{SessionTokenEntry, token_key};

pub fn normalize_token(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|character| !character.is_whitespace()).collect();
    Uuid::parse_str(&cleaned).ok().map(|uuid| uuid.to_string())
}

pub fn authenticate_session(state: &AppState, raw_token: &str) -> Result<String, LogicError> {
    let token = normalize_token(raw_token)
        .ok_or_else(|| LogicError::unauthorized("invalid session"))?;
    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash session token: {error}")))?;
    state
        .caches
        .session
        .read(&key)
        .map(|entry| entry.user_id)
        .ok_or_else(|| {
            tracing::warn!(session_hash = %key, "invalid or expired session");
            LogicError::unauthorized("invalid session")
        })
}

pub async fn handle_token_exchange(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;
    let token =
        normalize_token(&pow.payload).ok_or_else(|| LogicError::bad_request("invalid or expired token"))?;

    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let entry = state
        .caches
        .authenticate
        .consume(&key)
        .ok_or_else(|| {
            tracing::warn!(token_hash = %key, "invalid or expired email token");
            LogicError::bad_request("invalid or expired token")
        })?;

    let user_id = match crate::repository::user::find_or_create_user(
        &state.graph,
        &entry.email_address_hash,
    )
    .await
    {
        Ok(user_id) => user_id,
        Err(error) => {
            state.caches.authenticate.insert(&key, entry);
            return Err(LogicError::internal(format!("database query failed: {error}")));
        }
    };

    crate::repository::role::hold_role(&state.graph, &user_id, crate::repository::role::ROLE_MEMBER)
        .await
        .map_err(|error| LogicError::internal(format!("failed to grant member role: {error}")))?;

    let session_token = Uuid::now_v7().to_string();
    let session_key = token_key(&session_token).map_err(|error| {
        LogicError::internal(format!("failed to hash session token: {error}"))
    })?;
    state.caches.session.insert(
        &session_key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );

    tracing::info!(user_id = %user_id, "session created after email token exchange");
    Ok(session_token)
}
