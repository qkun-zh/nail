use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::TestCtx;
use crate::repository::cache::{SessionTokenEntry, token_key};

#[tokio::test]
async fn session_lifecycle_over_http() {
    let context = TestCtx::new().await.expect("test context");

    let (status, body) = context
        .post(
            "/tokens",
            json!({ "purpose": "create_user", "email": "alice@example.com" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "email body: {body}");
    let email_subject = body["data"]["email_subject"].as_str().expect("subject");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let (to, message_subject, token) = &messages[0];
    assert_eq!(to, "alice@example.com");
    assert_eq!(message_subject, email_subject);

    let (status, body) = context
        .post("/users", json!({ "token": token }), None)
        .await;
    assert_eq!(status, StatusCode::OK, "create body: {body}");
    let session_token = body["data"]["session_token"]
        .as_str()
        .expect("session token")
        .to_string();

    let (status, body) = context.get("/user?id=true", Some(&session_token)).await;
    assert_eq!(status, StatusCode::OK, "session body: {body}");
    assert!(!body["data"]["id"].as_str().unwrap_or("").is_empty());

    let (status, body) = context.get("/user?name=true", Some(&session_token)).await;
    assert_eq!(status, StatusCode::OK, "session name body: {body}");
    assert!(!body["data"]["name"].as_str().unwrap_or("").is_empty());

    let (status, _) = context.get("/user?id=true", Some("not-a-uuid")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = context.delete("/session", Some(&session_token)).await;
    assert_eq!(status, StatusCode::OK, "delete-session body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));

    let (status, _) = context.get("/user?id=true", Some(&session_token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn token_create_with_missing_or_invalid_purpose_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let (status, _) = context.post("/tokens", json!({}), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = context
        .post(
            "/tokens",
            json!({ "purpose": "bogus", "email": "alice@example.com" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_create_rejects_a_disallowed_domain() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context
        .post(
            "/tokens",
            json!({ "purpose": "create_user", "email": "alice@other.org" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("email domain not allowed"));
    assert!(context.emails().is_empty());
}

#[tokio::test]
async fn session_read_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context.get("/user", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("missing session-token header")
    );
}

#[tokio::test]
async fn user_create_rejects_an_unknown_token() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context
        .post(
            "/users",
            json!({ "token": Uuid::now_v7().to_string() }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("invalid or expired token"));
}

#[tokio::test]
async fn token_create_requires_an_email() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context
        .post("/tokens", json!({ "purpose": "create_user" }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("email is required"));
}

fn insert_session(context: &TestCtx) -> String {
    let token = Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.cache.session.insert(
        &key,
        SessionTokenEntry {
            user_id: Uuid::now_v7().to_string(),
        },
    );
    token
}

#[tokio::test]
async fn session_delete_returns_envelope() {
    let context = TestCtx::new().await.expect("test context");
    let token = insert_session(&context);

    let (status, body) = context.delete("/session", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["code"].as_u64(), Some(200));
}

#[tokio::test]
async fn session_read_with_malformed_query_returns_envelope() {
    let context = TestCtx::new().await.expect("test context");
    let token = insert_session(&context);

    let (status, body) = context.get("/user?id=not-a-boolean", Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["code"].as_u64(), Some(400));
    assert!(body["data"].is_null());
    assert_eq!(body["message"].as_str(), Some("invalid query parameters"));
}
