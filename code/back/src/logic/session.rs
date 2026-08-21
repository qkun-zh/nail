use cache::UserId;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity};
use crate::logic::error::LogicError;

pub fn normalize_token(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    Uuid::parse_str(&cleaned).ok().map(|uuid| uuid.to_string())
}

pub fn cache_key(token: &str) -> Result<String, LogicError> {
    nail_common::hash::hash(token.as_bytes())
        .map_err(|error| LogicError::internal(format!("failed to hash token: {error}")))
}

pub fn hash_canonical_token(token: &str) -> Result<String, LogicError> {
    cache_key(token)
}

pub fn hash_token(raw: &str, invalid: LogicError) -> Result<String, LogicError> {
    let token = normalize_token(raw).ok_or(invalid)?;
    hash_canonical_token(&token)
}

pub fn read_session(state: &AppState, raw_token: &str) -> Result<String, LogicError> {
    let key = hash_token(raw_token, LogicError::unauthorized("invalid session"))?;
    state
        .cache
        .session
        .read(&key)
        .map(|entry| entry.as_str().to_string())
        .ok_or_else(|| LogicError::unauthorized("invalid session"))
}

pub fn create_session(state: &AppState, user_id: &str) -> Result<String, LogicError> {
    let session_token = Uuid::now_v7().to_string();
    let session_key = cache_key(&session_token)?;
    let user_id = UserId::new(user_id.to_string())
        .map_err(|error| LogicError::internal(format!("invalid user id: {error}")))?;
    state.cache.session.insert(&session_key, user_id);
    Ok(session_token)
}

pub fn read_user_name(state: &AppState, session_token: &str) -> Result<String, LogicError> {
    let user_id = read_session(state, session_token)?;
    authorize_entity(
        state,
        &user_id,
        crate::repository::role::PERMISSION_USER_READ,
        EntityRef::User(&user_id),
    )?;
    let entry = crate::repository::user::read_user(&state.database, &user_id)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    Ok(entry.name)
}

pub fn delete_session(state: &AppState, session_token: &str) -> Result<(), LogicError> {
    let user_id = read_session(state, session_token)?;
    let key = hash_token(session_token, LogicError::unauthorized("invalid session"))?;
    let _ = state.cache.session.delete(&key);
    tracing::info!(user_id = %user_id, "session deleted");
    Ok(())
}
