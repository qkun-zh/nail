use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use cache::UserId;

use super::context::TestCtx;
use crate::logic::session::cache_key;

fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    let token = Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));
    (user_id, token)
}

fn admin_session(context: &TestCtx) -> (String, String) {
    session_for(context, "user-zero@example.com")
}

#[tokio::test]
async fn user_read_self_returns_name_and_optional_email_hash() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .get(&format!("/users/{user_id}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let default_name = user_id.replace('-', "");
    assert_eq!(body["data"]["name"].as_str(), Some(default_name.as_str()));
    assert!(body["data"].get("email_hash").is_none());

    let (status, body) = context
        .get(&format!("/users/{user_id}?email_hash=true"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["data"]["email_hash"].as_str(),
        Some(
            common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed")
                .as_str()
        )
    );
}

#[tokio::test]
async fn user_read_other_by_member_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = session_for(&context, "alice@example.com");
    let (target, _) = session_for(&context, "bob@example.com");

    let (status, body) = context.get(&format!("/users/{target}"), Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn user_read_other_by_admin_returns_profile() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, admin_token) = admin_session(&context);
    let (target, _) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .get(
            &format!("/users/{target}?email_hash=true"),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["id"].as_str(), Some(target.as_str()));
    assert_eq!(
        body["data"]["email_hash"].as_str(),
        Some(
            common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed")
                .as_str()
        )
    );
    let _ = admin;
}

#[tokio::test]
async fn user_read_self_after_hard_delete_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com");
    crate::repository::delete::delete_user(&context.state.database, &user_id).expect("hard delete");

    let (status, body) = context
        .get(&format!("/users/{user_id}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("user not found"));
}

#[tokio::test]
async fn user_update_self_rename_via_name() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .patch(
            &format!("/users/{user_id}"),
            json!({ "name": "alice-renamed" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("alice-renamed"));
}

#[tokio::test]
async fn user_update_admin_rename() {
    let context = TestCtx::new().await.expect("test context");
    let (_, admin_token) = admin_session(&context);
    let (target, _) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .patch(
            &format!("/users/{target}"),
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
    let (_, admin_token) = admin_session(&context);
    let (target, _) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .delete(&format!("/users/{target}?mode=hard"), Some(&admin_token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["user_id"].as_str(), Some(target.as_str()));
    assert_eq!(body["message"].as_str(), Some("deleted"));
}

#[tokio::test]
async fn user_delete_rejects_a_missing_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com");
    let (status, body) = context
        .delete(&format!("/users/{user_id}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("missing or unsupported delete mode (expected \"transfer\", \"soft\" or \"hard\")")
    );
}

#[tokio::test]
async fn user_delete_transfer_after_email_confirmation() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .post(
            "/tokens",
            json!({ "purpose": "delete_user", "email": "alice@example.com" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let confirmation_token = &messages[0].2;

    let (status, body) = context
        .delete(
            &format!("/users/{user_id}?mode=transfer&token={confirmation_token}"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));

    assert!(
        crate::repository::user::read_user(&context.state.database, &user_id)
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn email_change_two_step_flow_updates_email_and_rotates_session() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, old_session) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .post(
            "/tokens",
            json!({
                "purpose": "update_user_email",
                "old_email": "alice@example.com",
                "new_email": "alice-new@example.com",
            }),
            Some(&old_session),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(
        !body["data"]["old_email_subject"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
    assert!(
        !body["data"]["new_email_subject"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );

    let messages = context.emails();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, "alice@example.com");
    assert_eq!(messages[1].0, "alice-new@example.com");
    let old_token = messages[0].2.clone();
    let new_token = messages[1].2.clone();
    assert!(context.state.cache.email_update.read(&user_id).is_some());

    let (status, body) = context
        .patch(
            &format!("/users/{user_id}"),
            json!({
                "old_email_token": old_token,
                "new_email_token": new_token,
            }),
            Some(&old_session),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(
        !body["data"]["session_token"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );

    let entry = crate::repository::user::read_user(&context.state.database, &user_id)
        .expect("read")
        .expect("entry");
    assert_eq!(
        entry.email_address_hash,
        common::hash::hash("alice-new@example.com".as_bytes()).expect("hash must succeed")
    );

    let (status, _) = context.get("/user?id=true", Some(&old_session)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_change_rejects_same_old_and_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session) = session_for(&context, "alice@example.com");

    let (status, body) = context
        .post(
            "/tokens",
            json!({
                "purpose": "update_user_email",
                "old_email": "alice@example.com",
                "new_email": "alice@example.com",
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
    let (_, session) = session_for(&context, "alice@example.com");
    session_for(&context, "bob@example.com");

    let (status, body) = context
        .post(
            "/tokens",
            json!({
                "purpose": "update_user_email",
                "old_email": "alice@example.com",
                "new_email": "bob@example.com",
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
async fn email_change_rejects_an_invalid_old_token() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session) = session_for(&context, "alice@example.com");

    let (status, _) = context
        .post(
            "/tokens",
            json!({
                "purpose": "update_user_email",
                "old_email": "alice@example.com",
                "new_email": "alice-new@example.com",
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let messages = context.emails();
    let _old_token = messages[0].2.clone();
    let new_token = messages[1].2.clone();

    let (status, body) = context
        .patch(
            &format!("/users/{user_id}"),
            json!({
                "old_email_token": "not-a-uuid",
                "new_email_token": new_token,
            }),
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("invalid old email token"));
}
