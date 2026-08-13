
use crate::unit_tests::context::TestCtx;
use serde_json::json;
use uuid::Uuid;

#[allow(dead_code)]
async fn comment_author_id(ctx: &TestCtx, comment_id: &str) -> Option<String> {
    ctx.incoming_edge_from_id(
        crate::repo::types::ENTITY_TYPE_COMMENT,
        crate::repo::types::EDGE_USER_TO_COMMENT,
        comment_id,
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
async fn delete_comment_requires_session() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            &format!("/comments/{}/delete", Uuid::now_v7()),
            json!({}),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_comment_transfers_ownership_to_recycler_and_keeps_content() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&alice_session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({"content": "top"}),
                Some(&alice_session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };
    let (bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let reply_id = {
        let (status, body) = ctx
            .post(
                &format!("/comments/{comment_id}/replies"),
                serde_json::json!({"content": "reply"}),
                Some(&bob_session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };

    let (status, body) = ctx
        .post(
            &format!("/comments/{comment_id}/delete"),
            json!({}),
            Some(&alice_session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["ok"].as_bool(), Some(true));

    assert_eq!(
        comment_author_id(&ctx, &comment_id).await.as_deref(),
        Some(recycler_id(&ctx).await.as_str()),
        "评论所有权必须转移到回收者"
    );
    assert_eq!(
        comment_author_id(&ctx, &reply_id).await.as_deref(),
        Some(bob_id.as_str()),
        "回复仍归原作者"
    );

    let (status, _) = ctx
        .post(
            &format!("/comments/{comment_id}/delete"),
            json!({}),
            Some(&alice_session),
        )
        .await;
    ctx.forbidden(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_comment_by_non_author_is_forbidden() {
    let ctx = TestCtx::new().await;
    let (alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&alice_session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({"content": "mine"}),
                Some(&alice_session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };

    let (status, _) = ctx
        .post(
            &format!("/comments/{comment_id}/delete"),
            json!({}),
            Some(&bob_session),
        )
        .await;
    ctx.forbidden(status);

    assert_eq!(
        comment_author_id(&ctx, &comment_id).await.as_deref(),
        Some(alice_id.as_str()),
        "非作者删除不得改动所有权"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_missing_comment_is_not_found() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .post(
            &format!("/comments/{}/delete", Uuid::now_v7()),
            json!({}),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}
