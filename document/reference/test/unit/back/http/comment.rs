
use crate::unit_tests::context::TestCtx;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

async fn update_name(ctx: &TestCtx, session: &str, name: &str) {
    let (status, _) = ctx
        .post(
            "/user/name",
            json!({"pow": ctx.issued_proof_of_work(name)}),
            Some(session),
        )
        .await;
    ctx.ok(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_requires_session() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            &format!("/version/{}/comments", Uuid::now_v7()),
            json!({"content": "hi"}),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_with_404_for_missing_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .post(
            &format!("/version/{}/comments", Uuid::now_v7()),
            json!({"content": "hi"}),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_rejects_bad_content() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let uri = format!("/version/{version_id}/comments");
    let (status, _) = ctx
        .post(&uri, json!({"content": "   "}), Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .post(&uri, json!({"content": "x".repeat(1025)}), Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .post(&uri, json!({"content": "你好"}), Some(&session))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_comment_ok_returns_comment_id() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let (status, body) = ctx
        .post(
            &format!("/version/{version_id}/comments"),
            json!({"content": "nice"}),
            Some(&session),
        )
        .await;
    ctx.created(status);
    assert!(body["comment_id"].as_str().is_some(), "必须返回 comment_id");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_reply_requires_session_and_404_missing_parent() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            &format!("/comments/{}/replies", Uuid::now_v7()),
            json!({"content": "hi"}),
            None,
        )
        .await;
    ctx.unauth(status);
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .post(
            &format!("/comments/{}/replies", Uuid::now_v7()),
            json!({"content": "hi"}),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_reply_ok() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let (status, body) = ctx
        .post(
            &format!("/version/{version_id}/comments"),
            json!({"content": "top"}),
            Some(&session),
        )
        .await;
    ctx.created(status);
    let parent = body["comment_id"]
        .as_str()
        .expect("必须有 comment_id")
        .to_string();
    let (status, body) = ctx
        .post(
            &format!("/comments/{parent}/replies"),
            json!({"content": "reply"}),
            Some(&session),
        )
        .await;
    ctx.created(status);
    assert!(
        body["comment_id"].as_str().is_some(),
        "回复必须返回 comment_id"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_comments_requires_session_and_404_missing_version() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .get(&format!("/version/{}/comments", Uuid::now_v7()), None)
        .await;
    ctx.unauth(status);
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .get(
            &format!("/version/{}/comments", Uuid::now_v7()),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_comments_shape_tree_and_author_names() {
    let ctx = TestCtx::new().await;
    let (alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    update_name(&ctx, &alice, "Alice").await;
    update_name(&ctx, &bob, "Bob").await;
    let (_article_id, version_id) = ctx.seed_article(&alice).await;

    let (status, body) = ctx
        .post(
            &format!("/version/{version_id}/comments"),
            json!({"content": "top"}),
            Some(&alice),
        )
        .await;
    ctx.created(status);
    let c1 = body["comment_id"].as_str().unwrap().to_string();
    let (status, body) = ctx
        .post(
            &format!("/comments/{c1}/replies"),
            json!({"content": "reply1"}),
            Some(&bob),
        )
        .await;
    ctx.created(status);
    let r1 = body["comment_id"].as_str().unwrap().to_string();
    let (status, body) = ctx
        .post(
            &format!("/comments/{r1}/replies"),
            json!({"content": "reply2"}),
            Some(&alice),
        )
        .await;
    ctx.created(status);
    let r2 = body["comment_id"].as_str().unwrap().to_string();

    let (status, body) = ctx
        .get(&format!("/version/{version_id}/comments"), Some(&alice))
        .await;
    ctx.ok(status);
    let comments = body["comments"]
        .as_array()
        .expect("响应必须有 comments 数组");
    assert_eq!(comments.len(), 3, "三层树共 3 条评论");
    let by_id: HashMap<String, &Value> = comments
        .iter()
        .map(|c| (c["id"].as_str().unwrap().to_string(), c))
        .collect();

    let c1e = by_id[&c1];
    assert_eq!(c1e["content"], "top");
    assert_eq!(c1e["user_id"], alice_id);
    assert!(c1e["parent_id"].is_null(), "顶层评论 parent_id 必须为 null");
    assert_eq!(
        c1e["user_name"], "Alice",
        "author_name 必须显示各自设置的名字"
    );
    assert!(
        c1e["created_at"].as_u64().is_some(),
        "created_at 必须是秒时间戳"
    );
    let r1e = by_id[&r1];
    assert_eq!(r1e["parent_id"], c1, "reply1 的父必须是 c1");
    assert_eq!(r1e["user_name"], "Bob");
    assert_eq!(r1e["content"], "reply1");
    let r2e = by_id[&r2];
    assert_eq!(r2e["parent_id"], r1, "reply2 的父必须是 r1");
    assert_eq!(r2e["user_name"], "Alice");

    let depth_of = |id: &str| -> usize {
        let mut depth = 0usize;
        let mut cur = id.to_string();
        loop {
            let entry = by_id[&cur];
            match entry["parent_id"].as_str() {
                Some(parent) => {
                    depth += 1;
                    cur = parent.to_string();
                }
                None => return depth,
            }
        }
    };
    assert_eq!(depth_of(&c1), 0, "顶层评论深度必须为 0");
    assert_eq!(depth_of(&r1), 1, "reply1 深度必须为 1");
    assert_eq!(depth_of(&r2), 2, "reply2 深度必须为 2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reply_beyond_tree_depth_cap_is_rejected() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;

    let max_depth = ctx.state.config.server.max_comment_tree_depth as usize;
    let mut chain: Vec<String> = Vec::new();
    let root = Uuid::now_v7().to_string();
    crate::repo::comment::create_top_level_comment(
        &ctx.state.db,
        &root,
        &user_id,
        &version_id,
        "deep root",
    )
    .await
    .expect("repo 直插顶层评论");
    chain.push(root);
    for i in 0..max_depth {
        let id = Uuid::now_v7().to_string();
        let parent = chain.last().expect("父 id 必须存在");
        crate::repo::comment::create_reply_comment(
            &ctx.state.db,
            &id,
            &user_id,
            parent,
            &format!("reply {i}"),
            max_depth,
        )
        .await
        .expect("repo 直插回复（未超上限）");
        chain.push(id);
    }

    let (status, body) = ctx
        .post(
            &format!("/comments/{}/replies", chain.last().unwrap()),
            json!({"content": "too deep"}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("too deep"),
        "拒绝原因必须指向深度上限，实际: {}",
        ctx.reason(&body)
    );

    let (status, body) = ctx
        .get(&format!("/version/{version_id}/comments"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        body["comments"].as_array().map(Vec::len),
        Some(65),
        "65 层链必须可完整列出"
    );
}
