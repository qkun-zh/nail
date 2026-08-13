use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::request::LogoutRequest;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::{ApiError, json_response};
use crate::interface::principal::Principal;

#[derive(Debug, Default, Deserialize)]
pub struct SessionReadParams {
    pub id: Option<bool>,
    pub name: Option<bool>,
}

pub async fn read_session(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<SessionReadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let mut data = serde_json::Map::new();
    if params.id.unwrap_or(false) {
        data.insert("id".to_string(), serde_json::json!(principal.user_id));
    }
    if params.name.unwrap_or(false) {
        let name = crate::logic::session::read_user_name(&state, &principal.token).await?;
        data.insert("name".to_string(), serde_json::json!(name));
    }
    Ok(json_response::<serde_json::Value>(
        StatusCode::OK,
        serde_json::Value::Object(data),
        "ok",
    ))
}

pub async fn delete_session(
    State(state): State<AppState>,
    principal: Principal,
    Json(payload): Json<LogoutRequest>,
) -> Result<impl IntoResponse, ApiError> {
    crate::logic::session::delete_session(&state, &payload.pow, &principal.token).await?;
    Ok(json_response::<serde_json::Value>(
        StatusCode::OK,
        serde_json::json!({}),
        "deleted",
    ))
}
