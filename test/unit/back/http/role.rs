use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::TestCtx;
use crate::repository::cache::{SessionTokenEntry, token_key};
use crate::repository::role::{ROLE_MEMBER, hold_role};

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

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    let (user_id, token) = session_for(context, email).await;
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    (user_id, token)
}

#[tokio::test]
async fn create_role_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context
        .post("/role/create", json!({ "name": "editor" }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("editor"));
    assert_eq!(body["message"].as_str(), Some("created"));
}

#[tokio::test]
async fn create_duplicate_role_returns_400() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let _ = context
        .post("/role/create", json!({ "name": "editor" }), Some(&token))
        .await;
    let (status, body) = context
        .post("/role/create", json!({ "name": "editor" }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("role already exists"));
}

#[tokio::test]
async fn role_manage_requires_admin() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let (status, body) = context
        .post("/role/create", json!({ "name": "editor" }), Some(&token))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn read_roles_reports_real_member_counts() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    let (editor_id, _) = member_session(&context, "alice@example.com").await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");
    crate::repository::role::hold_role(&context.state.graph, &editor_id, "editor")
        .await
        .expect("hold editor");

    let (status, body) = context
        .get("/role/read?page=1&limit=200", Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let roles = body["data"]["role_list"].as_array().expect("role list");
    let editor = roles
        .iter()
        .find(|role| role["name"].as_str() == Some("editor"))
        .expect("editor role");
    assert_eq!(editor["member_count"].as_u64(), Some(1));
    assert_eq!(body["data"]["total"].as_u64(), Some(4));
}

#[tokio::test]
async fn read_roles_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context.get("/role/read?page=1025", Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("page exceeds max search pages")
    );
}

#[tokio::test]
async fn read_role_returns_members_and_permissions() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context.get("/role/admin/read", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("admin"));
    assert!(body["data"]["members"].as_array().map_or(0, Vec::len) >= 1);
    assert!(body["data"]["permissions"].as_array().map_or(0, Vec::len) > 0);
}

#[tokio::test]
async fn update_role_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");

    let (status, body) = context
        .post(
            "/role/editor/update",
            json!({ "permissions": { "add": ["Article::Update"] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("editor"));

    let (_, detail) = context.get("/role/editor/read", Some(&token)).await;
    let permissions = detail["data"]["permissions"]
        .as_array()
        .expect("permissions");
    assert!(
        permissions
            .iter()
            .any(|p| p.as_str() == Some("Article::Update"))
    );
}

#[tokio::test]
async fn delete_required_role_returns_400() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context
        .post(
            "/role/member/delete",
            json!({ "mode": "hard" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("role member is a required role and cannot be deleted")
    );
}

#[tokio::test]
async fn delete_role_requires_hard_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");

    let (status, body) = context
        .post(
            "/role/editor/delete",
            json!({ "mode": "transfer" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("role delete only supports mode \"hard\"")
    );
}

#[tokio::test]
async fn delete_role_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");

    let (status, body) = context
        .post(
            "/role/editor/delete",
            json!({ "mode": "hard" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["name"].as_str(), Some("editor"));
    assert_eq!(body["message"].as_str(), Some("deleted"));

    let (status, _) = context.get("/role/editor/read", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_required_role_rejects_destructive_changes() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context
        .post(
            "/role/member/update",
            json!({ "permissions": { "remove": ["Article::Create"] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("role member is a required role and cannot be modified destructively")
    );
}

#[tokio::test]
async fn revoke_from_the_admin_role_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context
        .post(
            "/role/admin/update",
            json!({ "permissions": { "remove": ["Article::Update"] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn revoke_a_permission_from_a_custom_role_succeeds() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");
    crate::repository::role::grant_permission_to_role(
        &context.state.graph,
        "editor",
        "Article::Update",
    )
    .await
    .expect("grant");

    let (status, body) = context
        .post(
            "/role/editor/update",
            json!({ "permissions": { "remove": ["Article::Update"] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (_, detail) = context.get("/role/editor/read", Some(&token)).await;
    let permissions = detail["data"]["permissions"]
        .as_array()
        .expect("permissions");
    assert!(
        !permissions
            .iter()
            .any(|p| p.as_str() == Some("Article::Update"))
    );
}

#[tokio::test]
async fn update_role_holds_and_unholds_users() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("create role");
    let (plain_user, _) = session_for(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            "/role/editor/update",
            json!({ "users": { "add": [plain_user.clone()] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (_, detail) = context.get("/role/editor/read", Some(&token)).await;
    let members = detail["data"]["members"].as_array().expect("members");
    assert!(
        members
            .iter()
            .any(|m| m.as_str() == Some(plain_user.as_str()))
    );

    let (status, body) = context
        .post(
            "/role/editor/update",
            json!({ "users": { "remove": [plain_user.clone()] } }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (_, detail) = context.get("/role/editor/read", Some(&token)).await;
    let members = detail["data"]["members"].as_array().expect("members");
    assert!(
        !members
            .iter()
            .any(|m| m.as_str() == Some(plain_user.as_str()))
    );
}

#[tokio::test]
async fn read_role_reports_a_missing_role() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;

    let (status, body) = context.get("/role/nosuchrole/read", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("role not found"));
}
