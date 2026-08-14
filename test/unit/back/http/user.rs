use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::TestCtx;
use crate::repository::cache::{SessionTokenEntry, token_key};

async fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    let token = Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );
    (user_id, token)
}

async fn admin_session(context: &TestCtx) -> (String, String) {
    session_for(context, "user-zero@example.com").await
}

#[tokio::test]
async fn user_read_self_returns_name_and_optional_email_hash() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .get(&format!("/user/{user_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some(user_id.replace('-', "").as_str()));
    assert!(body["data"].get("email_hash").is_none());

    let (status, body) = context
        .get(&format!("/user/{user_id}/read?email_hash=true"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["data"]["email_hash"].as_str(),
        Some(nail_common::hash::email("alice@example.com").as_str())
    );
}

#[tokio::test]
async fn user_read_other_by_member_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = session_for(&context, "alice@example.com").await;
    let (target, _) = session_for(&context, "bob@example.com").await;

    let (status, body) = context.get(&format!("/user/{target}/read"), Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn user_read_other_by_admin_returns_profile() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, admin_token) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .get(&format!("/user/{target}/read?email_hash=true"), Some(&admin_token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["id"].as_str(), Some(target.as_str()));
    assert_eq!(
        body["data"]["email_hash"].as_str(),
        Some(nail_common::hash::email("alice@example.com").as_str())
    );
    let _ = admin;
}

#[tokio::test]
async fn user_list_by_member_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = session_for(&context, "alice@example.com").await;
    let (status, body) = context.get("/user/read", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn user_list_by_admin_returns_paginated_users() {
    let context = TestCtx::new().await.expect("test context");
    let (_, admin_token) = admin_session(&context).await;
    session_for(&context, "alice@example.com").await;
    session_for(&context, "bob@example.com").await;

    let (status, body) = context.get("/user/read?page=1&limit=2", Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["total"].as_u64(), Some(3));
    assert_eq!(body["data"]["user_list"].as_array().map(Vec::len), Some(2));
    assert_eq!(body["data"]["has_next"].as_bool(), Some(true));
}

#[tokio::test]
async fn user_update_self_rename_via_pow() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;
    let pow = context.issued_pow("alice-renamed");

    let (status, body) = context
        .post(
            &format!("/user/{user_id}/update"),
            json!({ "pow": pow }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("alice-renamed"));
}

#[tokio::test]
async fn user_update_admin_rename() {
    let context = TestCtx::new().await.expect("test context");
    let (_, admin_token) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            &format!("/user/{target}/update"),
            json!({ "name": "alice-by-admin" }),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("alice-by-admin"));
}

#[tokio::test]
async fn user_delete_hard_by_admin() {
    let context = TestCtx::new().await.expect("test context");
    let (_, admin_token) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;

    let pow = context.issued_pow("ignored");
    let (status, body) = context
        .post(
            &format!("/user/{target}/delete"),
            json!({ "mode": "hard", "pow": pow }),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["user_id"].as_str(), Some(target.as_str()));
    assert_eq!(body["message"].as_str(), Some("deleted"));
}

#[tokio::test]
async fn user_delete_rejects_a_missing_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;
    let pow = context.issued_pow("ignored");
    let (status, body) = context
        .post(&format!("/user/{user_id}/delete"), json!({ "pow": pow }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("missing or unsupported delete mode (expected \"transfer\" or \"hard\")")
    );
}

#[tokio::test]
async fn user_delete_transfer_after_email_confirmation() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let pow = context.issued_pow("alice@example.com");
    let (status, body) = context
        .post(
            "/email/read?intent=deregister",
            json!({ "pow": pow }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let confirmation_token = &messages[0].2;

    let confirm_pow = context.issued_pow(confirmation_token);
    let (status, body) = context
        .post(
            &format!("/user/{user_id}/delete"),
            json!({ "mode": "transfer", "pow": confirm_pow }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));

    assert!(crate::repository::user::read_user(&context.state.graph, &user_id)
        .await
        .expect("read")
        .is_none());
}

#[tokio::test]
async fn email_change_two_step_flow_updates_email_and_rotates_session() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, old_session) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            "/email/read?intent=change_email",
            json!({
                "old_email_pow": context.issued_pow("alice@example.com"),
                "new_email_pow": context.issued_pow("alice-new@example.com"),
            }),
            Some(&old_session),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(!body["data"]["old_email_subject"].as_str().unwrap_or("").is_empty());
    assert!(!body["data"]["new_email_subject"].as_str().unwrap_or("").is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, "alice@example.com");
    assert_eq!(messages[1].0, "alice-new@example.com");
    let old_token = messages[0].2.clone();
    let new_token = messages[1].2.clone();
    assert!(context.state.caches.email_update.read(&user_id).is_some());

    let payload = format!("{old_token}\n{new_token}");
    let (status, body) = context
        .post(
            &format!("/user/{user_id}/update"),
            json!({
                "pow": context.issued_pow(&payload),
                "old_email_token": old_token,
                "new_email_token": new_token,
            }),
            Some(&old_session),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(!body["data"]["session_token"].as_str().unwrap_or("").is_empty());

    let entry = crate::repository::user::read_user(&context.state.graph, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(
        entry.email_address_hash,
        nail_common::hash::email("alice-new@example.com")
    );

    let (status, _) = context.get("/session/read?id=true", Some(&old_session)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_change_rejects_same_old_and_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            "/email/read?intent=change_email",
            json!({
                "old_email_pow": context.issued_pow("alice@example.com"),
                "new_email_pow": context.issued_pow("alice@example.com"),
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("new email must be different from old email")
    );
}

#[tokio::test]
async fn email_change_rejects_a_taken_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session) = session_for(&context, "alice@example.com").await;
    session_for(&context, "bob@example.com").await;

    let (status, body) = context
        .post(
            "/email/read?intent=change_email",
            json!({
                "old_email_pow": context.issued_pow("alice@example.com"),
                "new_email_pow": context.issued_pow("bob@example.com"),
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("new email is already used by another account")
    );
}

#[tokio::test]
async fn email_change_rejects_a_pow_payload_mismatch() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session) = session_for(&context, "alice@example.com").await;

    let (status, _) = context
        .post(
            "/email/read?intent=change_email",
            json!({
                "old_email_pow": context.issued_pow("alice@example.com"),
                "new_email_pow": context.issued_pow("alice-new@example.com"),
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let messages = context.emails();
    let old_token = messages[0].2.clone();
    let new_token = messages[1].2.clone();

    let (status, body) = context
        .post(
            &format!("/user/{user_id}/update"),
            json!({
                "pow": context.issued_pow("does-not-match"),
                "old_email_token": old_token,
                "new_email_token": new_token,
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("PoW payload does not match token"));
}

