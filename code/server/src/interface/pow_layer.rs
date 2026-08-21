use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use common::pow::Pow;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;
use crate::logic::pow::verify_issued_pow;

pub const X_POW_HEADER: &str = "x-pow";

pub async fn require_pow(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() == Method::POST && request.uri().path() == "/challenges" {
        return Ok(next.run(request).await);
    }
    let header = request
        .headers()
        .get(X_POW_HEADER)
        .ok_or_else(|| ApiError::bad_request("missing x-pow header"))?;
    let pow: Pow = serde_json::from_slice(header.as_bytes())
        .map_err(|_| ApiError::bad_request("invalid x-pow header"))?;
    verify_issued_pow(&state, &pow).map_err(ApiError::from_logic)?;
    Ok(next.run(request).await)
}
