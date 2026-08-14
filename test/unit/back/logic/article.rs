use super::context::{TestCtx, unique_pdf, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

#[tokio::test]
async fn create_article_writes_the_article_and_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let upload = context.upload(&valid_pdf());

    let (article_id, version_id) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "My Article",
            summary: "A summary.",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload,
        },
    )
    .await
    .expect("create article");

    assert!(!article_id.is_empty());
    assert!(!version_id.is_empty());
    assert!(
        crate::repository::article::read_article(&context.state.graph, &article_id)
            .await
            .expect("read")
            .is_some()
    );
}

#[tokio::test]
async fn create_article_requires_article_create_permission() {
    let context = TestCtx::new().await.expect("test context");
    let actor = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Title",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn create_article_rejects_an_empty_title() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("text cannot be empty"));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_title() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Duplicated",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("first")),
        },
    )
    .await
    .expect("first");

    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Duplicated",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("second")),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("title already exists"));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_content_hash() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let pdf = valid_pdf();
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "First",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&pdf),
        },
    )
    .await
    .expect("first");

    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Second",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&pdf),
        },
    )
    .await
    .unwrap_err();
    let message = error.to_string();
    assert!(
        message.starts_with("identical PDF already exists"),
        "{message}"
    );
}

#[tokio::test]
async fn read_article_returns_detail_and_is_author() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Titled",
            summary: "Summary",
            tags: "#rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let data = crate::logic::article::read_article(&context.state, &actor, &article_id, true)
        .await
        .expect("read");
    assert_eq!(data.title, "Titled");
    assert_eq!(data.is_author, Some(true));
}

#[tokio::test]
async fn read_article_missing_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let error = crate::logic::article::read_article(&context.state, &actor, "missing", false)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::not_found("article not found"));
}
