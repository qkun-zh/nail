use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;
use crate::logic::session::read_session;

pub const SESSION_TOKEN_HEADER: &str = "session-token";

#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: String,
    pub token: String,
}

impl FromRequestParts<AppState> for Principal {
    type Rejection = ApiError;

    #[allow(unknown_lints)]
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(SESSION_TOKEN_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| ApiError::unauthorized("missing session-token header"))?;
        let user_id = read_session(state, &token).map_err(ApiError::from_logic)?;
        Ok(Self { user_id, token })
    }
}
