// Probe 003 — axum `Path<String>` placeholder-key agnosticism + `Parts` extraction.
//
// Purpose: pin the claims behind Task VII-backend Stage D3 (`{role_id}` ->
//   `{id}` route-const rename) and Stage E (`Parts` in handler signature):
//   1) axum's `Path<String>` deserializes the single path parameter by VALUE,
//      ignoring the placeholder KEY (`{role_id}` vs `{id}` produce identical
//      extraction), so renaming the placeholder key is URL-value-preserving.
//   2) a handler may extract `axum::http::request::Parts` directly (used by
//      `token.rs` to read the optional `session-token` header).
//
// Source evidence: axum-0.8.9 `src/extract/path/de.rs` (`parse_single_value!`
//   and `deserialize_str` use `url_params[0]`, never the key); axum-core-0.5.6
//   `src/extract/request_parts.rs` line 141 `impl<S> FromRequestParts<S> for
//   Parts`.
//
// Acceptance question: "does `/role/{id}/read` match the literal URL
//   `/role/<value>/read` and extract the value through `AppPath<String>` (the
//   `{role_id}` route today), and can a handler take `Parts` and read a
//   header?" Expect: yes on both.
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use serde_json::json;
use tower::ServiceExt;

use crate::interface::envelope::{ApiError, json_response};
use crate::interface::extractor::AppPath;

async fn echo_path_id(AppPath(id): AppPath<String>) -> Result<axum::response::Response, ApiError> {
    Ok(json_response(StatusCode::OK, json!({ "id": id }), "ok"))
}

async fn echo_session_header(
    parts: axum::http::request::Parts,
) -> Result<axum::response::Response, ApiError> {
    let token = parts
        .headers
        .get("session-token")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    Ok(json_response(StatusCode::OK, json!({ "token": token }), "ok"))
}

#[tokio::test]
async fn probe_003_path_key_does_not_matter_for_single_string_extraction() {
    let app = Router::new().route("/role/{id}/read", get(echo_path_id));
    let request = Request::builder()
        .uri("/role/xyz-123/read")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("oneshot");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["id"].as_str(), Some("xyz-123"));
}

#[tokio::test]
async fn probe_003_handler_can_extract_parts_and_read_headers() {
    let app = Router::new().route("/echo", get(echo_session_header));
    let request = Request::builder()
        .uri("/echo")
        .header("session-token", "tok-abc")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("oneshot");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["token"].as_str(), Some("tok-abc"));
}