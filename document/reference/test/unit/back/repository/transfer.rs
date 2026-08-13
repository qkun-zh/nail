
use crate::repo::transfer::{
    TargetTransferError, transfer_article_ownership, transfer_comment_ownership,
};
use crate::unit_tests::context::TestCtx;
use uuid::Uuid;

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
async fn transfer_article_ownership_is_idempotent() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _version_id) = ctx.seed_article(&session).await;

    transfer_article_ownership(&ctx.state.db, &article_id)
        .await
        .expect("首次转移必须成功");
    transfer_article_ownership(&ctx.state.db, &article_id)
        .await
        .expect("幂等重放必须成功");

    let db = ctx.state.db.read().await;
    let article = crate::repo::db::resolve_node_id_sync(
        &db,
        crate::repo::types::ENTITY_TYPE_ARTICLE,
        &article_id,
    )
    .expect("查询")
    .expect("文章存在");
    let edges = db
        .exec(
            agdb::QueryBuilder::search()
                .to(article)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(crate::repo::types::KEY_TYPE)
                .value(crate::repo::types::EDGE_USER_TO_ARTICLE)
                .query(),
        )
        .expect("查询边");
    let owner_id = edges.elements.first().and_then(|el| {
        crate::repo::db::read_node_sync::<crate::repo::types::IdRow>(&db, el.from)
            .expect("查询")
            .map(|row| row.id)
    });
    assert_eq!(
        owner_id.as_deref(),
        Some(recycler_id(&ctx).await.as_str()),
        "边必须指向回收者"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transfer_comment_ownership_is_idempotent() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({"content": "mine"}),
                Some(&session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };

    transfer_comment_ownership(&ctx.state.db, &comment_id)
        .await
        .expect("首次转移必须成功");
    transfer_comment_ownership(&ctx.state.db, &comment_id)
        .await
        .expect("幂等重放必须成功");

    let db = ctx.state.db.read().await;
    let comment = crate::repo::db::resolve_node_id_sync(
        &db,
        crate::repo::types::ENTITY_TYPE_COMMENT,
        &comment_id,
    )
    .expect("查询")
    .expect("评论存在");
    let edges = db
        .exec(
            agdb::QueryBuilder::search()
                .to(comment)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(crate::repo::types::KEY_TYPE)
                .value(crate::repo::types::EDGE_USER_TO_COMMENT)
                .query(),
        )
        .expect("查询边");
    let owner_id = edges.elements.first().and_then(|el| {
        crate::repo::db::read_node_sync::<crate::repo::types::IdRow>(&db, el.from)
            .expect("查询")
            .map(|row| row.id)
    });
    assert_eq!(
        owner_id.as_deref(),
        Some(recycler_id(&ctx).await.as_str()),
        "边必须指向回收者"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transfer_missing_target_is_not_found() {
    let ctx = TestCtx::new().await;
    let err = transfer_article_ownership(&ctx.state.db, &Uuid::now_v7().to_string())
        .await
        .expect_err("目标不存在必须报错");
    assert!(
        matches!(err, TargetTransferError::TargetNotFound),
        "必须映射为 TargetNotFound，实际: {err:?}"
    );
}
