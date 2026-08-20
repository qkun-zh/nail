use crate::logic::user::{UserDeleteView, UserUpdateView};
use nail_common::request::{UserDeleteQuery, UserUpdateRequest};

use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::repository::cache::{SessionTokenEntry, token_key};

async fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.cache.session.insert(
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
async fn read_user_self_returns_name_and_optional_email_hash() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let data = crate::logic::user::read_user(&context.state, &user_id, &user_id, true, false)
        .await
        .expect("read");
    assert_eq!(
        data.name.as_deref(),
        Some(user_id.replace('-', "").as_str())
    );
    assert!(data.email_hash.is_none());

    let data = crate::logic::user::read_user(&context.state, &user_id, &user_id, true, true)
        .await
        .expect("read");
    assert_eq!(
        data.email_hash.as_deref(),
        Some(
            nail_common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed")
                .as_str()
        )
    );
}

#[tokio::test]
async fn read_user_other_by_member_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let (actor, _) = session_for(&context, "alice@example.com").await;
    let (target, _) = session_for(&context, "bob@example.com").await;
    let error = crate::logic::user::read_user(&context.state, &actor, &target, true, false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn read_user_other_by_admin_returns_profile() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;
    let data = crate::logic::user::read_user(&context.state, &admin, &target, true, true)
        .await
        .expect("read");
    assert_eq!(data.id.as_deref(), Some(target.as_str()));
    assert_eq!(
        data.email_hash.as_deref(),
        Some(
            nail_common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed")
                .as_str()
        )
    );
}

#[tokio::test]
async fn update_user_self_rename_via_name() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let data = crate::logic::user::update_user(
        &context.state,
        &user_id,
        &user_id,
        UserUpdateRequest {
            name: Some("alice-renamed".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .expect("update");
    let UserUpdateView::Name(view) = data else {
        panic!("unexpected session token");
    };
    assert_eq!(view.name, "alice-renamed");
}

#[tokio::test]
async fn update_user_admin_rename() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;
    let data = crate::logic::user::update_user(
        &context.state,
        &admin,
        &target,
        UserUpdateRequest {
            name: Some("alice-by-admin".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .expect("update");
    let UserUpdateView::Name(view) = data else {
        panic!("unexpected session token");
    };
    assert_eq!(view.name, "alice-by-admin");
}

#[tokio::test]
async fn update_user_admin_rename_to_a_taken_name_is_a_bad_request() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;
    let (other, _) = session_for(&context, "bob@example.com").await;
    crate::repository::user::update_user_name(&context.state.database, &other, "taken-name")
        .await
        .expect("set other name");

    let error = crate::logic::user::update_user(
        &context.state,
        &admin,
        &target,
        UserUpdateRequest {
            name: Some("taken-name".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("name already taken"));
}

#[tokio::test]
async fn update_user_rejects_a_taken_name() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let (other, _) = session_for(&context, "bob@example.com").await;
    crate::repository::user::update_user_name(&context.state.database, &other, "alice-renamed")
        .await
        .expect("rename other");
    let error = crate::logic::user::update_user(
        &context.state,
        &user_id,
        &user_id,
        UserUpdateRequest {
            name: Some("alice-renamed".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("name already taken"));
}

#[tokio::test]
async fn delete_user_rejects_a_missing_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let error = crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: None,
            token: Some(uuid::Uuid::now_v7().to_string()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request(
            "missing or unsupported delete mode (expected \"transfer\", \"soft\" or \"hard\")"
        )
    );
}

#[tokio::test]
async fn delete_user_hard_by_admin_removes_the_user() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (target, _) = session_for(&context, "alice@example.com").await;
    let data = crate::logic::user::delete_user(
        &context.state,
        &admin,
        &target,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Hard),
            token: Some(uuid::Uuid::now_v7().to_string()),
        },
    )
    .await
    .expect("delete");
    let UserDeleteView::UserId(view) = data else {
        panic!("unexpected empty delete");
    };
    assert_eq!(view.user_id, target);
    assert!(
        crate::repository::user::read_user(&context.state.database, &target)
            .await
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn delete_user_transfer_after_email_confirmation() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let confirmation_token = messages[0].2.clone();

    let delete_token = confirmation_token.clone();
    let data = crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Transfer),
            token: Some(delete_token),
        },
    )
    .await
    .expect("transfer delete");
    assert!(matches!(data, UserDeleteView::Empty(_)));
    assert!(
        crate::repository::user::read_user(&context.state.database, &user_id)
            .await
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn update_user_admin_rename_of_missing_user_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let error = crate::logic::user::update_user(
        &context.state,
        &admin,
        "no-such-user",
        UserUpdateRequest {
            name: Some("renamed".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::not_found("user not found"));
}

#[tokio::test]
async fn delete_user_soft_after_email_confirmation() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let confirmation_token = messages[0].2.clone();

    let delete_token = confirmation_token.clone();
    let data = crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Soft),
            token: Some(delete_token),
        },
    )
    .await
    .expect("soft delete");
    assert!(matches!(data, UserDeleteView::Empty(_)));
    assert!(
        crate::repository::delete::is_soft_deleted(&context.state.database, "user", &user_id)
            .await
            .expect("soft-deleted check")
    );
}

