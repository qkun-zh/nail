use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::logic::role::{
    RoleUpdate, create_role, delete_role, read_role, read_roles, update_role, validate_role_name,
};

async fn admin(context: &TestCtx) -> String {
    crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("user")
}

#[tokio::test]
async fn validate_role_name_rejects_empty() {
    assert_eq!(
        validate_role_name(""),
        Err(LogicError::bad_request("invalid role name"))
    );
}

#[tokio::test]
async fn validate_role_name_rejects_too_long() {
    let long = "a".repeat(65);
    assert_eq!(
        validate_role_name(&long),
        Err(LogicError::bad_request("invalid role name"))
    );
}

#[tokio::test]
async fn validate_role_name_rejects_invalid_chars() {
    assert_eq!(
        validate_role_name("bad name!"),
        Err(LogicError::bad_request("invalid role name"))
    );
}

#[tokio::test]
async fn validate_role_name_accepts_valid() {
    assert_eq!(
        validate_role_name("admin-role"),
        Ok("admin-role".to_string())
    );
    assert_eq!(validate_role_name(" editor "), Ok("editor".to_string()));
}

#[tokio::test]
async fn create_role_rejects_duplicate() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    create_role(&context.state, &admin_id, "editor")
        .await
        .expect("create");
    let err = create_role(&context.state, &admin_id, "editor")
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::bad_request("role already exists"));
}

#[tokio::test]
async fn read_role_not_found() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    let err = read_role(&context.state, &admin_id, "missing")
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::not_found("role not found"));
}

#[tokio::test]
async fn delete_role_rejects_required_role() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    let role = read_roles(&context.state, &admin_id, 1, 100)
        .await
        .expect("roles")
        .items
        .into_iter()
        .find(|role| role.name == "admin")
        .expect("admin role");
    let err = delete_role(&context.state, &admin_id, &role.id)
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("required role"), "got: {msg}");
}

#[tokio::test]
async fn update_role_not_found() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    let err = update_role(
        &context.state,
        &admin_id,
        "missing",
        RoleUpdate {
            permissions_add: &[],
            permissions_remove: &[],
            users_add: &[],
            users_remove: &[],
        },
    )
    .await
    .unwrap_err();
    assert_eq!(err, LogicError::not_found("role not found"));
}

#[tokio::test]
async fn update_role_rejects_destructive_change_on_required_role() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    let role = read_roles(&context.state, &admin_id, 1, 100)
        .await
        .expect("roles")
        .items
        .into_iter()
        .find(|role| role.name == "member")
        .expect("member role");
    let err = update_role(
        &context.state,
        &admin_id,
        &role.id,
        RoleUpdate {
            permissions_add: &[],
            permissions_remove: &["some-perm".to_string()],
            users_add: &[],
            users_remove: &[],
        },
    )
    .await
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("required role"), "got: {msg}");
}

#[tokio::test]
async fn read_roles_is_paginated() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    create_role(&context.state, &admin_id, "role-a")
        .await
        .expect("a");
    create_role(&context.state, &admin_id, "role-b")
        .await
        .expect("b");
    let page = read_roles(&context.state, &admin_id, 1, 1)
        .await
        .expect("page");
    assert_eq!(page.items.len(), 1);
    assert!(page.has_next);
}

#[tokio::test]
async fn role_crud_round_trip_by_id() {
    let context = TestCtx::new().await.expect("context");
    let admin_id = admin(&context).await;
    let (role_id, name) = create_role(&context.state, &admin_id, "editor")
        .await
        .expect("create");
    assert_eq!(name, "editor");
    let view = read_role(&context.state, &admin_id, &role_id)
        .await
        .expect("read");
    assert_eq!(view.id, role_id);
    assert_eq!(view.name, "editor");
    let view = update_role(
        &context.state,
        &admin_id,
        &role_id,
        RoleUpdate {
            permissions_add: &["Article::Read".to_string()],
            permissions_remove: &[],
            users_add: &[],
            users_remove: &[],
        },
    )
    .await
    .expect("update");
    assert_eq!(view.id, role_id);
    let view = read_role(&context.state, &admin_id, &role_id)
        .await
        .expect("read after update");
    assert!(view.permissions.contains(&"Article::Read".to_string()));
    let view = delete_role(&context.state, &admin_id, &role_id)
        .await
        .expect("delete");
    assert_eq!(view.id, role_id);
    let err = read_role(&context.state, &admin_id, &role_id)
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::not_found("role not found"));
}
