use nail_common::pow::Pow;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;

pub fn verify_issued_pow(state: &AppState, pow: &Pow) -> Result<(), ApiError> {
    crate::logic::pow::verify_issued_pow(state, pow).map_err(ApiError::from)
}
