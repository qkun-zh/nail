use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{
    PERMISSION_ARTICLE_DELETE, PERMISSION_ARTICLE_UPDATE, PERMISSION_USER_READ,
};
use crate::repository::version::VersionDraft;

async fn create_user(context: &TestCtx, email: &str) -> String {
    crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_article_fixture(
    context: &TestCtx,
    author_id: &str,
    title: &str,
) -> (String, String) {
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
                content_hash: "a".repeat(32),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn require_permission_grants_admin_and_denies_member() {
    let context = TestCtx::new().await.expect("test context");
    let admin = create_user(&context, "user-zero@example.com").await;
    let member = create_user(&context, "alice@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &member, "member")
        .await
        .expect("member role");

    assert!(crate::logic::authorize::require_permission(&context.state, &admin, PERMISSION_USER_READ)
        .await
        .is_ok());
    assert_eq!(
        crate::logic::authorize::require_permission(&context.state, &member, PERMISSION_USER_READ)
            .await
            .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn owner_can_update_own_article_without_permission() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &owner, "member")
        .await
        .expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine").await;

    assert!(crate::logic::authorize::require_owner_or_permission_for_article(
        &context.state,
        &owner,
        &article_id,
        PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .is_ok());
}

#[tokio::test]
async fn non_owner_without_permission_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    let other = create_user(&context, "bob@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &other, "member")
        .await
        .expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine").await;

    assert_eq!(
        crate::logic::authorize::require_owner_or_permission_for_article(
            &context.state,
            &other,
            &article_id,
            PERMISSION_ARTICLE_UPDATE,
        )
        .await
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn missing_article_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com").await;
    assert_eq!(
        crate::logic::authorize::require_owner_or_permission_for_article(
            &context.state,
            &actor,
            "missing",
            PERMISSION_ARTICLE_UPDATE,
        )
        .await
        .unwrap_err(),
        LogicError::not_found("article not found")
    );
}

#[tokio::test]
async fn is_article_author_is_true_for_owner_and_article_update_holder() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    let admin = create_user(&context, "user-zero@example.com").await;
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine").await;

    assert!(
        crate::logic::authorize::is_article_author(&context.state, &owner, &article_id)
            .await
            .expect("owner check")
    );
    assert!(
        crate::logic::authorize::is_article_author(&context.state, &admin, &article_id)
            .await
            .expect("admin check")
    );
    let stranger = create_user(&context, "bob@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &stranger, "member")
        .await
        .expect("member");
    assert!(
        !crate::logic::authorize::is_article_author(&context.state, &stranger, &article_id)
            .await
            .expect("stranger check")
    );
    let _ = PERMISSION_ARTICLE_DELETE;
}
