use common::pow::{Pow, verify};

use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;

pub fn verify_issued_pow(state: &AppState, pow: &Pow) -> Result<(), LogicError> {
    let challenge_id = pow.challenge.id.to_string();
    if state.cache.challenge.delete(&challenge_id).is_none() {
        tracing::warn!(
            challenge_id = %challenge_id,
            "challenge not issued, expired, or already used"
        );
        return Err(LogicError::bad_request(
            "challenge not issued, expired, or already used",
        ));
    }
    if !verify(pow, state.config.server.pow_difficulty_iterations) {
        tracing::warn!(challenge_id = %challenge_id, "PoW verification failed");
        return Err(LogicError::bad_request("PoW verification failed"));
    }
    Ok(())
}
