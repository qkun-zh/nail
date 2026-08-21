use super::context::TestCtx;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::authorization::{Resource, assemble_resource};
use crate::repository::version::VersionDraft;

fn article_fixture(ctx: &TestCtx, author: &str) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &ctx.state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author.to_string(),
            title: "t".into(),
            summary: "s".into(),
            tags: vec![],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".into(),
                content_hash: "a".repeat(32),
                note: "n".into(),
            },
        },
    )
    .expect("article");
    (article_id, version_id)
}

#[tokio::test]
async fn article_success_and_missing_notfound() {
    let ctx = TestCtx::new().await.expect("ctx");
    let author = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"a1@ex.com").expect("hash"),
    )
    .expect("user");
    let (aid, _) = article_fixture(&ctx, &author);
    let (uid, ents) =
        assemble_resource(&ctx.state.database, Resource::Article(aid.clone())).expect("assemble");
    assert_eq!(uid.to_string(), format!("Article::\"{aid}\""));
    assert_eq!(ents.len(), 1);
    let err =
        assemble_resource(&ctx.state.database, Resource::Article("missing".into())).unwrap_err();
    assert!(matches!(
        err,
        crate::repository::authorization::AssemblyError::ResourceNotFound
    ));
}

#[tokio::test]
async fn version_chain_builds_two_entities() {
    let ctx = TestCtx::new().await.expect("ctx");
    let author = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"a2@ex.com").expect("hash"),
    )
    .expect("user");
    let (_, vid) = article_fixture(&ctx, &author);
    let (uid, ents) =
        assemble_resource(&ctx.state.database, Resource::Version(vid.clone())).expect("ok");
    assert_eq!(uid.to_string(), format!("Version::\"{vid}\""));
    assert_eq!(ents.len(), 2);
}

#[tokio::test]
async fn comment_chain_builds_three_entities() {
    let ctx = TestCtx::new().await.expect("ctx");
    let author = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"a3@ex.com").expect("hash"),
    )
    .expect("user");
    let (_, vid) = article_fixture(&ctx, &author);
    let cid = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &ctx.state.database,
        &cid,
        &author,
        &vid,
        "hi",
    )
    .expect("comment");
    let (uid, ents) =
        assemble_resource(&ctx.state.database, Resource::Comment(cid.clone())).expect("ok");
    assert_eq!(uid.to_string(), format!("Comment::\"{cid}\""));
    assert_eq!(ents.len(), 3);
}

#[tokio::test]
async fn virtual_always_exists() {
    let ctx = TestCtx::new().await.expect("ctx");
    let (uid, ents) =
        assemble_resource(&ctx.state.database, Resource::Virtual("any".into())).expect("ok");
    assert_eq!(uid.to_string(), "Virtual::\"any\"");
    assert_eq!(ents.len(), 1);
}
