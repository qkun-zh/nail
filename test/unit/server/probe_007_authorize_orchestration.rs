use super::context::TestCtx;
use crate::infrastructure::authorizer::AuthorizationError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::authorization::Resource;
use crate::repository::version::VersionDraft;

fn article_fixture(ctx: &TestCtx, author: &str) -> String {
    let aid = uuid::Uuid::now_v7().to_string();
    let vid = uuid::Uuid::now_v7().to_string();
    create_article(
        &ctx.state.database,
        &ArticleDraft {
            article_id: aid.clone(),
            author_id: author.to_string(),
            title: "t".into(),
            summary: "s".into(),
            tags: vec![],
            first_version: VersionDraft {
                version_id: vid,
                version_number: "1.0.0".into(),
                content_hash: "a".repeat(32),
                note: "n".into(),
            },
        },
    )
    .expect("article");
    aid
}

#[tokio::test]
async fn member_read_via_role_grant_allow() {
    let ctx = TestCtx::new().await.expect("ctx");
    let owner = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"own@ex.com").expect("hash"),
    )
    .expect("user");
    let member = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"mem@ex.com").expect("hash"),
    )
    .expect("user");
    crate::repository::role::hold_role(&ctx.state.database, &member, "member").expect("hold");
    let aid = article_fixture(&ctx, &owner);
    assert!(
        ctx.state
            .authorizer
            .authorize(&member, "Article::Read", &Resource::Article(aid))
            .is_ok()
    );
}

#[tokio::test]
async fn non_owner_without_grant_deny() {
    let ctx = TestCtx::new().await.expect("ctx");
    let owner = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"o2@ex.com").expect("hash"),
    )
    .expect("user");
    let outsider = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"out@ex.com").expect("hash"),
    )
    .expect("user");
    crate::repository::role::hold_role(&ctx.state.database, &outsider, "member").expect("hold");
    // outsider has Article::Read via member? Actually member has read in seed? Need a clean member without read – create fresh role
    // For baseline, ensure outsider holds no role that grants read: use fresh user with no roles
    let no_role = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"noread@ex.com").expect("hash"),
    )
    .expect("user");
    let aid = article_fixture(&ctx, &owner);
    let err = ctx
        .state
        .authorizer
        .authorize(&no_role, "Article::Update", &Resource::Article(aid))
        .unwrap_err();
    assert!(matches!(err, AuthorizationError::Denied));
}

#[tokio::test]
async fn missing_resource_notfound() {
    let ctx = TestCtx::new().await.expect("ctx");
    let actor = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"act@ex.com").expect("hash"),
    )
    .expect("user");
    let err = ctx
        .state
        .authorizer
        .authorize(
            &actor,
            "Article::Read",
            &Resource::Article("missing".into()),
        )
        .unwrap_err();
    assert!(matches!(err, AuthorizationError::ResourceNotFound));
}

#[tokio::test]
async fn virtual_create_allow_for_member() {
    let ctx = TestCtx::new().await.expect("ctx");
    let member = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"vm@ex.com").expect("hash"),
    )
    .expect("user");
    crate::repository::role::hold_role(&ctx.state.database, &member, "member").expect("hold");
    assert!(
        ctx.state
            .authorizer
            .authorize(&member, "Article::Create", &Resource::Virtual("any".into()))
            .is_ok()
    );
}
