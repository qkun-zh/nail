use axum::http::StatusCode;
use uuid::Uuid;

use super::context::{TestCtx, valid_pdf};
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::cache::{SessionTokenEntry, token_key};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops"];

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
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

async fn plain_session(context: &TestCtx, email: &str) -> (String, String) {
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

async fn create_article_over_http(context: &TestCtx, token: &str) -> (String, String) {
    let fields: Vec<(&str, &str)> = vec![
        ("title", "Downloadable"),
        ("summary", "summary"),
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
    let version_id = body["data"]["version_id"]
        .as_str()
        .expect("version id")
        .to_string();
    (article_id, version_id)
}

async fn article_without_pdf_file(context: &TestCtx, author_id: &str) -> (String, String) {
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    create_article(
        &context.state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Missing {article_id}"),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&valid_pdf()),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

fn token_from_url(url: &str) -> &str {
    url.split("?token=").nth(1).expect("token in minted url")
}

#[tokio::test]
async fn read_content_mints_a_json_url() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = create_article_over_http(&context, &token).await;

    let (status, body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read?mode=download"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("ok"));
    let url = body["data"]["url"].as_str().expect("url");
    assert!(url.starts_with(&format!(
        "/api/article/{article_id}/version/{version_id}/content/read?token="
    )));
    assert!(Uuid::parse_str(token_from_url(url)).is_ok());
}

#[tokio::test]
async fn read_content_consumes_a_minted_token_once() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = create_article_over_http(&context, &token).await;

    let (_, mint_body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read?mode=download"),
            Some(&token),
        )
        .await;
    let download_token = token_from_url(mint_body["data"]["url"].as_str().expect("url"));

    let (status, _, bytes) = context
        .get_bytes(
            &format!(
                "/article/{article_id}/version/{version_id}/content/read?token={download_token}"
            ),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, valid_pdf());

    let (status, body) = context
        .get(
            &format!(
                "/article/{article_id}/version/{version_id}/content/read?token={download_token}"
            ),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("invalid or expired download token")
    );
}

#[tokio::test]
async fn read_content_rejects_a_token_bound_to_another_account() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (_, other_token) = member_session(&context, "bob@example.com").await;
    let (article_id, version_id) = create_article_over_http(&context, &token).await;

    let (_, mint_body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read?mode=download"),
            Some(&token),
        )
        .await;
    let download_token = token_from_url(mint_body["data"]["url"].as_str().expect("url"));

    let (status, body) = context
        .get(
            &format!(
                "/article/{article_id}/version/{version_id}/content/read?token={download_token}"
            ),
            Some(&other_token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("download token is bound to another account")
    );
}

#[tokio::test]
async fn read_content_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = create_article_over_http(&context, &token).await;

    let (status, _, _) = context
        .get_bytes(
            &format!("/article/{article_id}/version/{version_id}/content/read"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn read_content_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = article_without_pdf_file(&context, &user_id).await;

    let (_, outsider) = plain_session(&context, "bob@example.com").await;
    let (status, body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read?mode=download"),
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_content_requires_a_token() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = create_article_over_http(&context, &token).await;

    let (status, body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("missing download token"));
}

#[tokio::test]
async fn read_content_reports_a_missing_pdf_file() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (article_id, version_id) = article_without_pdf_file(&context, &user_id).await;

    let (_, mint_body) = context
        .get(
            &format!("/article/{article_id}/version/{version_id}/content/read?mode=download"),
            Some(&token),
        )
        .await;
    let download_token = token_from_url(mint_body["data"]["url"].as_str().expect("url"));

    let (status, body) = context
        .get(
            &format!(
                "/article/{article_id}/version/{version_id}/content/read?token={download_token}"
            ),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("PDF file not found"));
}

#[tokio::test]
async fn read_content_reports_a_missing_version() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (article_id, _) = create_article_over_http(&context, &token).await;

    let (status, body) = context
        .get(
            &format!(
                "/article/{article_id}/version/{}/content/read?mode=download",
                Uuid::now_v7()
            ),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("article version not found"));
}
