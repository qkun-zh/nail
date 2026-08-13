use super::context::TestCtx;
use crate::logic::authenticate::{authenticate_session, normalize_token};
use crate::logic::error::LogicError;
use crate::repository::cache::{AuthenticateTokenEntry, SessionTokenEntry, token_key};

#[test]
fn normalize_token_strips_whitespace_and_requires_a_uuid() {
    let uuid = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        normalize_token(&format!(" {uuid}\n")).as_deref(),
        Some(uuid.as_str())
    );
    assert_eq!(normalize_token("not-a-uuid"), None);
    assert_eq!(normalize_token(""), None);
}

#[tokio::test]
async fn authenticate_session_returns_the_user_id_for_a_known_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-123".to_string(),
        },
    );
    assert_eq!(
        authenticate_session(&context.state, &token).expect("session"),
        "user-123"
    );
}

#[tokio::test]
async fn authenticate_session_rejects_garbage_and_unknown_tokens() {
    let context = TestCtx::new().await.expect("test context");
    assert_eq!(
        authenticate_session(&context.state, "not-a-uuid").unwrap_err(),
        LogicError::unauthorized("invalid session")
    );
    let unknown = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        authenticate_session(&context.state, &unknown).unwrap_err(),
        LogicError::unauthorized("invalid session")
    );
}

#[tokio::test]
async fn token_exchange_creates_a_user_with_member_role_and_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let email_hash = nail_common::hash::email("alice@example.com");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.authenticate.insert(
        &key,
        AuthenticateTokenEntry {
            email_address_hash: email_hash.clone(),
            email_subject: uuid::Uuid::now_v7().to_string(),
        },
    );

    let pow = context.issued_pow(&token);
    let session_token = crate::logic::authenticate::handle_token_exchange(&context.state, &pow)
        .await
        .expect("exchange");

    let user_id = crate::repository::user::find_user_by_email_address_hash(
        &context.state.graph,
        &email_hash,
    )
    .await
    .expect("read user")
    .expect("user exists");
    let session_key = token_key(&session_token).expect("session key");
    let entry = context
        .state
        .caches
        .session
        .read(&session_key)
        .expect("session entry");
    assert_eq!(entry.user_id, user_id);

    let member_held = crate::repository::role::user_holds_role(
        &context.state.graph,
        &user_id,
        crate::repository::role::ROLE_MEMBER,
    )
    .await
    .expect("holds check");
    assert!(member_held);
}

#[tokio::test]
async fn token_exchange_rejects_an_invalid_token() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("not-a-uuid");
    assert_eq!(
        crate::logic::authenticate::handle_token_exchange(&context.state, &pow)
            .await
            .unwrap_err(),
        LogicError::bad_request("invalid or expired token")
    );
}

#[tokio::test]
async fn token_exchange_rejects_an_unknown_or_expired_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let pow = context.issued_pow(&token);
    assert_eq!(
        crate::logic::authenticate::handle_token_exchange(&context.state, &pow)
            .await
            .unwrap_err(),
        LogicError::bad_request("invalid or expired token")
    );
}
