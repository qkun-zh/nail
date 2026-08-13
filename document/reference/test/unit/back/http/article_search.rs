
use crate::unit_tests::context::TestCtx;
use common::time::uuidv7_timestamp_secs;
use serde_json::json;
use std::time::Duration;

fn separate_creation() {
    std::thread::sleep(Duration::from_millis(2));
}

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

async fn search_ids(ctx: &TestCtx, session: &str, query: &str) -> Vec<String> {
    let (status, body) = ctx
        .get(&format!("/article/search{query}"), Some(session))
        .await;
    ctx.ok(status);
    body["article_list"]
        .as_array()
        .expect("响应必须有 article_list 数组")
        .iter()
        .filter_map(|a| a["id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.get("/article/search", None).await;
    ctx.unauth(status);
    let (status, _) = ctx
        .get("/article/search", Some(&ctx.malformed_session()))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_rejects_invalid_params() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .get("/article/search?from=100&to=50", Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .get("/article/search?q=x&ranges=bogus", Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .get("/article/search?sort=time:sideways", Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .get("/article/search?sort=weight:asc", Some(&session))
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .get(
            &format!("/article/search?q={}", "a".repeat(513)),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_q_text_AND_terms() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "memory safety guide", "s", "#t", "1.0.0", "n")
        .await
        .0;
    ctx.create_article(&session, "memory alone", "s", "#t", "1.0.0", "n")
        .await;
    let got = search_ids(&ctx, &session, "?q=memory%20safety").await;
    assert_eq!(
        got,
        vec![a.clone()],
        "q=memory safety 必须只命中标题或摘要含全部词的文章"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_ranges_subset_title_only() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "rustic title", "plain", "#t", "1.0.0", "n")
        .await
        .0;
    let b = ctx
        .create_article(
            &session,
            "plain title",
            "rustic summary",
            "#t",
            "1.0.0",
            "n",
        )
        .await
        .0;
    assert_eq!(
        search_ids(&ctx, &session, "?q=rustic&ranges=title").await,
        vec![a.clone()],
        "ranges=title 必须只命中标题含词的文章"
    );
    assert_eq!(
        search_ids(&ctx, &session, "?q=rustic&ranges=summary").await,
        vec![b.clone()],
        "ranges=summary 必须只命中摘要含词的文章"
    );
    let got = search_ids(&ctx, &session, "?q=rustic&ranges=").await;
    assert_eq!(got.len(), 2, "ranges 空串 = 全部范围，两篇都中");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_or_across_ranges() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "needle in title", "plain", "#t", "1.0.0", "n")
        .await
        .0;
    let b = ctx
        .create_article(&session, "plain", "needle in summary", "#t", "1.0.0", "n")
        .await
        .0;
    let (status, body) = ctx.get("/article/search?q=needle", Some(&session)).await;
    ctx.ok(status);
    let mut got: Vec<String> = body["article_list"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect();
    got.sort();
    let mut want = vec![a.clone(), b.clone()];
    want.sort();
    assert_eq!(got, want, "OR 合并必须命中任一勾选字段的文章");

    let by_id: serde_json::Map<String, serde_json::Value> = body["article_list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["id"].as_str().unwrap().to_string(),
                x.get("hits").cloned().unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();
    let a_field: Vec<_> = by_id[&a]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|h| h["field"].as_str())
        .collect();
    let b_field: Vec<_> = by_id[&b]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|h| h["field"].as_str())
        .collect();
    assert_eq!(a_field, vec!["title"], "a 只命中 title");
    assert_eq!(b_field, vec!["summary"], "b 只命中 summary");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_time_window_latest_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_a, v) = ctx
        .create_article(&session, "time t", "s", "#t", "1.0.0", "n")
        .await;
    let t = uuidv7_timestamp_secs(&v).expect("版本 id 必须是 uuidv7");
    assert_eq!(
        search_ids(&ctx, &session, &format!("?from={t}&to={t}")).await,
        vec![_a.clone()],
        "时间窗必须含边界且以最新版本时间计"
    );
    assert_eq!(
        search_ids(&ctx, &session, &format!("?from={}", t + 1))
            .await
            .len(),
        0,
        "from 晚于创建 → 不命中"
    );
    assert_eq!(
        search_ids(&ctx, &session, &format!("?to={}", t.saturating_sub(1)))
            .await
            .len(),
        0,
        "to 早于创建 → 不命中"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_sort_by_time_title_and_author() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    update_name(&ctx, &alice, "Alice").await;
    update_name(&ctx, &bob, "Bob").await;
    let alpha = ctx
        .create_article(&alice, "alpha", "s", "#x", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let bravo = ctx
        .create_article(&alice, "bravo", "s", "#x", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let charlie = ctx
        .create_article(&bob, "charlie", "s", "#x", "1.0.0", "n")
        .await
        .0;
    assert_eq!(
        search_ids(&ctx, &alice, "?sort=title:asc").await,
        vec![alpha.clone(), bravo.clone(), charlie.clone()],
        "title 升序必须按字母序"
    );
    assert_eq!(
        search_ids(&ctx, &alice, "?sort=title:desc").await,
        vec![charlie.clone(), bravo.clone(), alpha.clone()],
        "title 降序必须按字母倒序"
    );
    assert_eq!(
        search_ids(&ctx, &alice, "?sort=time:desc").await,
        vec![charlie.clone(), bravo.clone(), alpha.clone()],
        "time desc 必须最新在前"
    );
    let ordered = search_ids(&ctx, &alice, "?sort=author:asc").await;
    assert_eq!(ordered.len(), 3, "author 排序返回全部文章");
    let pos_alpha = ordered
        .iter()
        .position(|x| x == &alpha)
        .expect("alpha 在列");
    let pos_bravo = ordered
        .iter()
        .position(|x| x == &bravo)
        .expect("bravo 在列");
    let pos_charlie = ordered
        .iter()
        .position(|x| x == &charlie)
        .expect("charlie 在列");
    assert!(
        pos_alpha < pos_charlie && pos_bravo < pos_charlie,
        "author 升序：Alice 的两篇都在 Bob 前，实际: {ordered:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_default_relevance_sort() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let x = ctx
        .create_article(&session, "zzz alpha", "zzz beta", "#t", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let y = ctx
        .create_article(&session, "zzz only", "plain", "#t", "1.0.0", "n")
        .await
        .0;
    assert_eq!(
        search_ids(&ctx, &session, "?q=zzz").await,
        vec![x.clone(), y.clone()],
        "默认按相关度排序：多来源命中的 x 应在 y 前（无视创建先后）"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_pagination_and_clamps() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "a", "s", "#x", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let b = ctx
        .create_article(&session, "b", "s", "#x", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let c = ctx
        .create_article(&session, "c", "s", "#x", "1.0.0", "n")
        .await
        .0;
    assert_eq!(
        search_ids(&ctx, &session, "").await,
        vec![c.clone(), b.clone(), a.clone()]
    );

    let (status, body) = ctx
        .get("/article/search?limit=2&page=1", Some(&session))
        .await;
    ctx.ok(status);
    let page1: Vec<String> = body["article_list"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect();
    assert_eq!(page1, vec![c.clone(), b.clone()], "第 1 页必须是前两篇");
    assert_eq!(body["total"].as_u64(), Some(3));
    assert_eq!(body["has_more"].as_bool(), Some(true));
    assert_eq!(body["total_pages"].as_u64(), Some(2));

    let (status, body) = ctx
        .get("/article/search?limit=2&page=2", Some(&session))
        .await;
    ctx.ok(status);
    let page2: Vec<String> = body["article_list"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect();
    assert_eq!(page2, vec![a.clone()], "第 2 页必须是最后一篇");
    assert_eq!(body["has_more"].as_bool(), Some(false));
    assert_eq!(body["has_prev"].as_bool(), Some(true));

    let (status, body) = ctx.get("/article/search?limit=0", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        body["article_list"].as_array().map(Vec::len),
        Some(1),
        "limit=0 必须钳到 1"
    );
    let (status, body) = ctx
        .get("/article/search?limit=2&page=0", Some(&session))
        .await;
    ctx.ok(status);
    let page0: Vec<String> = body["article_list"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect();
    assert_eq!(page0, page1, "page=0 必须钳到第 1 页");
    let (status, body) = ctx
        .get("/article/search?limit=2&page=999999", Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        body["article_list"].as_array().map(Vec::len),
        Some(0),
        "越界 page 钳制后必须空页而非报错"
    );
}
