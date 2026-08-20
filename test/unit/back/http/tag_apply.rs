use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::{TestCtx, valid_pdf};
use crate::repository::cache::{SessionTokenEntry, token_key};

async fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    let token = Uuid::now_v7().to_string();
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

async fn create_article_and_tag(context: &TestCtx, token: &str) -> (String, String) {
    for name in ["rust", "devops"] {
        let (status, body) = context
            .post("/tag/create", json!({ "name": name }), Some(token))
            .await;
        assert_eq!(status, StatusCode::CREATED, "body: {body}");
    }
    let tag_id = {
        let (_, body) = context.get("/tag/read?page=1&limit=200", Some(token)).await;
        assert_eq!(body["data"]["total"].as_u64(), Some(2));
        body["data"]["items"]
            .as_array()
            .expect("tag list")
            .iter()
            .find(|tag| tag["name"].as_str() == Some("devops"))
            .expect("devops tag")
            .get("id")
            .and_then(|id| id.as_str())
            .expect("tag id")
            .to_string()
    };

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Http Tag Article"),
        ("summary", "A summary."),
        ("tags", "rust"),
        ("version", "1.0.0"),
        ("note", "note"),
    ];
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(token),
            &fields,
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    let article_id = body["data"]["article_id"]
        .as_str()
        .expect("article id")
        .to_string();
    (article_id, tag_id)
}

#[tokio::test]
async fn apply_and_unapply_tag_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    let (article_id, tag_id) = create_article_and_tag(&context, &token).await;

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/tag/{tag_id}/apply"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (status, body) = context
        .get(&format!("/tag/{tag_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["article_count"].as_u64(), Some(1));

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/tag/{tag_id}/unapply"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (status, body) = context
        .get(&format!("/tag/{tag_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["article_count"].as_u64(), Some(0));
}

#[tokio::test]
async fn apply_tag_to_a_missing_article_returns_404() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    let (_, tag_id) = create_article_and_tag(&context, &token).await;

    let (status, body) = context
        .post(
            &format!("/article/missing/tag/{tag_id}/apply"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("article not found"));
}

#[tokio::test]
async fn apply_tag_to_a_missing_tag_returns_404() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = admin_session(&context).await;
    let (article_id, _) = create_article_and_tag(&context, &token).await;

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/tag/missing/apply"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("tag not found"));
}
