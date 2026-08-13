use nail_common::request::{DeleteMode, UserDeleteRequest, UserUpdateRequest};

use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::logic::user::{create_user, delete_user, read_user, read_users, update_user};
use crate::repository::cache::token_key;

async fn member_id(context: &TestCtx, email: &str) -> String {
    crate::repository::user::find_or_create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("member")
}

async fn admin_id(context: &TestCtx) -> String {
    crate::repository::user::find_user_by_email_address_hash(
        &context.state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup")
    .expect("user zero")
}

#[tokio::test]
async fn read_user_self_returns_name_by_default_and_email_hash_on_request() {
    let context = TestCtx::new().await.expect("test context");
    let user_id = member_id(&context, "alice@example.com").await;

    let data = read_user(&context.state, &user_id, &user_id, true, false)
        .await
        .expect("self read");
    assert_eq!(data["name"].as_str(), Some(user_id.replace('-', "").as_str()));
    assert!(data.get("email_hash").is_none());

    let data = read_user(&context.state, &user_id, &user_id, true, true)
        .await
        .expect("self read with email");
    assert_eq!(
        data["email_hash"].as_str(),
        Some(nail_common::hash::email("alice@example.com").as_str())
    );
}

#[tokio::test]
async fn read_user_self_surfaces_a_missing_user() {
    let context = TestCtx::new().await.expect("test context");
    let error = read_user(&context.state, "missing", "missing", true, false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::unauthorized("user not found"));
}

#[tokio::test]
async fn read_user_other_requires_user_read_permission() {
    let context = TestCtx::new().await.expect("test context");
    let member = member_id(&context, "alice@example.com").await;
    let target = member_id(&context, "bob@example.com").await;
    let error = read_user(&context.state, &member, &target, true, false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn read_user_other_returns_id_name_and_email_hash() {
    let context = TestCtx::new().await.expect("test context");
    let admin = admin_id(&context).await;
    let target = member_id(&context, "alice@example.com").await;

    let data = read_user(&context.state, &admin, &target, true, true)
        .await
        .expect("admin read");
    assert_eq!(data["id"].as_str(), Some(target.as_str()));
    assert_eq!(data["email_hash"].as_str(), Some(nail_common::hash::email("alice@example.com").as_str()));
}

#[tokio::test]
async fn read_user_other_reports_a_missing_user() {
    let context = TestCtx::new().await.expect("test context");
    let admin = admin_id(&context).await;
    let error = read_user(&context.state, &admin, "missing", true, false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::not_found("user not found"));
}

#[tokio::test]
async fn update_user_self_rename_requires_a_pow() {
    let context = TestCtx::new().await.expect("test context");
    let user_id = member_id(&context, "alice@example.com").await;
    let pow = context.issued_pow("alice-renamed");
    let data = update_user(
        &context.state,
        &user_id,
        &user_id,
        UserUpdateRequest {
            pow: Some(pow),
            name: None,
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .expect("rename");
    assert_eq!(data["name"].as_str(), Some("alice-renamed"));
}

#[tokio::test]
async fn update_user_self_rename_rejects_a_taken_name() {
    let context = TestCtx::new().await.expect("test context");
    let first = member_id(&context, "a@example.com").await;
    let second = member_id(&context, "b@example.com").await;
    crate::repository::user::update_user_name(&context.state.graph, &first, "taken")
        .await
        .expect("first rename");

    let pow = context.issued_pow("taken");
    let error = update_user(
        &context.state,
        &second,
        &second,
        UserUpdateRequest {
            pow: Some(pow),
            name: None,
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("name already taken"));
}

#[tokio::test]
async fn update_user_admin_rename_requires_permission() {
    let context = TestCtx::new().await.expect("test context");
    let member = member_id(&context, "alice@example.com").await;
    let target = member_id(&context, "bob@example.com").await;
    let error = update_user(
        &context.state,
        &member,
        &target,
        UserUpdateRequest {
            pow: None,
            name: Some("renamed".to_string()),
            old_email_token: None,
            new_email_token: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn update_user_admin_rename_reports_a_missing_user() {
    let context = TestCtx::new().await.expect("test context");
    let admin = admin_id(&context).await;
    let error = update_user(
        &context.state,
        &admin,
        "missing",
        UserUpdateRequest {
            pow: None,
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
async fn delete_user_transfer_removes_the_user() {
    let context = TestCtx::new().await.expect("test context");
    let user_id = member_id(&context, "alice@example.com").await;
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.delete_user.insert(
        &key,
        crate::repository::cache::DeleteUserTokenEntry {
            user_id: user_id.clone(),
            email_address_hash: nail_common::hash::email("alice@example.com"),
        },
    );
    let pow = context.issued_pow(&token);

    let data = delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteRequest {
            mode: Some(DeleteMode::Transfer),
            pow,
        },
    )
    .await
    .expect("delete");
    assert_eq!(data, serde_json::json!({}));
    assert!(crate::repository::user::read_user(&context.state.graph, &user_id)
        .await
        .expect("read")
        .is_none());
}

#[tokio::test]
async fn delete_user_transfer_is_idempotent_when_the_user_is_already_gone() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let pow = context.issued_pow(&token);
    let data = delete_user(
        &context.state,
        "missing-user",
        "missing-user",
        UserDeleteRequest {
            mode: Some(DeleteMode::Transfer),
            pow,
        },
    )
    .await
    .expect("idempotent delete");
    assert_eq!(data, serde_json::json!({}));
}

#[tokio::test]
async fn delete_user_transfer_rejects_a_token_bound_to_another_account() {
    let context = TestCtx::new().await.expect("test context");
    let user_id = member_id(&context, "alice@example.com").await;
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.delete_user.insert(
        &key,
        crate::repository::cache::DeleteUserTokenEntry {
            user_id: "another-user".to_string(),
            email_address_hash: nail_common::hash::email("alice@example.com"),
        },
    );
    let pow = context.issued_pow(&token);

    let error = delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteRequest {
            mode: Some(DeleteMode::Transfer),
            pow,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("delete token does not match your account")
    );
}

#[tokio::test]
async fn delete_user_transfer_rejects_an_unknown_token_for_a_live_user() {
    let context = TestCtx::new().await.expect("test context");
    let user_id = member_id(&context, "alice@example.com").await;
    let token = uuid::Uuid::now_v7().to_string();
    let pow = context.issued_pow(&token);
    let error = delete_user(
        &context.state,
        &user_id,
        &user_id,
        UserDeleteRequest {
            mode: Some(DeleteMode::Transfer),
            pow,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("invalid or expired delete token"));
}

#[tokio::test]
async fn delete_user_hard_requires_permission() {
    let context = TestCtx::new().await.expect("test context");
    let member = member_id(&context, "alice@example.com").await;
    let target = member_id(&context, "bob@example.com").await;
    let error = delete_user(
        &context.state,
        &member,
        &target,
        UserDeleteRequest {
            mode: Some(DeleteMode::Hard),
            pow: context.issued_pow("ignored"),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn delete_user_hard_removes_the_target_user() {
    let context = TestCtx::new().await.expect("test context");
    let admin = admin_id(&context).await;
    let target = member_id(&context, "alice@example.com").await;
    let data = delete_user(
        &context.state,
        &admin,
        &target,
        UserDeleteRequest {
            mode: Some(DeleteMode::Hard),
            pow: context.issued_pow("ignored"),
        },
    )
    .await
    .expect("delete");
    assert_eq!(data["user_id"].as_str(), Some(target.as_str()));
}

#[tokio::test]
async fn read_users_requires_permission() {
    let context = TestCtx::new().await.expect("test context");
    let member = member_id(&context, "alice@example.com").await;
    let error = read_users(&context.state, &member, 1, 8).await.unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn read_users_returns_the_paginated_list() {
    let context = TestCtx::new().await.expect("test context");
    let admin = admin_id(&context).await;
    member_id(&context, "alice@example.com").await;
    let data = read_users(&context.state, &admin, 1, 8).await.expect("read");
    assert!(data["user_list"].as_array().is_some());
    assert!(data["total"].as_u64().is_some());
    assert_eq!(data["has_next"].as_bool(), Some(false));
}

#[tokio::test]
async fn create_user_consumes_the_token_and_creates_a_user_with_member_role() {
    let context = TestCtx::new().await.expect("test context");
    let email_hash = nail_common::hash::email("alice@example.com");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.create_user.insert(
        &key,
        crate::repository::cache::CreateUserTokenEntry {
            email_address_hash: email_hash.clone(),
            email_subject: uuid::Uuid::now_v7().to_string(),
        },
    );

    let pow = context.issued_pow(&token);
    let user_id = create_user(&context.state, &pow).await.expect("create");

    let created = crate::repository::user::find_user_by_email_address_hash(
        &context.state.graph,
        &email_hash,
    )
    .await
    .expect("read user")
    .expect("user exists");
    assert_eq!(user_id, created);

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
async fn create_user_rejects_an_invalid_token() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("not-a-uuid");
    assert_eq!(
        create_user(&context.state, &pow).await.unwrap_err(),
        LogicError::bad_request("invalid or expired token")
    );
}

#[tokio::test]
async fn create_user_rejects_an_unknown_or_expired_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let pow = context.issued_pow(&token);
    assert_eq!(
        create_user(&context.state, &pow).await.unwrap_err(),
        LogicError::bad_request("invalid or expired token")
    );
}
