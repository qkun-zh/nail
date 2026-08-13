
use uuid::Uuid;

use crate::logic::comment::{handle_create_comment, handle_create_reply, handle_read_comments};
use crate::logic::error::LogicError;
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_requires_session() {
    let ctx = TestCtx::new().await;
    let err = handle_create_comment(&ctx.state, &ctx.ghost_session(), "v", "hi")
        .await
        .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_rejects_missing_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_create_comment(&ctx.state, &session, &Uuid::now_v7().to_string(), "hi")
        .await
        .expect_err("版本不存在必须 404");
    assert!(
        matches!(err, LogicError::NotFound(_)),
        "版本缺失 → NotFound(404)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_validates_content() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;

    let err = handle_create_comment(&ctx.state, &session, &version_id, "   ")
        .await
        .expect_err("空白内容必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let err = handle_create_comment(&ctx.state, &session, &version_id, "中文评论")
        .await
        .expect_err("非 ASCII 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let max = ctx.state.config.server.max_comment_body_chars as usize;
    let long = "a".repeat(max + 1);
    let err = handle_create_comment(&ctx.state, &session, &version_id, &long)
        .await
        .expect_err("超长内容必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_happy_path_trims_and_persists() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = handle_create_comment(&ctx.state, &session, &version_id, "  first comment  ")
        .await
        .expect("评论");
    assert!(!comment_id.is_empty());

    let list = handle_read_comments(&ctx.state, &session, &version_id)
        .await
        .expect("列表");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].get("content").and_then(|v| v.as_str()),
        Some("first comment"),
        "必须 trim 后存储"
    );
    assert_eq!(
        list[0].get("comment_id").and_then(|v| v.as_str()),
        Some(comment_id.as_str())
    );
    assert!(list[0].get("author").is_some(), "author 必须存在");
    assert!(
        list[0].get("parent").is_none() || list[0].get("parent").unwrap().is_null(),
        "顶层评论 parent 为 Null"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_requires_existing_parent() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, _version_id) = ctx.seed_article(&session).await;
    let err = handle_create_reply(&ctx.state, &session, &Uuid::now_v7().to_string(), "reply")
        .await
        .expect_err("父评论不存在必须 404");
    assert!(
        matches!(err, LogicError::NotFound(_)),
        "父缺失 → NotFound(404)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_builds_tree_and_lists_layered() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;

    let c1 = handle_create_comment(&ctx.state, &session, &version_id, "first")
        .await
        .expect("c1");
    let c2 = handle_create_comment(&ctx.state, &session, &version_id, "second")
        .await
        .expect("c2");
    let r1 = handle_create_reply(&ctx.state, &session, &c1, "reply to first")
        .await
        .expect("r1");
    let r2 = handle_create_reply(&ctx.state, &session, &r1, "nested")
        .await
        .expect("r2");

    let list = handle_read_comments(&ctx.state, &session, &version_id)
        .await
        .expect("列表");
    assert_eq!(list.len(), 4);
    let ids: Vec<String> = list
        .iter()
        .filter_map(|v| {
            v.get("comment_id")
                .and_then(|i| i.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    let pos_c1 = ids.iter().position(|i| i == &c1).expect("c1 在列");
    let pos_c2 = ids.iter().position(|i| i == &c2).expect("c2 在列");
    assert!(pos_c2 < pos_c1, "顶层评论最新在前");
    let pos_r1 = ids.iter().position(|i| i == &r1).expect("r1 在列");
    let pos_r2 = ids.iter().position(|i| i == &r2).expect("r2 在列");
    assert!(pos_r1 > pos_c1, "回复层在顶层之后");
    assert!(pos_r1 < pos_r2, "回复层时间正序（旧在前）");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_exceeding_tree_depth_cap_is_rejected() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let max_depth = ctx.state.config.server.max_comment_tree_depth as usize;

    let mut parent = handle_create_comment(&ctx.state, &session, &version_id, "root")
        .await
        .expect("root");
    for _ in 0..max_depth {
        let next = handle_create_reply(&ctx.state, &session, &parent, "layer")
            .await
            .expect("上限内回复必须成功");
        parent = next;
    }
    let err = handle_create_reply(&ctx.state, &session, &parent, "too deep")
        .await
        .expect_err("超深回复必须拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "树深超限 → BadRequest(400)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_comments_rejects_missing_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_read_comments(&ctx.state, &session, &Uuid::now_v7().to_string())
        .await
        .expect_err("版本不存在必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}
