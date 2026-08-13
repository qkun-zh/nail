use axum::Json;
use axum::extract::{Query, State};
use nail_common::request::LogoutRequest;
use nail_common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;
use crate::interface::principal::Principal;

#[derive(Debug, Default, Deserialize)]
pub struct SessionReadParams {
    pub id: Option<bool>,
    pub name: Option<bool>,
}

pub async fn read(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<SessionReadParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let mut data = serde_json::Map::new();
    if params.id.unwrap_or(false) {
        data.insert("id".to_string(), serde_json::json!(principal.user_id));
    }
    if params.name.unwrap_or(false) {
        let name = crate::logic::session::read_user_name(&state, &principal.token).await?;
        data.insert("name".to_string(), serde_json::json!(name));
    }
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::Value::Object(data),
        "ok",
    )))
}

pub async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    crate::logic::session::handle_logout(&state, &payload.pow, &principal.token).await?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({}),
        "deleted",
    )))
}
