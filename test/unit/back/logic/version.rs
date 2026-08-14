use super::context::{TestCtx, unique_pdf};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

async fn article_fixture(context: &TestCtx, author_id: &str, title: &str) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &context.state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&unique_pdf(title)),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[test]
fn validate_version_canonicalizes_valid_semver() {
    assert_eq!(
        crate::logic::version::validate_version("1.0.0").expect("valid"),
        "1.0.0"
    );
    assert_eq!(
        crate::logic::version::validate_version(" 1.2.3 ").expect("valid"),
        "1.2.3"
    );
    assert!(matches!(
        crate::logic::version::validate_version("not-semver"),
        Err(LogicError::BadRequest(_))
    ));
}

#[tokio::test]
async fn create_version_requires_a_strictly_greater_semver() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Article").await;

    let error = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "1.0.0",
        "note",
        context.upload(&unique_pdf("version-next")),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("new version must be strictly greater than the latest version")
    );
}

#[tokio::test]
async fn create_version_writes_a_new_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Article").await;

    let version_id = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "1.1.0",
        "next note",
        context.upload(&unique_pdf("version-1.1.0")),
    )
    .await
    .expect("create version");

    let entry = crate::repository::version::read_version(&context.state.graph, &version_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.version_number, "1.1.0");
    assert_eq!(entry.note, "next note");
}

#[tokio::test]
async fn read_version_cross_checks_the_parent_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;
    let (other_article, _) = article_fixture(&context, &actor, "Other").await;

    let data = crate::logic::version::read_version(
        &context.state,
        &actor,
        &version_id,
        Some(&article_id),
        false,
    )
    .await
    .expect("read");
    assert_eq!(data.version, "1.0.0");

    let error = crate::logic::version::read_version(
        &context.state,
        &actor,
        &version_id,
        Some(&other_article),
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::not_found("version not found"));
}

#[tokio::test]
async fn delete_version_hard_removes_the_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;

    let data = crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("delete");
    assert_eq!(data.version_id, version_id);
    assert!(crate::repository::version::read_version(&context.state.graph, &version_id)
        .await
        .expect("read")
        .is_none());
    let _ = article_id;
}

#[tokio::test]
async fn delete_version_rejects_transfer_mode() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;
    let error = crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Transfer),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("version delete only supports mode \"hard\"")
    );
}
