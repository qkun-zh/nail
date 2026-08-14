use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::{TestCtx, unique_pdf};
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::cache::{SessionTokenEntry, token_key};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
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

async fn article_fixture(context: &TestCtx, author_id: &str) -> (String, String) {
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    let title = format!("Versioned {article_id}");
    create_article(
        &context.state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.clone(),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&unique_pdf(&title)),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn create_version_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &user_id).await;

    let fields: Vec<(&str, &str)> = vec![("version", "1.1.0"), ("note", "next")];
    let (status, body) = context
        .post_multipart(
            &format!("/article/{article_id}/version/create"),
            Some(&token),
            &fields,
            "file",
            "version.pdf",
            &unique_pdf("version-1.1.0"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["data"]["version_id"].as_str().is_some());
    assert_eq!(body["message"].as_str(), Some("created"));
}

#[tokio::test]
async fn create_version_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &user_id).await;

    let fields: Vec<(&str, &str)> = vec![("version", "1.1.0")];
    let (status, body) = context
        .post_multipart(
            &format!("/article/{article_id}/version/create"),
            None,
            &fields,
            "file",
            "version.pdf",
            &unique_pdf("version-unauth"),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}

#[tokio::test]
async fn read_versions_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &user_id).await;

    let (status, body) = context
        .get(&format!("/article/{article_id}/version/read?page=1&limit=8"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
    assert_eq!(body["data"]["version_list"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn read_version_cross_check_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = article_fixture(&context, &user_id).await;
    let (other_article, _) = article_fixture(&context, &user_id).await;

    let (status, body) = context
        .get(
            &format!("/version/{version_id}/read?article_id={article_id}"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["version"].as_str(), Some("1.0.0"));

    let (status, _) = context
        .get(
            &format!("/version/{version_id}/read?article_id={other_article}"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_version_note_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = admin_session(&context).await;
    let (_, version_id) = article_fixture(&context, &user_id).await;

    let (status, body) = context
        .post(
            &format!("/version/{version_id}/update"),
            json!({ "note": "updated note" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["version_id"].as_str(), Some(version_id.as_str()));
}

#[tokio::test]
async fn delete_version_rejects_transfer_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (_, version_id) = article_fixture(&context, &user_id).await;

    let (status, body) = context
        .post(
            &format!("/version/{version_id}/delete"),
            json!({ "mode": "transfer" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("version delete only supports mode \"hard\"")
    );
}

#[tokio::test]
async fn delete_version_hard_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = admin_session(&context).await;
    let (_, version_id) = article_fixture(&context, &user_id).await;

    let (status, body) = context
        .post(
            &format!("/version/{version_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
}
