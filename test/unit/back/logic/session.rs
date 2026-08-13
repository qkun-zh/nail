use super::context::TestCtx;
use crate::repository::cache::{SessionTokenEntry, token_key};

#[tokio::test]
async fn logout_removes_the_session_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-123".to_string(),
        },
    );

    let pow = context.issued_pow("logout-nonce");
    crate::logic::session::handle_logout(&context.state, &pow, &token)
        .await
        .expect("logout");
    assert!(context.state.caches.session.read(&key).is_none());
}

#[tokio::test]
async fn logout_requires_a_valid_session() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("logout-nonce");
    let error = crate::logic::session::handle_logout(&context.state, &pow, "not-a-uuid")
        .await
        .unwrap_err();
    assert_eq!(
        error,
        crate::logic::error::LogicError::unauthorized("invalid session")
    );
}

#[tokio::test]
async fn read_user_name_returns_the_account_name() {
    let context = TestCtx::new().await.expect("test context");
    let email_hash = nail_common::hash::email("alice@example.com");
    let user_id = crate::repository::user::find_or_create_user(&context.state.graph, &email_hash)
        .await
        .expect("create user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );

    let name = crate::logic::session::read_user_name(&context.state, &token)
        .await
        .expect("name");
    assert_eq!(name, user_id.replace('-', ""));
}
