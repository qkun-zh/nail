use nail_common::pow::Pow;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity};
use crate::logic::error::{LogicError, database_error};
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::SessionTokenEntry;

pub fn normalize_token(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    Uuid::parse_str(&cleaned).ok().map(|uuid| uuid.to_string())
}

pub fn hash_canonical_token(token: &str) -> Result<String, LogicError> {
    crate::repository::cache::token_key(token)
        .map_err(|error| LogicError::internal(format!("failed to hash token: {error}")))
}

pub fn hash_token(raw: &str, invalid: LogicError) -> Result<String, LogicError> {
    let token = normalize_token(raw).ok_or(invalid)?;
    hash_canonical_token(&token)
}

pub fn read_session(state: &AppState, raw_token: &str) -> Result<String, LogicError> {
    let key = hash_token(raw_token, LogicError::unauthorized("invalid session"))?;
    state
        .caches
        .session
        .read(&key)
        .map(|entry| entry.user_id)
        .ok_or_else(|| LogicError::unauthorized("invalid session"))
}

pub fn create_session(state: &AppState, user_id: &str) -> Result<String, LogicError> {
    let session_token = Uuid::now_v7().to_string();
    let session_key = hash_canonical_token(&session_token)?;
    state.caches.session.insert(
        &session_key,
        SessionTokenEntry {
            user_id: user_id.to_string(),
        },
    );
    Ok(session_token)
}

pub async fn read_user_name(state: &AppState, session_token: &str) -> Result<String, LogicError> {
    let user_id = read_session(state, session_token)?;
    authorize_entity(
        state,
        &user_id,
        crate::repository::role::PERMISSION_USER_READ,
        EntityRef::User(&user_id),
    )
    .await?;
    let entry = crate::repository::user::read_user(&state.graph, &user_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    Ok(entry.name)
}

pub fn delete_session(state: &AppState, pow: &Pow, session_token: &str) -> Result<(), LogicError> {
    let user_id = read_session(state, session_token)?;
    verify_issued_pow(state, pow)?;
    let key = hash_token(session_token, LogicError::unauthorized("invalid session"))?;
    state.caches.session.delete(&key);
    tracing::info!(user_id = %user_id, "session deleted");
    Ok(())
}
