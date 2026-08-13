use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::{TestCtx, valid_pdf};
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

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    let (user_id, token) = session_for(context, email).await;
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    (user_id, token)
}

#[tokio::test]
async fn create_article_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "My Article"),
        ("summary", "A summary."),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["data"]["article_id"].as_str().is_some());
    assert!(body["data"]["version_id"].as_str().is_some());
    assert_eq!(body["message"].as_str(), Some("created"));
}

#[tokio::test]
async fn create_article_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let fields: Vec<(&str, &str)> = vec![("title", "Title")];
    let (status, body) = context
        .post_multipart("/article/create", None, &fields, "file", "a.pdf", &valid_pdf())
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("missing session-token header"));
}

#[tokio::test]
async fn create_article_requires_permission() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = session_for(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Title"),
        ("summary", "summary"),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "a.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn create_article_rejects_an_empty_title() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", ""),
        ("summary", "summary"),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "a.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("text cannot be empty"));
}

#[tokio::test]
async fn read_article_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Titled"),
        ("summary", "A summary."),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (_, create_body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;
    let article_id = create_body["data"]["article_id"].as_str().expect("article id");

    let (status, body) = context
        .get(&format!("/article/{article_id}/read?check_if_is_author=true"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["title"].as_str(), Some("Titled"));
    assert_eq!(body["data"]["author_id"].as_str(), Some(user_id.as_str()));
    assert_eq!(body["data"]["is_author"].as_bool(), Some(true));
}

#[tokio::test]
async fn read_articles_plain_list_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Listed"),
        ("summary", "summary"),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let _ = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;

    let (status, body) = context.get("/article/read?page=1&limit=8", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
    assert_eq!(body["data"]["article_list"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn article_requires_a_session_for_reads() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context.get("/article/read", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}

#[tokio::test]
async fn delete_article_rejects_missing_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Deletable"),
        ("summary", "summary"),
        ("tags", "#rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (_, create_body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;
    let article_id = create_body["data"]["article_id"].as_str().expect("article id");

    let (status, body) = context
        .post(&format!("/article/{article_id}/delete"), json!({}), Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}
