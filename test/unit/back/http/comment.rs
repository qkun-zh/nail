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
        &context.state.database,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
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
    member_session(context, "user-zero@example.com").await
}

async fn plain_session(context: &TestCtx, email: &str) -> (String, String) {
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

async fn version_fixture(context: &TestCtx, author_id: &str) -> String {
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    create_article(
        &context.state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Article {article_id}"),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
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
            &format!("/version/{version_id}/comment/create"),
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
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "top" }),
            Some(&token),
        )
        .await;
    let top_id = created["data"]["comment_id"].as_str().expect("top id");

    let (status, body) = context
        .post(
            &format!("/comment/{top_id}/reply/create"),
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
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;

    let (status, body) = context
        .get(
            &format!("/version/{version_id}/comment/read?page=1&limit=8"),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"]["has_next"].as_bool(), Some(false));
    assert_eq!(body["data"]["total"].as_u64(), Some(1));
    assert!(body["data"]["items"][0]["user_name"].as_str().is_some());
}

#[tokio::test]
async fn read_comments_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (status, body) = context
        .get(
            &format!("/version/{version_id}/comment/read?page=1025"),
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
async fn read_comments_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (_, outsider) = plain_session(&context, "bob@example.com").await;
    let (status, body) = context
        .get(
            &format!("/version/{version_id}/comment/read"),
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_comment_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (status, create_body) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {create_body}");
    let comment_id = create_body["data"]["comment_id"]
        .as_str()
        .expect("comment id");

    let (_, outsider) = plain_session(&context, "bob@example.com").await;
    let (status, body) = context
        .get(&format!("/comment/{comment_id}/read"), Some(&outsider))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_comment_children_requires_a_read_grant() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (status, create_body) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "top" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {create_body}");
    let comment_id = create_body["data"]["comment_id"]
        .as_str()
        .expect("comment id");

    let (_, outsider) = plain_session(&context, "bob@example.com").await;
    let (status, body) = context
        .get(
            &format!("/comment/{comment_id}/reply/read"),
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
async fn read_comment_children_returns_the_replies_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (status, create_body) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "top" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {create_body}");
    let comment_id = create_body["data"]["comment_id"]
        .as_str()
        .expect("comment id");

    let (status, reply_body) = context
        .post(
            &format!("/comment/{comment_id}/reply/create"),
            json!({ "content": "a reply" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "body: {reply_body}");

    let (status, body) = context
        .get(&format!("/comment/{comment_id}/reply/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let comments = body["data"]["items"].as_array().expect("comments");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["content"].as_str(), Some("a reply"));
}

#[tokio::test]
async fn read_comment_children_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, create_body) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "root" }),
            Some(&token),
        )
        .await;
    let comment_id = create_body["data"]["comment_id"].as_str().unwrap();

    let (status, body) = context
        .get(
            &format!("/comment/{comment_id}/reply/read?page=1025"),
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
async fn update_comment_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (_, stranger_token) = member_session(&context, "bob@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, _) = context
        .post(
            &format!("/comment/{comment_id}/update"),
            json!({ "content": "stolen" }),
            Some(&stranger_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/update"),
            json!({ "content": "edited" }),
            Some(&token),
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
            &format!("/version/{version_id}/comment/create"),
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
async fn delete_comment_soft_hides_the_comment_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));

    let (status, body) = context
        .get(&format!("/comment/{comment_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("comment not found"));
}

#[tokio::test]
async fn undelete_soft_comment_revives_the_comment_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (_, admin_token) = admin_session(&context).await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/undelete-soft"),
            json!({}),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("undeleted"));

    let (status, body) = context
        .get(&format!("/comment/{comment_id}/read"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
}

#[tokio::test]
async fn undelete_soft_comment_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, _) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "soft" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/undelete-soft"),
            json!({}),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn delete_comment_requires_a_mode_over_http() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
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
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}

#[tokio::test]
async fn create_comment_reports_a_missing_version() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            &format!("/version/{}/comment/create", Uuid::now_v7()),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("comment target not found (the version may have been removed)")
    );
}

#[tokio::test]
async fn create_reply_reports_a_missing_parent() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            &format!("/comment/{}/reply/create", Uuid::now_v7()),
            json!({ "content": "reply" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("reply target not found (the parent comment may have been removed)")
    );
}

#[tokio::test]
async fn create_reply_reports_a_thread_too_deep() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;

    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "root" }),
            Some(&token),
        )
        .await;
    let mut parent_id = created["data"]["comment_id"]
        .as_str()
        .expect("root id")
        .to_string();

    for _ in 0..64 {
        let (status, body) = context
            .post(
                &format!("/comment/{parent_id}/reply/create"),
                json!({ "content": "reply" }),
                Some(&token),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "body: {body}");
        parent_id = body["data"]["comment_id"]
            .as_str()
            .expect("reply id")
            .to_string();
    }

    let (status, body) = context
        .post(
            &format!("/comment/{parent_id}/reply/create"),
            json!({ "content": "overflow" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(
        body["message"].as_str(),
        Some("comment thread too deep (max 64 reply layers)")
    );
}

#[tokio::test]
async fn update_comment_reports_a_missing_comment() {
    let context = TestCtx::new().await.expect("test context");
    let (_, token) = member_session(&context, "alice@example.com").await;

    let (status, body) = context
        .post(
            &format!("/comment/{}/update", Uuid::now_v7()),
            json!({ "content": "edited" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("comment not found"));
}

#[tokio::test]
async fn delete_comment_hard_removes_the_comment() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (_, admin_token) = admin_session(&context).await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&admin_token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("deleted"));
}

#[tokio::test]
async fn delete_comment_hard_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}

#[tokio::test]
async fn delete_comment_is_forbidden_for_a_non_owner() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, token) = member_session(&context, "alice@example.com").await;
    let (_, stranger_token) = member_session(&context, "bob@example.com").await;
    let version_id = version_fixture(&context, &user_id).await;
    let (_, created) = context
        .post(
            &format!("/version/{version_id}/comment/create"),
            json!({ "content": "hello" }),
            Some(&token),
        )
        .await;
    let comment_id = created["data"]["comment_id"].as_str().expect("comment id");

    let (status, body) = context
        .post(
            &format!("/comment/{comment_id}/delete"),
            json!({ "mode": "hard" }),
            Some(&stranger_token),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"].as_str(), Some("you are denied"));
}
