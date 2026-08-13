use axum::http::StatusCode;
use nail_common::pow::{Challenge, Pow, ProveInput, prove};
use serde_json::json;
use uuid::Uuid;

use super::context::TestCtx;

async fn issue_and_prove(context: &TestCtx, payload: &str) -> Pow {
    let (status, body) = context.get("/challenge/read", None).await;
    assert_eq!(status, StatusCode::OK, "challenge body: {body}");
    let id = body["data"]["id"].as_str().expect("challenge id").to_string();
    let difficulty = body["data"]["difficulty"].as_u64().expect("difficulty");
    let challenge = Challenge {
        id: Uuid::parse_str(&id).expect("uuid challenge id"),
        difficulty,
    };
    prove(ProveInput {
        challenge,
        payload: payload.to_string(),
    })
    .expect("proof generation must succeed")
}

#[tokio::test]
async fn full_authentication_lifecycle() {
    let context = TestCtx::new().await.expect("test context");

    let pow = issue_and_prove(&context, "alice@example.com").await;
    let (status, body) = context
        .post(
            "/email/read?intent=authenticate",
            json!({ "pow": pow }),
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

    let token_pow = issue_and_prove(&context, token).await;
    let (status, body) = context
        .post("/user/create", json!({ "pow": token_pow }), None)
        .await;
    assert_eq!(status, StatusCode::OK, "create body: {body}");
    let session_token = body["data"]["session_token"]
        .as_str()
        .expect("session token")
        .to_string();

    let (status, body) = context
        .get("/session/read?id=true", Some(&session_token))
        .await;
    assert_eq!(status, StatusCode::OK, "session body: {body}");
    assert!(!body["data"]["id"].as_str().unwrap_or("").is_empty());

    let (status, _) = context.get("/session/read?id=true", Some("not-a-uuid")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let logout_pow = issue_and_prove(&context, "logout-nonce").await;
    let (status, body) = context
        .post(
            "/session/delete",
            json!({ "pow": logout_pow }),
            Some(&session_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "logout body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));

    let (status, _) = context.get("/session/read?id=true", Some(&session_token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_read_with_missing_or_invalid_intent_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let (status, _) = context.post("/email/read", json!({}), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let pow = issue_and_prove(&context, "alice@example.com").await;
    let (status, _) = context
        .post("/email/read?intent=bogus", json!({ "pow": pow }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn email_read_rejects_a_disallowed_domain() {
    let context = TestCtx::new().await.expect("test context");
    let pow = issue_and_prove(&context, "alice@other.org").await;
    let (status, body) = context
        .post("/email/read?intent=authenticate", json!({ "pow": pow }), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("email domain not allowed"));
    assert!(context.emails().is_empty());
}

#[tokio::test]
async fn session_read_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context.get("/session/read", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("missing session-token header"));
}
