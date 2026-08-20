use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::context::{TestCtx, test_config, unique_pdf, valid_pdf};
use crate::repository::cache::{SessionTokenEntry, token_key};
use crate::repository::role::{ROLE_MEMBER, hold_role};

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops", "web", "go", "cpp"];

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

async fn member_session(context: &TestCtx, email: &str) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    let (user_id, token) = session_for(context, email).await;
    hold_role(&context.state.database, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    (user_id, token)
}

async fn admin_session(context: &TestCtx) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    session_for(context, "user-zero@example.com").await
}
fn article_fields<'a>(title: &'a str, tags: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("title", title),
        ("summary", "summary"),
        ("tags", tags),
        ("version", "1.0.0"),
        ("note", "note"),
    ]
}

async fn create_article_fixture(context: &TestCtx, token: &str, title: &str) -> String {
    let fields = article_fields(title, "rust");
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
    body["data"]["article_id"]
        .as_str()
        .expect("article id")
        .to_string()
}

#[tokio::test]
async fn create_article_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "My Article"),
        ("summary", "A summary."),
        ("tags", "rust"),
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
        .post_multipart(
            "/article/create",
            None,
            &fields,
            "file",
            "a.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("missing session-token header")
    );
}

#[tokio::test]
async fn create_article_requires_permission() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = session_for(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Title"),
        ("summary", "summary"),
        ("tags", "rust"),
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
        ("tags", "rust"),
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
        ("tags", "rust"),
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
    let article_id = create_body["data"]["article_id"]
        .as_str()
        .expect("article id");

    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["title"].as_str(), Some("Titled"));
    assert_eq!(body["data"]["author_id"].as_str(), Some(user_id.as_str()));
}

#[tokio::test]
async fn article_requires_a_session_for_reads() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context.get("/article/read", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}

#[tokio::test]
async fn read_article_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (status, create_body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &article_fields("Gated Read", "rust"),
            "file",
            "article.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {create_body}");
    let article_id = create_body["data"]["article_id"]
        .as_str()
        .expect("article id");

    let (_, outsider) = session_for(&context, "bob@example.com").await;
    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&outsider))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn search_articles_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (_, outsider) = session_for(&context, "bob@example.com").await;
    let (status, body) = context.get("/article/read?q=rust", Some(&outsider)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn delete_article_rejects_missing_mode() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let fields: Vec<(&str, &str)> = vec![
        ("title", "Deletable"),
        ("summary", "summary"),
        ("tags", "rust"),
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
    let article_id = create_body["data"]["article_id"]
        .as_str()
        .expect("article id");

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_title() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let fields = article_fields("Twin Title", "rust");
    let (status, _) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "a.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "b.pdf",
            &unique_pdf("other-pdf"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("title already exists"));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_content_hash() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let first = article_fields("First Upload", "rust");
    let (status, _) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &first,
            "file",
            "a.pdf",
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let second = article_fields("Second Upload", "rust");
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &second,
            "file",
            "b.pdf",
            &valid_pdf(),
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
async fn create_article_accepts_plain_tags_and_rejects_invalid_characters() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let fields = article_fields("Plain Tags", "rust web");
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
    assert_eq!(status, StatusCode::CREATED, "body: {body}");

    let fields = article_fields("Bad Tags", "rust#web");
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
    assert_eq!(
        body["message"].as_str(),
        Some("tag name cannot contain '#'")
    );
}

#[tokio::test]
async fn create_article_rejects_an_empty_note() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let mut fields = article_fields("No Note", "rust");
    fields[4] = ("note", "");
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
async fn create_article_rejects_a_non_pdf_file() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let fields = article_fields("Not A Pdf", "rust");
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            "a.txt",
            b"this is definitely not a pdf",
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("Invalid PDF header: must start with %PDF-")
    );
}

#[tokio::test]
async fn create_article_reports_an_oversized_text_field() {
    let mut config = test_config();
    config.server.max_text_field_bytes = 8;
    let context = TestCtx::with_config(config).await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let fields = article_fields("My Article", "rust");
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
    assert_eq!(body["message"].as_str(), Some("text field too large"));
}

#[tokio::test]
async fn create_article_reports_body_too_large() {
    let mut config = test_config();
    config.server.max_pdf_size_bytes = 4096;
    config.server.max_text_field_bytes = 64;
    let context = TestCtx::with_config(config).await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let fields = article_fields("Huge Body", "rust");
    let (status, body) = context
        .post_multipart(
            "/article/create",
            Some(&token),
            &fields,
            "file",
            &"x".repeat(100_000),
            &valid_pdf(),
        )
        .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("invalid multipart form data")
    );
}

