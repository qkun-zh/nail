use super::context::TestCtx;
use crate::logic::authorize::{
    EntityRef, authorize, authorize_entity, authorize_entity_or, authorize_global, authorize_or,
    require_entity_readable, require_entity_visible,
};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UNDELETE_SOFT,
    PERMISSION_ARTICLE_UPDATE, PERMISSION_COMMENT_READ, PERMISSION_COMMENT_UNDELETE_SOFT,
    PERMISSION_COMMENT_UPDATE, PERMISSION_ROLE_READ, PERMISSION_TAG_READ, PERMISSION_USER_READ,
    PERMISSION_USER_UNDELETE_SOFT, PERMISSION_VERSION_READ, PERMISSION_VERSION_UNDELETE_SOFT,
    PERMISSION_VERSION_UPDATE,
};
use crate::repository::version::VersionDraft;
use database::NodeKind;

fn create_user(context: &TestCtx, email: &str) -> String {
    crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user")
}

fn create_article_fixture(context: &TestCtx, author_id: &str, title: &str) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &context.state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: "a".repeat(32),
                note: "note".to_string(),
            },
        },
    )
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn user_read_grants_admin_and_denies_member() {
    let context = TestCtx::new().await.expect("test context");
    let admin = create_user(&context, "user-zero@example.com");
    let member = create_user(&context, "alice@example.com");
    let target = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &member, "member")
        .expect("member role");

    assert!(
        authorize(
            &context.state,
            &admin,
            PERMISSION_USER_READ,
            &Resource::User(target.clone()),
        )
        .is_ok()
    );
    assert_eq!(
        authorize(
            &context.state,
            &member,
            PERMISSION_USER_READ,
            &Resource::User(target),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn owner_can_update_own_article_without_permission() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    crate::repository::role::hold_role(&context.state.database, &owner, "member").expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine");

    assert!(
        authorize(
            &context.state,
            &owner,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .is_ok()
    );
}

#[tokio::test]
async fn non_owner_without_permission_is_forbidden() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    let other = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &other, "member").expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine");

    assert_eq!(
        authorize(
            &context.state,
            &other,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn missing_article_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com");
    assert_eq!(
        authorize_or(
            &context.state,
            &actor,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article("missing".to_string()),
            "article not found",
        )
        .unwrap_err(),
        LogicError::not_found("article not found")
    );
}

#[tokio::test]
async fn authorize_article_create_on_the_virtual_desk_grants_a_member() {
    let context = TestCtx::new().await.expect("test context");
    let member = create_user(&context, "alice@example.com");
    crate::repository::role::hold_role(&context.state.database, &member, "member").expect("member");

    assert!(
        authorize(
            &context.state,
            &member,
            PERMISSION_ARTICLE_CREATE,
            &Resource::Virtual("article-create".to_string()),
        )
        .is_ok()
    );
}

#[tokio::test]
async fn authorize_article_create_on_the_virtual_desk_denies_a_non_holder() {
    let context = TestCtx::new().await.expect("test context");
    let outsider = create_user(&context, "bob@example.com");

    assert_eq!(
        authorize(
            &context.state,
            &outsider,
            PERMISSION_ARTICLE_CREATE,
            &Resource::Virtual("article-create".to_string()),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn virtual_desk_assembly_covers_the_create_and_admin_uids() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com");
    for name in ["article-create", "comment-create", "role-console"] {
        let assembly = crate::repository::authorization::assemble(
            &context.state.database,
            &actor,
            Resource::Virtual(name.to_string()),
        )
        .expect("assemble");
        let expected: cedar_policy::EntityUid =
            format!("Virtual::\"{name}\"").parse().expect("uid");
        assert_eq!(assembly.resource, expected);
    }
}

#[tokio::test]
async fn comment_author_can_update_own_comment_but_article_owner_cannot() {
    let context = TestCtx::new().await.expect("test context");
    let article_owner = create_user(&context, "alice@example.com");
    let comment_author = create_user(&context, "bob@example.com");
    let (_, version_id) = create_article_fixture(&context, &article_owner, "Mine");
    let comment_id = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &context.state.database,
        &comment_id,
        &comment_author,
        &version_id,
        "hello",
    )
    .expect("comment");

    assert!(
        authorize(
            &context.state,
            &comment_author,
            PERMISSION_COMMENT_UPDATE,
            &Resource::Comment(comment_id.clone()),
        )
        .is_ok()
    );
    assert_eq!(
        authorize(
            &context.state,
            &article_owner,
            PERMISSION_COMMENT_UPDATE,
            &Resource::Comment(comment_id),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}
#[tokio::test]
async fn role_grant_authorizes_any_article() {
    let context = TestCtx::new().await.expect("test context");
    let editor = create_user(&context, "alice@example.com");
    let owner = create_user(&context, "bob@example.com");
    crate::repository::role::create_role(&context.state.database, "editor").expect("role");
    crate::repository::role::grant_permission_to_role(
        &context.state.database,
        "editor",
        PERMISSION_ARTICLE_UPDATE,
    )
    .expect("grant");
    crate::repository::role::hold_role(&context.state.database, &editor, "editor")
        .expect("hold editor");
    let (article_id, _) = create_article_fixture(&context, &owner, "Global");

    assert!(
        authorize(
            &context.state,
            &editor,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .is_ok()
    );
}

#[tokio::test]
async fn role_read_on_a_non_role_resource_is_denied() {
    let context = TestCtx::new().await.expect("test context");
    let member = create_user(&context, "alice@example.com");
    crate::repository::role::hold_role(&context.state.database, &member, "member").expect("member");

    assert_eq!(
        authorize(
            &context.state,
            &member,
            PERMISSION_USER_READ,
            &Resource::Virtual("users".to_string()),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn role_resource_assembly_covers_role_uids() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com");
    for name in ["admin", "member", "recycler"] {
        let assembly = crate::repository::authorization::assemble(
            &context.state.database,
            &actor,
            Resource::Role(name.to_string()),
        )
        .expect("assemble");
        let expected: cedar_policy::EntityUid = format!("Role::\"{name}\"").parse().expect("uid");
        assert_eq!(assembly.resource, expected);
    }
}

#[tokio::test]
async fn version_owner_is_the_article_owner() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    let other = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &other, "member").expect("member");
    let (_, version_id) = create_article_fixture(&context, &owner, "Versioned");

    assert!(
        authorize(
            &context.state,
            &owner,
            PERMISSION_VERSION_UPDATE,
            &Resource::Version(version_id.clone()),
        )
        .is_ok()
    );
    assert_eq!(
        authorize(
            &context.state,
            &other,
            PERMISSION_VERSION_UPDATE,
            &Resource::Version(version_id),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn member_can_read_articles_and_versions_via_role_grant() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    let member = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &member, "member").expect("member");
    let (article_id, version_id) = create_article_fixture(&context, &owner, "Open");

    assert!(
        authorize(
            &context.state,
            &member,
            PERMISSION_ARTICLE_READ,
            &Resource::Article(article_id),
        )
        .is_ok()
    );
    assert!(
        authorize(
            &context.state,
            &member,
            PERMISSION_VERSION_READ,
            &Resource::Version(version_id),
        )
        .is_ok()
    );
}

#[test]
fn entity_ref_mapping_is_canonical() {
    assert_eq!(
        EntityRef::Article("a1").resource(),
        Resource::Article("a1".to_string())
    );
    assert_eq!(
        EntityRef::Article("a1").not_found_message(),
        "article not found"
    );
    assert_eq!(EntityRef::Article("a1").id(), "a1");
    assert_eq!(
        EntityRef::Article("a1").visibility(),
        Some((NodeKind::Article, PERMISSION_ARTICLE_UNDELETE_SOFT))
    );
    assert_eq!(
        EntityRef::Article("a1").read_permission(),
        PERMISSION_ARTICLE_READ
    );

    assert_eq!(
        EntityRef::Version("v1").resource(),
        Resource::Version("v1".to_string())
    );
    assert_eq!(
        EntityRef::Version("v1").not_found_message(),
        "version not found"
    );
    assert_eq!(
        EntityRef::Version("v1").visibility(),
        Some((NodeKind::Version, PERMISSION_VERSION_UNDELETE_SOFT))
    );
    assert_eq!(
        EntityRef::Version("v1").read_permission(),
        PERMISSION_VERSION_READ
    );

    assert_eq!(
        EntityRef::Comment("c1").resource(),
        Resource::Comment("c1".to_string())
    );
    assert_eq!(
        EntityRef::Comment("c1").not_found_message(),
        "comment not found"
    );
    assert_eq!(
        EntityRef::Comment("c1").visibility(),
        Some((NodeKind::Comment, PERMISSION_COMMENT_UNDELETE_SOFT))
    );
    assert_eq!(
        EntityRef::Comment("c1").read_permission(),
        PERMISSION_COMMENT_READ
    );

    assert_eq!(
        EntityRef::User("u1").resource(),
        Resource::User("u1".to_string())
    );
    assert_eq!(EntityRef::User("u1").not_found_message(), "user not found");
    assert_eq!(
        EntityRef::User("u1").visibility(),
        Some((NodeKind::User, PERMISSION_USER_UNDELETE_SOFT))
    );
    assert_eq!(
        EntityRef::User("u1").read_permission(),
        PERMISSION_USER_READ
    );

    assert_eq!(
        EntityRef::Tag("t1").resource(),
        Resource::Tag("t1".to_string())
    );
    assert_eq!(EntityRef::Tag("t1").not_found_message(), "tag not found");
    assert_eq!(EntityRef::Tag("t1").visibility(), None);
    assert_eq!(EntityRef::Tag("t1").read_permission(), PERMISSION_TAG_READ);

    assert_eq!(
        EntityRef::Role("r1").resource(),
        Resource::Role("r1".to_string())
    );
    assert_eq!(EntityRef::Role("r1").not_found_message(), "role not found");
    assert_eq!(EntityRef::Role("r1").visibility(), None);
    assert_eq!(
        EntityRef::Role("r1").read_permission(),
        PERMISSION_ROLE_READ
    );
}

#[tokio::test]
async fn authorize_global_grants_member_and_denies_outsider() {
    let context = TestCtx::new().await.expect("test context");
    let member = create_user(&context, "alice@example.com");
    let outsider = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &member, "member").expect("member");

    assert!(authorize_global(&context.state, &member, PERMISSION_ARTICLE_CREATE).is_ok());
    assert_eq!(
        authorize_global(&context.state, &outsider, PERMISSION_ARTICLE_CREATE).unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn authorize_entity_matches_plain_authorize() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    let other = create_user(&context, "bob@example.com");
    crate::repository::role::hold_role(&context.state.database, &owner, "member").expect("member");
    crate::repository::role::hold_role(&context.state.database, &other, "member").expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Mine");

    assert!(
        authorize_entity(
            &context.state,
            &owner,
            PERMISSION_ARTICLE_UPDATE,
            EntityRef::Article(&article_id),
        )
        .is_ok()
    );
    assert_eq!(
        authorize_entity(
            &context.state,
            &other,
            PERMISSION_ARTICLE_UPDATE,
            EntityRef::Article(&article_id),
        )
        .unwrap_err(),
        LogicError::forbidden("you are denied")
    );
}

#[tokio::test]
async fn authorize_entity_or_reports_canonical_message() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com");
    assert_eq!(
        authorize_entity_or(
            &context.state,
            &actor,
            PERMISSION_ARTICLE_UPDATE,
            EntityRef::Article("missing"),
        )
        .unwrap_err(),
        LogicError::not_found("article not found")
    );
}

#[tokio::test]
async fn require_entity_readable_hides_soft_deleted_article() {
    let context = TestCtx::new().await.expect("test context");
    let owner = create_user(&context, "alice@example.com");
    crate::repository::role::hold_role(&context.state.database, &owner, "member").expect("member");
    let (article_id, _) = create_article_fixture(&context, &owner, "Hidden");

    assert!(
        require_entity_readable(&context.state, &owner, EntityRef::Article(&article_id)).is_ok()
    );
    crate::repository::delete::soft_delete_article(&context.state.database, &article_id)
        .expect("soft delete");
    assert_eq!(
        require_entity_readable(&context.state, &owner, EntityRef::Article(&article_id))
            .unwrap_err(),
        LogicError::not_found("article not found")
    );
    let admin = crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &nail_common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("lookup user zero")
    .expect("seeded user zero");
    assert!(
        require_entity_readable(&context.state, &admin, EntityRef::Article(&article_id)).is_ok()
    );
}

#[tokio::test]
async fn require_entity_visible_is_noop_without_lifecycle() {
    let context = TestCtx::new().await.expect("test context");
    let actor = create_user(&context, "alice@example.com");
    assert!(require_entity_visible(&context.state, &actor, EntityRef::Tag("missing")).is_ok());
    assert!(require_entity_visible(&context.state, &actor, EntityRef::Role("missing")).is_ok());
}
