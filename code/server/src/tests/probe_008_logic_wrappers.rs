use super::context::TestCtx;
use crate::logic::authorize::{authorize_anonymous, authorize_or, require_visible_if_soft_deleted};
use crate::repository::authorization::Resource;
use crate::repository::delete::soft_delete_article;
use database::NodeKind;

#[tokio::test]
async fn anonymous_user_create_allow() {
    let ctx = TestCtx::new().await.expect("ctx");
    assert!(
        authorize_anonymous(&ctx.state, "User::Create", &Resource::Virtual("any".into())).is_ok()
    );
}

#[tokio::test]
async fn authorize_or_rewrites_notfound_msg() {
    let ctx = TestCtx::new().await.expect("ctx");
    let actor = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"zz@ex.com").expect("hash"),
    )
    .expect("user");
    let err = authorize_or(
        &ctx.state,
        &actor,
        "Article::Read",
        &Resource::Article("nope".into()),
        "article not found",
    )
    .unwrap_err();
    assert_eq!(
        err,
        crate::logic::error::LogicError::not_found("article not found")
    );
}

#[tokio::test]
async fn soft_deleted_visibility_gated() {
    let ctx = TestCtx::new().await.expect("ctx");
    let owner = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"vis@ex.com").expect("hash"),
    )
    .expect("user");
    let aid = uuid::Uuid::now_v7().to_string();
    let vid = uuid::Uuid::now_v7().to_string();
    crate::repository::article::create_article(
        &ctx.state.database,
        &crate::repository::article::ArticleDraft {
            article_id: aid.clone(),
            author_id: owner.clone(),
            title: "t".into(),
            summary: "s".into(),
            tags: vec![],
            first_version: crate::repository::version::VersionDraft {
                version_id: vid,
                version_number: "1.0.0".into(),
                content_hash: "a".repeat(32),
                note: "n".into(),
            },
        },
    )
    .expect("art");
    soft_delete_article(&ctx.state.database, &aid).expect("soft delete");
    // actor without undelete perm sees not_found
    let viewer = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"viewer@ex.com").expect("hash"),
    )
    .expect("user");
    let res = require_visible_if_soft_deleted(
        &ctx.state,
        &viewer,
        NodeKind::Article,
        &aid,
        "Article::Undelete::Soft",
        &Resource::Article(aid.clone()),
        "article not found",
    );
    assert!(res.is_err());
}
