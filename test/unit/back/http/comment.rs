use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::{TestCtx, unique_pdf};
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::cache::{SessionTokenEntry, token_key};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER).await.expect("member role");
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
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("admin");
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

async fn version_fixture(context: &TestCtx, author_id: &str) -> String {
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    create_article(
        &context.state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Article {article_id}"),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&unique_pdf(&article_id)),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    version_id
}

#[tokio::test]
async fn create_comment_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (status, body) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["data"]["comment_id"].as_str().is_some());
    assert_eq!(body["message"].as_str(), Some("created"));
}

#[tokio::test]
async fn create_reply_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "top" }),
            Some(&token),
        )
        .await;
    let top_id = created["data"]["comment_id"].as_str().expect("top id");

    let (status, body) = context
        .post(
            &format!("/comments/{top_id}/replies/create"),
            json!({ "content": "reply" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["data"]["comment_id"].as_str().is_some());
}

#[tokio::test]
async fn read_comments_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;

    let (status, body) = context
        .get(
            &format!("/version/{version_id}/comments/read?page=1&limit=8"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
    assert_eq!(body["data"]["comments"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"]["comments"][0]["user_name"].as_str().is_some(), true);
}

#[tokio::test]
async fn update_comment_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, _) = context
        .post(
            &format!("/comment/{comment_id}/update"),
            json!({ "content": "stolen" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (_, admin_token) = admin_session(&context).await;
    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/update"),
            json!({ "content": "edited" }),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["comment_id"].as_str(), Some(comment_id));
}

#[tokio::test]
async fn delete_comment_transfer_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "transfer" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
}

#[tokio::test]
async fn delete_comment_requires_a_mode_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

#[tokio::test]
async fn create_comment_requires_a_session_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (status, body) = context
        .post(
            &format!("/version/{version_id}/comments/create"),
            json!({ "content": "hello" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}
