use super::context::{TestCtx, valid_pdf};
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

async fn admin(context: &TestCtx) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &context.state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn article_with_tags(context: &TestCtx, actor: &str, tags: &str) -> String {
    let names: Vec<&str> = tags
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    context.seed_tags(&names).await;
    let upload = context.upload(&valid_pdf());
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        actor,
        crate::logic::article::ArticleCreateInput {
            title: "Tagged Article",
            summary: "A summary.",
            tags,
            version: "1.0.0",
            note: "note",
            upload,
        },
    )
    .await
    .expect("create article");
    article_id
}

async fn seeded_tag_id(context: &TestCtx, name: &str) -> String {
    context.seed_tags(&[name]).await;
    crate::repository::tag::read_tag_by_name(&context.state.graph, name)
        .await
        .expect("read tag")
        .expect("tag exists")
        .id
}

#[tokio::test]
async fn apply_tag_links_article_and_tag() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    crate::logic::tag::apply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("apply");
    let articles = crate::repository::tag::read_tag_articles(&context.state.graph, &tag_id)
        .await
        .expect("read articles");
    assert!(articles.contains(&article_id));
}

#[tokio::test]
async fn apply_tag_is_idempotent() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    crate::logic::tag::apply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("apply");
    crate::logic::tag::apply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("apply again");
    let articles = crate::repository::tag::read_tag_articles(&context.state.graph, &tag_id)
        .await
        .expect("read articles");
    assert_eq!(articles.iter().filter(|id| **id == article_id).count(), 1);
}

#[tokio::test]
async fn unapply_tag_removes_the_link() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    crate::logic::tag::apply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("apply");
    crate::logic::tag::unapply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("unapply");
    let articles = crate::repository::tag::read_tag_articles(&context.state.graph, &tag_id)
        .await
        .expect("read articles");
    assert!(!articles.contains(&article_id));
}

#[tokio::test]
async fn unapply_tag_when_not_applied_is_a_no_op() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    crate::logic::tag::unapply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .expect("unapply");
    let articles = crate::repository::tag::read_tag_articles(&context.state.graph, &tag_id)
        .await
        .expect("read articles");
    assert!(!articles.contains(&article_id));
}

#[tokio::test]
async fn apply_tag_to_a_missing_article_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    let err = crate::logic::tag::apply_tag(&context.state, &actor, "missing", &tag_id)
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::not_found("article not found"));
}

#[tokio::test]
async fn apply_tag_to_a_missing_tag_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = admin(&context).await;
    let article_id = article_with_tags(&context, &actor, "rust").await;

    let err = crate::logic::tag::apply_tag(&context.state, &actor, &article_id, "missing")
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::not_found("tag not found"));
}

#[tokio::test]
async fn apply_tag_requires_the_apply_permission() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    let err = crate::logic::tag::apply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn unapply_tag_requires_the_unapply_permission() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let article_id = article_with_tags(&context, &actor, "rust").await;
    let tag_id = seeded_tag_id(&context, "devops").await;

    let err = crate::logic::tag::unapply_tag(&context.state, &actor, &article_id, &tag_id)
        .await
        .unwrap_err();
    assert_eq!(err, LogicError::forbidden("you are denied"));
}
