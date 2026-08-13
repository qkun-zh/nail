
use crate::unit_tests::context::TestCtx;
use serde_json::json;
use uuid::Uuid;

#[allow(dead_code)]
async fn article_author_id(ctx: &TestCtx, article_id: &str) -> Option<String> {
    ctx.incoming_edge_from_id(
        crate::repo::types::ENTITY_TYPE_ARTICLE,
        crate::repo::types::EDGE_USER_TO_ARTICLE,
        article_id,
    )
    .await
}

async fn recycler_id(ctx: &TestCtx) -> String {
    crate::repo::user::find_user_by_email_address_hash(
        &ctx.state.db,
        &common::hash::email(&ctx.state.config.server.user_zero_email),
    )
    .await
    .expect("查询")
    .expect("user zero 必须存在")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_article_requires_session_and_rejects_malformed_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.post("/article/anything/delete", json!({}), None).await;
    ctx.unauth(status);
    let (status, _) = ctx
        .post(
            "/article/anything/delete",
            json!({}),
            Some(&ctx.malformed_session()),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_article_transfers_ownership_to_recycler_and_keeps_content() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({"content": "a comment"}),
                Some(&session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };

    let (status, body) = ctx
        .post(
            &format!("/article/{article_id}/delete"),
            json!({}),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["ok"].as_bool(), Some(true));

    let article = crate::repo::article::read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询")
        .expect("文章必须保留");
    assert_eq!(
        article.get("title").and_then(|v| v.as_str()),
        Some("seed title"),
        "文章内容必须保留"
    );

    assert_eq!(
        article_author_id(&ctx, &article_id).await.as_deref(),
        Some(recycler_id(&ctx).await.as_str()),
        "文章所有权边必须转移到回收者"
    );

    assert!(
        crate::repo::article::read_version(&ctx.state.db, &version_id)
            .await
            .expect("查询")
            .is_some(),
        "版本必须保留"
    );

    let comments = crate::repo::comment::read_comments_by_version(
        &ctx.state.db,
        &version_id,
        ctx.state.config.server.max_comment_tree_depth as usize,
    )
    .await
    .expect("查询");
    let comment = comments
        .iter()
        .find(|row| row.get("comment_id").and_then(|v| v.as_str()) == Some(comment_id.as_str()))
        .expect("评论必须保留");
    let comment_author = comment
        .get("author")
        .and_then(|v| v.as_str())
        .map(crate::repo::util::strip_record_id);
    assert_eq!(
        comment_author.as_deref(),
        Some(_user_id.as_str()),
        "评论仍归原评论者"
    );

    let (status, _) = ctx
        .post(
            &format!("/article/{article_id}/delete"),
            json!({}),
            Some(&session),
        )
        .await;
    ctx.forbidden(status);
    let (status, _) = ctx
        .post(
            &format!("/article/{article_id}"),
            serde_json::json!({"title": "new", "summary": "new"}),
            Some(&session),
        )
        .await;
    ctx.forbidden(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_article_by_non_author_is_forbidden() {
    let ctx = TestCtx::new().await;
    let (alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _version_id) = ctx.seed_article(&alice_session).await;

    let (status, _) = ctx
        .post(
            &format!("/article/{article_id}/delete"),
            json!({}),
            Some(&bob_session),
        )
        .await;
    ctx.forbidden(status);

    assert_eq!(
        article_author_id(&ctx, &article_id).await.as_deref(),
        Some(alice_id.as_str()),
        "非作者删除不得改动所有权"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_missing_article_is_not_found() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .post(
            &format!("/article/{}/delete", Uuid::now_v7()),
            json!({}),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}
