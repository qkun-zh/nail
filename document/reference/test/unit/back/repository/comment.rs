
use uuid::Uuid;

use crate::repo::comment::{
    CreateCommentError, create_reply_comment, create_top_level_comment, read_comments_by_version,
};
use crate::unit_tests::context::TestCtx;

fn v7() -> String {
    Uuid::now_v7().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn top_level_comment_rejects_missing_version() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let err = create_top_level_comment(&ctx.state.db, &v7(), &user_id, &v7(), "hi")
        .await
        .expect_err("版本缺失必须拒绝");
    assert!(
        matches!(err, CreateCommentError::TargetNotFound),
        "版本缺失 → TargetNotFound，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn top_level_comment_happy_path_commits_node_and_edges() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = v7();
    create_top_level_comment(&ctx.state.db, &comment_id, &user_id, &version_id, "hello")
        .await
        .expect("create");

    let rows = read_comments_by_version(&ctx.state.db, &version_id, 64)
        .await
        .expect("列表");
    let row = rows
        .iter()
        .find(|r| r.get("comment_id").and_then(|v| v.as_str()) == Some(comment_id.as_str()))
        .expect("评论节点必须存在");
    assert_eq!(row.get("content").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(
        row.get("author").and_then(|v| v.as_str()),
        Some(user_id.as_str()),
        "作者边必须指向评论作者"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn top_level_comment_rejects_duplicate_id() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = v7();
    create_top_level_comment(&ctx.state.db, &comment_id, &user_id, &version_id, "one")
        .await
        .expect("第一次");
    let err = create_top_level_comment(&ctx.state.db, &comment_id, &user_id, &version_id, "two")
        .await
        .expect_err("重复 comment_id 必须拒绝");
    assert!(
        matches!(err, CreateCommentError::CommentIdExists),
        "重复 id → CommentIdExists，实际: {err:?}"
    );
    let rows = read_comments_by_version(&ctx.state.db, &version_id, 64)
        .await
        .expect("查询");
    let row = rows
        .iter()
        .find(|r| r.get("comment_id").and_then(|v| v.as_str()) == Some(comment_id.as_str()))
        .expect("原评论必须还在");
    assert_eq!(row.get("content").and_then(|v| v.as_str()), Some("one"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_rejects_missing_parent() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let err = create_reply_comment(&ctx.state.db, &v7(), &user_id, &v7(), "reply", 64)
        .await
        .expect_err("父评论缺失必须拒绝");
    assert!(
        matches!(err, CreateCommentError::TargetNotFound),
        "父缺失 → TargetNotFound，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_builds_comment_to_comment_edge() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let root = v7();
    create_top_level_comment(&ctx.state.db, &root, &user_id, &version_id, "root")
        .await
        .expect("root");

    let reply = v7();
    create_reply_comment(&ctx.state.db, &reply, &user_id, &root, "reply", 64)
        .await
        .expect("reply");

    let rows = read_comments_by_version(&ctx.state.db, &version_id, 64)
        .await
        .expect("列表");
    let row = rows
        .iter()
        .find(|r| r.get("comment_id").and_then(|v| v.as_str()) == Some(reply.as_str()))
        .expect("reply 行");
    assert_eq!(
        row.get("parent").and_then(|v| v.as_str()),
        Some(root.as_str())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_depth_cap_is_enforced() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let max = ctx.state.config.server.max_comment_tree_depth;

    let mut parent = v7();
    create_top_level_comment(&ctx.state.db, &parent, &user_id, &version_id, "root")
        .await
        .expect("root");
    for _ in 0..max {
        let next = v7();
        create_reply_comment(
            &ctx.state.db,
            &next,
            &user_id,
            &parent,
            "layer",
            max as usize,
        )
        .await
        .expect("上限内可创建");
        parent = next;
    }
    let err = create_reply_comment(
        &ctx.state.db,
        &v7(),
        &user_id,
        &parent,
        "too deep",
        max as usize,
    )
    .await
    .expect_err("超深必须拒绝");
    assert!(
        matches!(err, CreateCommentError::CommentTreeTooDeep),
        "超深 → CommentTreeTooDeep，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_comments_shapes_tree_and_order() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;

    let c1 = v7();
    create_top_level_comment(&ctx.state.db, &c1, &user_id, &version_id, "first")
        .await
        .expect("c1");
    let c2 = v7();
    create_top_level_comment(&ctx.state.db, &c2, &user_id, &version_id, "second")
        .await
        .expect("c2");
    let r1 = v7();
    create_reply_comment(&ctx.state.db, &r1, &user_id, &c1, "reply", 64)
        .await
        .expect("r1");
    let r2 = v7();
    create_reply_comment(&ctx.state.db, &r2, &user_id, &r1, "nested", 64)
        .await
        .expect("r2");

    let list = read_comments_by_version(&ctx.state.db, &version_id, 64)
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
    let pos_r1 = ids.iter().position(|i| i == &r1).expect("r1 在列");
    let pos_r2 = ids.iter().position(|i| i == &r2).expect("r2 在列");
    assert!(pos_c2 < pos_c1, "顶层最新在前");
    assert!(pos_r1 > pos_c1, "回复在顶层之后");
    assert!(pos_r1 < pos_r2, "回复层时间正序");
    let c1_row = list
        .iter()
        .find(|v| v.get("comment_id").and_then(|i| i.as_str()) == Some(c1.as_str()))
        .expect("c1 行");
    assert!(c1_row.get("parent").is_none() || c1_row.get("parent").unwrap().is_null());
    let r1_row = list
        .iter()
        .find(|v| v.get("comment_id").and_then(|i| i.as_str()) == Some(r1.as_str()))
        .expect("r1 行");
    assert_eq!(
        r1_row.get("parent").and_then(|v| v.as_str()),
        Some(c1.as_str())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_comments_for_missing_version_returns_empty() {
    let ctx = TestCtx::new().await;
    let rows = read_comments_by_version(&ctx.state.db, &v7(), 64)
        .await
        .expect("repo 层不报错");
    assert!(rows.is_empty(), "缺失版本无评论边 → 空列表");
}