#[tokio::test]
async fn undelete_soft_user_by_admin_revives_the_user() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");

    let messages = context.emails();
    let confirmation_token = messages[0].2.clone();

    let delete_token = confirmation_token.clone();
    crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Soft),
            token: Some(delete_token),
        },
    )
    .await
    .expect("soft delete");
    assert!(
        crate::repository::delete::is_soft_deleted(&context.state.database, "user", &user_id)
            .await
            .expect("soft-deleted check")
    );

    crate::logic::user::undelete_soft_user(&context.state, &admin, &user_id)
        .await
        .expect("undelete");
    assert!(
        !crate::repository::delete::is_soft_deleted(&context.state.database, "user", &user_id)
            .await
            .expect("soft-deleted check")
    );
}

#[tokio::test]
async fn undelete_soft_user_is_forbidden_for_a_member() {
    let context = TestCtx::new().await.expect("test context");
    let (member, _) = session_for(&context, "bob@example.com").await;
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");

    let messages = context.emails();
    let confirmation_token = messages[0].2.clone();

    let delete_token = confirmation_token.clone();
    crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Soft),
            token: Some(delete_token),
        },
    )
    .await
    .expect("soft delete");

    let error = crate::logic::user::undelete_soft_user(&context.state, &member, &user_id)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn delete_user_transfer_rejects_a_token_for_a_different_account() {
    let context = TestCtx::new().await.expect("test context");
    let (alice_id, alice_token) = session_for(&context, "alice@example.com").await;
    let (bob_id, _) = session_for(&context, "bob@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(alice_token),
    )
    .await
    .expect("deregister email");

    let messages = context.emails();
    let confirmation_token = messages[0].2.clone();

    let delete_token = confirmation_token.clone();
    let error = crate::logic::user::delete_user(
        &context.state,
        &bob_id,
        &bob_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Transfer),
            token: Some(delete_token),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("delete token does not match your account")
    );
    let _ = alice_id;
}

#[tokio::test]
async fn update_user_requires_both_email_tokens() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let error = crate::logic::user::update_user(
        &context.state,
        &user_id,
        &user_id,
        UserUpdateRequest {
            name: None,
            old_email_token: Some(uuid::Uuid::now_v7().to_string()),
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("old_email_token and new_email_token must both be provided")
    );
}

#[tokio::test]
async fn delete_user_transfer_rejects_an_expired_token_for_an_existing_account() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let delete_token = uuid::Uuid::now_v7().to_string();
    let error = crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Transfer),
            token: Some(delete_token),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("invalid or expired delete token")
    );
}

#[tokio::test]
async fn create_user_rejects_a_soft_deleted_account() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");
    let messages = context.emails();
    let confirmation_token = messages[0].2.clone();
    let delete_token = confirmation_token.clone();
    crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Soft),
            token: Some(delete_token),
        },
    )
    .await
    .expect("soft delete");

    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::CreateUser,
            email: Some("alice@example.com".to_string()),
            old_email: None,
            new_email: None,
        },
        None,
    )
    .await
    .expect("create token");
    let messages = context.emails();
    let register_token = messages[1].2.clone();

    let error = crate::logic::user::create_user(&context.state, &register_token)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("email address is deactivated")
    );
    assert!(
        crate::repository::delete::is_soft_deleted(&context.state.database, "user", &user_id)
            .await
            .expect("soft-deleted check")
    );
}

#[tokio::test]
async fn read_user_hides_a_soft_deleted_account_from_members() {
    let context = TestCtx::new().await.expect("test context");
    let (admin, _) = admin_session(&context).await;
    let (user_id, token) = session_for(&context, "alice@example.com").await;

    let email = "alice@example.com";
    let _ = crate::logic::email::create_token(
        &context.state,
        nail_common::request::CreateTokenRequest {
            purpose: nail_common::request::TokenPurpose::DeleteUser,
            email: Some(email.to_string()),
            old_email: None,
            new_email: None,
        },
        Some(token),
    )
    .await
    .expect("deregister email");
    let messages = context.emails();
    let confirmation_token = messages[0].2.clone();
    let delete_token = confirmation_token.clone();
    crate::logic::user::delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Soft),
            token: Some(delete_token),
        },
    )
    .await
    .expect("soft delete");

    let error = crate::logic::user::read_user(&context.state, &user_id, &user_id, true, false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::not_found("user not found"));
    let view = crate::logic::user::read_user(&context.state, &admin, &user_id, true, false)
        .await
        .expect("admin holds User::Undelete::Soft");
    assert_eq!(view.id.as_deref(), Some(user_id.as_str()));
}
