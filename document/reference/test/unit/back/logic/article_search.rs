
use common::search::ArticleSearchParams;
use serde_json::json;

use crate::logic::article_search::handle_search_articles;
use crate::unit_tests::context::TestCtx;

fn ids(page: &crate::logic::article_search::SearchPage) -> Vec<String> {
    page.article_list.iter().map(|i| i.id.clone()).collect()
}

fn params(q: &str, ranges: &str) -> ArticleSearchParams {
    ArticleSearchParams {
        q: Some(q.to_string()),
        ranges: Some(ranges.to_string()),
        limit: Some(10),
        page: Some(1),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_q_title_AND_terms() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "memory safety guide", "s", "#t", "1.0.0", "n")
        .await
        .0;
    ctx.create_article(&session, "memory alone", "s", "#t", "1.0.0", "n")
        .await;
    let page = handle_search_articles(&ctx.state, &params("memory safety", "title"))
        .await
        .expect("搜索");
    assert_eq!(
        ids(&page),
        vec![a],
        "q=memory safety 必须只命中标题含两词的文章"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_q_title_AND_terms_with_id_expr_ranges() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "memory safety guide", "s", "#t", "1.0.0", "n")
        .await
        .0;
    ctx.create_article(&session, "memory alone", "s", "#t", "1.0.0", "n")
        .await;
    let page = handle_search_articles(&ctx.state, &params("memory safety", "title,author"))
        .await
        .expect("搜索");
    assert_eq!(
        ids(&page),
        vec![a],
        "title@AND@ + author(id 表达式) 同查时不得退化为单词命中"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_range_summary() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(
            &session,
            "Title",
            "about systems internals",
            "#t",
            "1.0.0",
            "n",
        )
        .await
        .0;
    let page = handle_search_articles(&ctx.state, &params("systems", "summary"))
        .await
        .expect("搜索");
    assert_eq!(ids(&page), vec![a], "summary 范围必须命中摘要含词的文章");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_range_author() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    update_name(&ctx, &alice, "rustacean").await;
    update_name(&ctx, &bob, "gofer").await;
    let a = ctx
        .create_article(&alice, "AA", "s", "#t", "1.0.0", "n")
        .await
        .0;
    ctx.create_article(&bob, "BB", "s", "#t", "1.0.0", "n")
        .await;
    let page = handle_search_articles(&ctx.state, &params("rustacean", "author"))
        .await
        .expect("搜索");
    assert_eq!(ids(&page), vec![a], "author 范围必须命中该作者的唯一文章");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_range_note_latest_version_only() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let x = ctx
        .create_article(&session, "X", "s", "#t", "1.0.0", "fix memory leak")
        .await
        .0;
    let (y, _) = ctx
        .create_article(&session, "Y", "s", "#t", "1.0.0", "fix memory leak")
        .await;
    ctx.add_version(
        &session,
        &y,
        "2.0.0",
        "unrelated",
        Some(&crate::unit_tests::context::test_pdf_variant("y2")),
    )
    .await;

    let page = handle_search_articles(&ctx.state, &params("memory", "note"))
        .await
        .expect("搜索");
    assert_eq!(
        ids(&page),
        vec![x],
        "note 范围只取最新版本命中，Y 的最新版本无 memory 不得上榜"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_range_comment() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (a, version_id) = ctx
        .create_article(&session, "AA", "s", "#t", "1.0.0", "n")
        .await;
    let (status, _) = ctx
        .post(
            &format!("/version/{version_id}/comments"),
            json!({"content": "memory safety explained here"}),
            Some(&session),
        )
        .await;
    ctx.created(status);
    let page = handle_search_articles(&ctx.state, &params("safety", "comment"))
        .await
        .expect("搜索");
    assert_eq!(ids(&page), vec![a], "comment 范围必须命中评论所属文章");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_range_tag() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "AA", "s", "#rust", "1.0.0", "n")
        .await
        .0;
    let page = handle_search_articles(&ctx.state, &params("rust", "tag"))
        .await
        .expect("搜索");
    assert_eq!(ids(&page), vec![a], "tag 范围必须命中带该标签的文章");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_or_merge_across_ranges() {
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

    let page = handle_search_articles(&ctx.state, &params("needle", "title,summary"))
        .await
        .expect("搜索");
    let mut got = ids(&page);
    got.sort();
    let mut want = vec![a.clone(), b.clone()];
    want.sort();
    assert_eq!(got, want, "OR 合并必须命中任一勾选字段的文章");

    let a_view = page
        .article_list
        .iter()
        .find(|i| i.id == a)
        .expect("a 在列");
    let b_view = page
        .article_list
        .iter()
        .find(|i| i.id == b)
        .expect("b 在列");
    assert_eq!(
        a_view
            .hits
            .iter()
            .map(|h| h.field.as_str())
            .collect::<Vec<_>>(),
        vec!["title"]
    );
    assert_eq!(
        b_view
            .hits
            .iter()
            .map(|h| h.field.as_str())
            .collect::<Vec<_>>(),
        vec!["summary"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_relevance_rrf_prefers_multi_source() {
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

    let page = handle_search_articles(&ctx.state, &params("zzz", "title,summary"))
        .await
        .expect("搜索");
    assert_eq!(
        ids(&page),
        vec![x.clone(), y.clone()],
        "多来源命中的 x 必须经 RRF 综合分排在仅单来源命中的 y 之前（无视创建先后）"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_multikey_sort() {
    let ctx = TestCtx::new().await;
    let (_u1, alice) = ctx.register("alice@qq.com").await;
    let (_u2, bob) = ctx.register("bob@qq.com").await;
    update_name(&ctx, &alice, "aauthor").await;
    update_name(&ctx, &bob, "bauthor").await;
    ctx.create_article(&alice, "zeta", "s", "#t", "1.0.0", "n")
        .await;
    ctx.create_article(&alice, "alpha", "s", "#t", "1.0.0", "n")
        .await;
    ctx.create_article(&bob, "middle", "s", "#t", "1.0.0", "n")
        .await;

    let mut p = params("", "");
    p.sort = Some("author:asc,title:asc".to_string());
    let page = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    let titles: Vec<&str> = page.article_list.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(
        titles,
        vec!["alpha", "zeta", "middle"],
        "author 主键 + title 次键的多键排序必须按序稳定"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_time_window_latest_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_a, v) = ctx
        .create_article(&session, "T", "s", "#t", "1.0.0", "n")
        .await;
    let t = common::time::uuidv7_timestamp_secs(&v).expect("版本 id 必须是 uuidv7");
    let mut p = params("", "");
    p.from = Some(t);
    p.to = Some(t);
    let page = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert_eq!(ids(&page).len(), 1, "from==to 的闭合窗必须命中");
    p.from = Some(t + 1);
    p.to = None;
    let page2 = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert!(ids(&page2).is_empty(), "from 晚于创建 → 空");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_pagination_and_reverse_scan() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let mut created = Vec::new();
    for i in 0..5 {
        created.push(
            ctx.create_article(&session, &format!("P{i}"), "s", "#t", "1.0.0", "n")
                .await
                .0,
        );
        separate_creation();
    }
    let mut p = params("", "");
    p.limit = Some(2);
    p.sort = Some("time:desc".to_string());

    p.page = Some(1);
    let page1 = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert_eq!(
        ids(&page1),
        vec![created[4].clone(), created[3].clone()],
        "第 1 页 = 最新 2 篇"
    );
    assert_eq!(page1.total, 5);
    assert_eq!(page1.total_pages, 3);
    assert!(page1.has_more);
    assert!(!page1.has_prev);

    p.page = Some(2);
    let page2 = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert_eq!(ids(&page2), vec![created[2].clone(), created[1].clone()]);

    p.page = Some(3);
    let page3 = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert_eq!(
        ids(&page3),
        vec![created[0].clone()],
        "反扫页必须返回最旧一篇"
    );
    assert!(!page3.has_more);
    assert!(page3.has_prev);

    p.page = Some(10);
    let p10 = handle_search_articles(&ctx.state, &p).await.expect("搜索");
    assert!(ids(&p10).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_truncated_when_pages_capped() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    for i in 0..5 {
        ctx.create_article(&session, &format!("T{i}"), "s", "#t", "1.0.0", "n")
            .await;
        separate_creation();
    }
    let mut ctx2 = TestCtx::new().await;
    let (_u2, s2) = ctx2.register("bob@qq.com").await;
    for i in 0..5 {
        ctx2.create_article(&s2, &format!("B{i}"), "s", "#t", "1.0.0", "n")
            .await;
        separate_creation();
    }
    std::sync::Arc::make_mut(&mut ctx2.state.config)
        .server
        .max_search_pages = 2;
    let mut p = params("", "");
    p.limit = Some(2);
    p.page = Some(3);
    let page3 = handle_search_articles(&ctx2.state, &p).await.expect("搜索");
    assert!(page3.truncated, "超过封顶必须 truncated=true");
    assert_eq!(page3.total_pages, 2);
    assert_eq!(page3.page, 2, "封顶后 page 钳制到最后一页");
    assert!(!page3.has_more, "封顶后不得 has_more");
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

fn separate_creation() {
    std::thread::sleep(std::time::Duration::from_millis(2));
}
