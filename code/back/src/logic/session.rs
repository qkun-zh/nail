use nail_common::pow::Pow;

use crate::infrastructure::state::AppState;
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::token_key;

pub async fn read_user_name(state: &AppState, session_token: &str) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    let entry = crate::repository::user::read_user(&state.graph, &user_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    Ok(entry.name)
}

pub async fn handle_logout(
    state: &AppState,
    pow: &Pow,
    session_token: &str,
) -> Result<(), LogicError> {
    let user_id = authenticate_session(state, session_token)?;
    verify_issued_pow(state, pow)?;
    let key = token_key(session_token)
        .map_err(|error| LogicError::internal(format!("failed to hash session token: {error}")))?;
    state.caches.session.delete(&key);
    tracing::info!(user_id = %user_id, "user logged out");
    Ok(())
}
