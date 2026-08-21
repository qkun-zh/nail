use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::response::EmptyView;
use nail_common::response::session::SessionView;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::AppQuery;
use crate::interface::principal::Principal;

#[derive(Debug, Default, Deserialize)]
pub struct SessionReadParams {
    pub id: Option<bool>,
    pub name: Option<bool>,
}

pub async fn read_session(
    State(state): State<AppState>,
    principal: Principal,
    AppQuery(params): AppQuery<SessionReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let mut view = SessionView::default();
    if params.id.unwrap_or(false) {
        view.id = Some(principal.user_id);
    }
    if params.name.unwrap_or(false) {
        view.name = Some(
            crate::logic::session::read_user_name(&state, &principal.token)
                .map_err(ApiError::from_logic)?,
        );
    }
    Ok(json_response(StatusCode::OK, view, "ok"))
}

pub async fn delete_session(
    State(state): State<AppState>,
    principal: Principal,
) -> Result<impl IntoResponse, ApiError> {
    crate::logic::session::delete_session(&state, &principal.token)
        .map_err(ApiError::from_logic)?;
    Ok(json_response(StatusCode::OK, EmptyView {}, "deleted"))
}
