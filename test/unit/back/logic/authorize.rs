use super::context::TestCtx;
use crate::logic::authorize::{authorize, authorize_create, authorize_or, is_author};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UPDATE,
    PERMISSION_COMMENT_UPDATE, PERMISSION_USER_READ, PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UPDATE,
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

fn admin_console() -> Resource {
    Resource::System("admin-console".to_string())
}

#[tokio::test]
async fn admin_console_authorize_grants_admin_and_denies_member() {
    let context = TestCtx::new().await.expect("test context");
    let admin = create_user(&context, "user-zero@example.com").await;
    let member = create_user(&context, "alice@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &member, "member")
        .await
        .expect("member role");

    assert!(
        authorize(&context.state, &admin, PERMISSION_USER_READ, &admin_console())
            .await
            .is_ok()
    );
    assert_eq!(
        authorize(&context.state, &member, PERMISSION_USER_READ, &admin_console())
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

    assert!(
        authorize(
            &context.state,
            &owner,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .await
        .is_ok()
    );
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
        authorize(
            &context.state,
            &other,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
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
        authorize_or(
            &context.state,
            &actor,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article("missing".to_string()),
            "article not found",
        )
        .await
        .unwrap_err(),
        LogicError::not_found("article not found")
    );
}

#[tokio::test]
async fn authorize_create_grants_a_member_article_create() {
    let context = TestCtx::new().await.expect("test context");
    let member = create_user(&context, "alice@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &member, "member")
        .await
        .expect("member");

    assert!(
        authorize_create(&context.state, &member, PERMISSION_ARTICLE_CREATE)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn is_author_is_true_for_owner_and_article_update_holder() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    let admin = create_user(&context, "user-zero@example.com").await;
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine").await;

    assert!(is_author(&context.state, &owner, Some(&article_id), None, None)
        .await
        .expect("owner check"));
    assert!(is_author(&context.state, &admin, Some(&article_id), None, None)
        .await
        .expect("admin check"));

    let stranger = create_user(&context, "bob@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &stranger, "member")
        .await
        .expect("member");
    assert!(!is_author(&context.state, &stranger, Some(&article_id), None, None)
        .await
        .expect("stranger check"));
}

#[tokio::test]
async fn is_author_rejects_zero_or_multiple_ids() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com").await;

    assert_eq!(
        is_author(&context.state, &actor, None, None, None)
            .await
            .unwrap_err(),
        LogicError::bad_request("exactly one of article_id, version_id or comment_id is required")
    );

    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        is_author(&context.state, &actor, Some(&article_id), Some(&version_id), None)
            .await
            .unwrap_err(),
        LogicError::bad_request("exactly one of article_id, version_id or comment_id is required")
    );
}

#[tokio::test]
async fn comment_author_can_update_own_comment_but_article_owner_cannot() {
    let context = TestCtx::new().await.expect("test context");
    let article_owner = create_user(&context, "alice@example.com").await;
    let comment_author = create_user(&context, "bob@example.com").await;
    let (_, version_id) = create_article_fixture(&context, &article_owner, "Mine").await;
    let comment_id = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &context.state.graph,
        &comment_id,
        &comment_author,
        &version_id,
        "hello",
    )
    .await
    .expect("comment");

    assert!(
        authorize(
            &context.state,
            &comment_author,
            PERMISSION_COMMENT_UPDATE,
            &Resource::Comment(comment_id.clone()),
        )
        .await
        .is_ok()
    );
    assert_eq!(
        authorize(
            &context.state,
            &article_owner,
            PERMISSION_COMMENT_UPDATE,
            &Resource::Comment(comment_id),
        )
        .await
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}
#[tokio::test]
async fn global_role_grants_permission_on_any_article() {
    let context = TestCtx::new().await.expect("test context");
    let editor = create_user(&context, "alice@example.com").await;
    let owner = create_user(&context, "bob@example.com").await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("role");
    crate::repository::role::grant_permission_to_role(
        &context.state.graph,
        "editor",
        PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("grant");
    crate::repository::role::hold_role(&context.state.graph, &editor, "editor")
        .await
        .expect("hold editor");
    let (article_id, _) = create_article_fixture(&context, &owner, "Global").await;

    assert!(
        authorize(
            &context.state,
            &editor,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn scoped_role_denies_when_tags_do_not_intersect() {
    let context = TestCtx::new().await.expect("test context");
    let editor = create_user(&context, "alice@example.com").await;
    let owner = create_user(&context, "bob@example.com").await;
    crate::repository::role::create_role(&context.state.graph, "editor")
        .await
        .expect("role");
    crate::repository::role::grant_permission_to_role(
        &context.state.graph,
        "editor",
        PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("grant");
    crate::repository::role::apply_tag_to_role(&context.state.graph, "editor", "#go")
        .await
        .expect("scope");
    crate::repository::role::hold_role(&context.state.graph, &editor, "editor")
        .await
        .expect("hold editor");
    let (article_id, _) = create_article_fixture(&context, &owner, "Scoped").await;

    assert_eq!(
        authorize(
            &context.state,
            &editor,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .await
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn admin_console_action_on_a_non_admin_console_resource_is_denied() {
    let context = TestCtx::new().await.expect("test context");
    let member = create_user(&context, "alice@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &member, "member")
        .await
        .expect("member");

    assert_eq!(
        authorize(
            &context.state,
            &member,
            PERMISSION_USER_READ,
            &Resource::System("users".to_string()),
        )
        .await
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn authorize_create_denies_a_principal_without_article_create() {
    let context = TestCtx::new().await.expect("test context");
    let outsider = create_user(&context, "bob@example.com").await;

    assert_eq!(
        authorize_create(&context.state, &outsider, PERMISSION_ARTICLE_CREATE)
            .await
            .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn version_owner_is_the_article_owner() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    let other = create_user(&context, "bob@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &other, "member")
        .await
        .expect("member");
    let (_, version_id) = create_article_fixture(&context, &owner, "Versioned").await;

    assert!(
        authorize(
            &context.state,
            &owner,
            PERMISSION_VERSION_UPDATE,
            &Resource::Version(version_id.clone()),
        )
        .await
        .is_ok()
    );
    assert_eq!(
        authorize(
            &context.state,
            &other,
            PERMISSION_VERSION_UPDATE,
            &Resource::Version(version_id),
        )
        .await
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn read_open_allows_an_authenticated_non_owner_to_read() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com").await;
    let member = create_user(&context, "bob@example.com").await;
    crate::repository::role::hold_role(&context.state.graph, &member, "member")
        .await
        .expect("member");
    let (article_id, version_id) = create_article_fixture(&context, &owner, "Open").await;

    assert!(
        authorize(
            &context.state,
            &member,
            PERMISSION_ARTICLE_READ,
            &Resource::Article(article_id),
        )
        .await
        .is_ok()
    );
    assert!(
        authorize(
            &context.state,
            &member,
            PERMISSION_VERSION_READ,
            &Resource::Version(version_id),
        )
        .await
        .is_ok()
    );
}

