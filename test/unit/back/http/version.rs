use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use cache::UserId;

use super::context::{TestCtx, unique_pdf};
use crate::logic::session::cache_key;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_MEMBER).expect("member role");
    let token = Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));
    (user_id, token)
}

fn admin_session(context: &TestCtx) -> (String, String) {
    member_session(context, "user-zero@example.com")
}

fn plain_session(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    let token = Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));
    (user_id, token)
}

fn article_fixture(context: &TestCtx, author_id: &str) -> (String, String) {
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    let title = format!("Versioned {article_id}");
    create_article(
        &context.state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.clone(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&unique_pdf(&title)),
                note: "note".to_string(),
            },
        },
    )
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn create_version_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let fields: Vec<(&str, &str)> = vec![("version", "1.1.0"), ("note", "next")];
    let (status, body) = context
        .post_multipart(
            &format!("/articles/{article_id}/versions"),
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
async fn create_version_ignores_unknown_multipart_fields() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let fields: Vec<(&str, &str)> = vec![
        ("version", "1.1.0"),
        ("note", "next"),
        ("unexpected_field", "ignored value"),
    ];
    let (status, body) = context
        .post_multipart(
            &format!("/articles/{article_id}/versions"),
            Some(&token),
            &fields,
            "file",
            "version.pdf",
            &unique_pdf("version-1.1.0"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["data"]["version_id"].as_str().is_some());
}

#[tokio::test]
async fn create_version_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let fields: Vec<(&str, &str)> = vec![("version", "1.1.0")];
    let (status, body) = context
        .post_multipart(
            &format!("/articles/{article_id}/versions"),
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
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let (status, body) = context
        .get(
            &format!("/articles/{article_id}/versions?page=1&limit=8"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"]["has_next"].as_bool(), Some(false));
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
}

#[tokio::test]
async fn read_versions_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let (status, body) = context
        .get(
            &format!("/articles/{article_id}/versions?page=1025"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("page exceeds max search pages")
    );
}

#[tokio::test]
async fn read_versions_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);

    let (_, outsider) = plain_session(&context, "bob@example.com");
    let (status, body) = context
        .get(&format!("/articles/{article_id}/versions"), Some(&outsider))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_version_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);

    let (_, outsider) = plain_session(&context, "bob@example.com");
    let (status, body) = context
        .get(&format!("/versions/{version_id}"), Some(&outsider))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_version_cross_check_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, version_id) = article_fixture(&context, &user_id);
    let (other_article, _) = article_fixture(&context, &user_id);

    let (status, body) = context
        .get(
            &format!("/versions/{version_id}?article_id={article_id}"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["version"].as_str(), Some("1.0.0"));

    let (status, _) = context
        .get(
            &format!("/versions/{version_id}?article_id={other_article}"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_version_note_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, body) = context
        .patch(
            &format!("/versions/{version_id}"),
            json!({ "note": "updated note" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["data"]["version_id"].as_str(),
        Some(version_id.as_str())
    );
}

#[tokio::test]
async fn delete_version_rejects_transfer_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, body) = context
        .delete(
            &format!("/versions/{version_id}?mode=transfer"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("version delete only supports mode \"soft\" or \"hard\"")
    );
}

#[tokio::test]
async fn delete_version_hard_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (_, admin_token) = admin_session(&context);
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, body) = context
        .delete(
            &format!("/versions/{version_id}?mode=hard"),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
}

#[tokio::test]
async fn delete_version_hard_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, body) = context
        .delete(&format!("/versions/{version_id}?mode=hard"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn undelete_soft_version_revives_the_version_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (_, admin_token) = admin_session(&context);
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, body) = context
        .delete(
            &format!("/versions/{version_id}?mode=soft"),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (status, body) = context
        .post(
            &format!("/versions/{version_id}/restore"),
            json!({}),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("undeleted"));

    let (status, body) = context
        .get(&format!("/versions/{version_id}"), Some(&admin_token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
}

#[tokio::test]
async fn undelete_soft_version_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (_, admin_token) = admin_session(&context);
    let (_, version_id) = article_fixture(&context, &user_id);

    let (status, _) = context
        .delete(
            &format!("/versions/{version_id}?mode=soft"),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = context
        .post(
            &format!("/versions/{version_id}/restore"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn create_version_rejects_an_older_version() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);
    let fields: Vec<(&str, &str)> = vec![("version", "0.9.0"), ("note", "older")];
    let (status, body) = context
        .post_multipart(
            &format!("/articles/{article_id}/versions"),
            Some(&token),
            &fields,
            "file",
            "older.pdf",
            &unique_pdf("older-version"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("new version must be strictly greater than the latest version")
    );
}

#[tokio::test]
async fn create_version_rejects_a_duplicate_content_hash() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com");
    let (article_id, _) = article_fixture(&context, &user_id);
    let title = format!("Versioned {article_id}");
    let fields: Vec<(&str, &str)> = vec![("version", "1.1.0"), ("note", "duplicate")];
    let (status, body) = context
        .post_multipart(
            &format!("/articles/{article_id}/versions"),
            Some(&token),
            &fields,
            "file",
            "dup.pdf",
            &unique_pdf(&title),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    let message = body["message"].as_str().expect("message");
    assert!(
        message.contains("identical PDF already exists"),
        "message: {message}"
    );
}

#[tokio::test]
async fn read_version_reports_a_missing_version() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com");
    let (status, body) = context
        .get(&format!("/versions/{}", Uuid::now_v7()), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("version not found"));
}

#[tokio::test]
async fn update_version_is_forbidden_for_a_non_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (_, stranger_token) = member_session(&context, "bob@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);
    let (status, body) = context
        .patch(
            &format!("/versions/{version_id}"),
            json!({ "note": "hijacked" }),
            Some(&stranger_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn delete_version_is_forbidden_for_a_non_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com");
    let (_, stranger_token) = member_session(&context, "bob@example.com");
    let (_, version_id) = article_fixture(&context, &user_id);
    let (status, body) = context
        .delete(
            &format!("/versions/{version_id}?mode=hard"),
            Some(&stranger_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn delete_version_reports_a_missing_version() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com");
    let (status, body) = context
        .delete(
            &format!("/versions/{}?mode=hard", Uuid::now_v7()),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("version not found"));
}
