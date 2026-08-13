
use axum::http::HeaderMap;
use axum::{Json, extract::Query, extract::State};
use common::pow::Challenge;
use common::request::EmailReadRequest;
use common::request::TokenRequest;
use common::response::ResponseEnvelope;
use serde::Deserialize;

use crate::api::{ApiError, logic_err, require_session};
use crate::logic;
use crate::other::AppState;

pub async fn issue_challenge(
    State(state): State<AppState>,
) -> Json<ResponseEnvelope<Challenge>> {
    Json(ResponseEnvelope::ok(
        200,
        logic::authenticate::generate_challenge(&state.config.server, &state.cache),
        "ok",
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct SessionReadParams {
    id: Option<bool>,
    name: Option<bool>,
}

pub async fn verify_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SessionReadParams>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = require_session(&state, &headers)?;
    let mut data = serde_json::Map::new();
    if params.id.unwrap_or(false) {
        let user_id = logic::authenticate::authenticate_session(&state, &session_token)
            .map_err(logic_err)?;
        data.insert("id".to_string(), serde_json::json!(user_id));
    }
    if params.name.unwrap_or(false) {
        let name = logic::user::handle_read_name(&state, &session_token)
            .await
            .map_err(logic_err)?;
        data.insert("name".to_string(), serde_json::json!(name));
    }
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::Value::Object(data),
        "ok",
    )))
}

pub async fn email_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<EmailReadRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    match (payload.old_email_pow, payload.new_email_pow) {
        (Some(old_pow), Some(new_pow)) => {
            let session_token = require_session(&state, &headers)?;
            let (old_email_subject, new_email_subject) =
                logic::email::handle_email_update_send(&state, &old_pow, &new_pow, &session_token)
                    .await
                    .map_err(logic_err)?;
            return Ok(Json(ResponseEnvelope::ok(
                200,
                serde_json::json!({
                    "old_email_subject": old_email_subject,
                    "new_email_subject": new_email_subject,
                }),
                "ok",
            )));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(ResponseEnvelope::err(
                    400,
                    "old_email_pow and new_email_pow must both be provided",
                )),
            ));
        }
        (None, None) => {}
    }

    let pow = payload.pow.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::err(400, "pow is required")),
        )
    })?;

    if let Ok(session_token) = crate::api::get_session_token(&headers)
        .ok_or(())
        .and_then(|t| logic::authenticate::authenticate_session(&state, &t).map_err(|_| ()))
    {
        let email_subject =
            logic::user::handle_deregister_request(&state, &pow, &session_token)
                .await
                .map_err(logic_err)?;
        Ok(Json(ResponseEnvelope::ok(
            200,
            serde_json::json!({ "email_subject": email_subject }),
            "ok",
        )))
    } else {
        let email_subject = logic::authenticate::handle_email_auth_request(&state, pow)
            .await
            .map_err(logic_err)?;
        Ok(Json(ResponseEnvelope::ok(
            200,
            serde_json::json!({ "email_subject": email_subject }),
            "ok",
        )))
    }
}

pub async fn redeem_token(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<Json<ResponseEnvelope<serde_json::Value>>, ApiError> {
    let session_token = logic::authenticate::handle_token_exchange(&state, &payload.pow)
        .await
        .map_err(logic_err)?;
    Ok(Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({ "session_token": session_token }),
        "ok",
    )))
}