#[tokio::test]
async fn read_article_reports_a_missing_article() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (status, body) = context
        .get(&format!("/article/{}/read", Uuid::now_v7()), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("article not found"));
}

#[tokio::test]
async fn update_article_is_forbidden_for_a_non_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (_, owner_token) = member_session(&context, "alice@example.com").await;
    let (_, stranger_token) = member_session(&context, "bob@example.com").await;
    let article_id = create_article_fixture(&context, &owner_token, "Private Article").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/update"),
            json!({ "title": "Stolen", "summary": "summary", "tags": "rust" }),
            Some(&stranger_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn update_article_reconciles_tags() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let article_id = create_article_fixture(&context, &token, "Retagged").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/update"),
            json!({ "title": "Retagged", "summary": "summary", "tags": "go cpp" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["data"]["article_id"].as_str(),
        Some(article_id.as_str())
    );
    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let tags: Vec<&str> = body["data"]["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .filter_map(|tag| tag["name"].as_str())
        .collect();
    assert_eq!(tags, vec!["go", "cpp"]);
}

#[tokio::test]
async fn delete_article_transfer_repoints_to_the_recycler() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let article_id = create_article_fixture(&context, &token, "Transferable").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "transfer" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
    let recycler_id = crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("user zero")
    .expect("recycler");
    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["data"]["author_id"].as_str(),
        Some(recycler_id.as_str())
    );
}

#[tokio::test]
async fn delete_article_soft_hides_the_article_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let article_id = create_article_fixture(&context, &token, "Soft Deletable").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("article not found"));
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(
        versions.len(),
        0,
        "versions hidden after article soft delete"
    );
}

#[tokio::test]
async fn delete_article_hard_cascades() {
    let context = TestCtx::new().await.expect("test context");
    let (_, owner_token) = member_session(&context, "alice@example.com").await;
    let (_, admin_token) = admin_session(&context).await;
    let article_id = create_article_fixture(&context, &owner_token, "Hard Deletable").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&admin_token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("article not found"));
}

#[tokio::test]
async fn delete_article_hard_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (_, owner_token) = member_session(&context, "alice@example.com").await;
    let article_id = create_article_fixture(&context, &owner_token, "Hard Denied").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&owner_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn undelete_soft_article_revives_the_article_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (_, owner_token) = member_session(&context, "alice@example.com").await;
    let (_, admin_token) = admin_session(&context).await;
    let article_id = create_article_fixture(&context, &owner_token, "Restorable").await;
    let (status, body) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&owner_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/undelete-soft"),
            json!({}),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("undeleted"));

    let (status, body) = context
        .get(&format!("/article/{article_id}/read"), Some(&owner_token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
}

#[tokio::test]
async fn undelete_soft_article_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (_, owner_token) = member_session(&context, "alice@example.com").await;
    let article_id = create_article_fixture(&context, &owner_token, "Restore Denied").await;
    let (status, _) = context
        .post(
            &format!("/article/{article_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&owner_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = context
        .post(
            &format!("/article/{article_id}/undelete-soft"),
            json!({}),
            Some(&owner_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn search_rejects_an_unknown_range() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (status, body) = context
        .get("/article/read?ranges=title,frobnicate", Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("unknown search range: frobnicate")
    );
}

#[tokio::test]
async fn search_rejects_from_greater_than_to() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (status, body) = context
        .get("/article/read?from=2024-01-16&to=2024-01-15", Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("from must not be greater than to")
    );
}

#[tokio::test]
async fn search_rejects_an_overlong_query() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let long_query = "a".repeat(513);
    let (status, body) = context
        .get(&format!("/article/read?q={long_query}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("search query too long (max 512 chars)")
    );
}

#[tokio::test]
async fn search_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    let (status, body) = context.get("/article/read?page=1025", Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("page exceeds max search pages")
    );
}

#[tokio::test]
async fn search_returns_hits_for_a_keyword() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;
    create_article_fixture(&context, &token, "Needle In A Haystack").await;
    let (status, body) = context
        .get(
            "/article/read?q=needle&ranges=title,summary,author_name,comment,note,tag,version_number",
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let list = body["data"]["items"].as_array().expect("article_list");
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
    assert!(
        list.iter()
            .any(|item| item["title"].as_str() == Some("<mark>Needle</mark> In A Haystack"))
    );
    assert!(list[0]["article_hits"].is_array());
}

#[tokio::test]
async fn create_article_ignores_unknown_multipart_fields() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let mut fields = article_fields("With Extra Fields", "rust");
    fields.push(("unexpected_field", "ignored value"));
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
    assert!(!body["data"]["article_id"].as_str().unwrap_or("").is_empty());
}
