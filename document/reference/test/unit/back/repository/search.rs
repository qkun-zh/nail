
use crate::repo::search::{SearchHitDoc, SearchQuery, search_articles, sync_article};
use crate::unit_tests::context::TestCtx;
use seekstorm::commit::Commit;
use seekstorm::index::IndexDocument;

fn titles(docs: &[SearchHitDoc]) -> Vec<String> {
    docs.iter().map(|d| d.title.clone()).collect()
}

fn ids(docs: &[SearchHitDoc]) -> Vec<String> {
    docs.iter().map(|d| d.id.clone()).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn count_and_list_articles_use_graph_facts() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (a1, _) = ctx
        .create_article(&session, "Zebra", "s", "#t", "1.0.0", "n")
        .await;
    let (a2, _) = ctx
        .create_article(&session, "Apple", "s", "#t", "1.0.0", "n")
        .await;
    let (a3, _) = ctx
        .create_article(&session, "Mango", "s", "#t", "1.0.0", "n")
        .await;

    let total = crate::repo::search::count_articles(&ctx.state.db)
        .await
        .expect("计数");
    assert_eq!(total, 3);

    let page = crate::repo::search::list_articles_page(&ctx.state.db, 10, 0)
        .await
        .expect("列表");
    assert_eq!(ids(&to_docs(&page)), vec![a3, a2, a1]);

    let page2 = crate::repo::search::list_articles_page(&ctx.state.db, 1, 1)
        .await
        .expect("列表");
    assert_eq!(page2.len(), 1);
}

fn to_docs(rows: &[serde_json::Value]) -> Vec<SearchHitDoc> {
    rows.iter()
        .map(|row| SearchHitDoc {
            id: row
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: row
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            author: String::new(),
            ts_secs: 0,
            hits: Vec::new(),
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_and_terms_require_all_words() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    ctx.create_article(&session, "Memory Safety", "rust docs", "#t", "1.0.0", "n")
        .await;
    ctx.create_article(&session, "Machine Learning", "ai docs", "#t", "1.0.0", "n")
        .await;

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("memory safety".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 1);
    assert_eq!(titles(&outcome.docs), vec!["Memory Safety"]);

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("memory ai".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 0);
    assert!(outcome.docs.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_field_filter_limits_fields() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    ctx.create_article(&session, "Ownership", "borrow checker", "#t", "1.0.0", "n")
        .await;

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("borrow".to_string()),
            fields: vec!["summary".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 1);
    let snippet = outcome.docs[0]
        .hits
        .iter()
        .find(|(field, _)| field == "summary")
        .map(|(_, s)| s.clone())
        .expect("summary 命中片段");
    assert!(snippet.contains("<mark>borrow</mark>"), "片段: {snippet}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_query_returns_all_with_graph_total() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    ctx.create_article(&session, "One", "s", "#t", "1.0.0", "n")
        .await;
    ctx.create_article(&session, "Two", "s", "#t", "1.0.0", "n")
        .await;
    ctx.create_article(&session, "Three", "s", "#t", "1.0.0", "n")
        .await;

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: None,
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 3);
    assert_eq!(outcome.docs.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_time_window_filters_by_latest_version_time() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (a1, _) = ctx
        .create_article(&session, "Old", "s", "#t", "1.0.0", "n")
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let _a2 = ctx
        .create_article(&session, "New", "s", "#t", "1.0.0", "n")
        .await;

    let ts1 = common::time::uuidv7_timestamp_secs(&a1).expect("a1 时间");
    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: None,
            fields: vec!["title".to_string()],
            from: Some(ts1),
            to: Some(ts1),
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 1);
    assert_eq!(titles(&outcome.docs), vec!["Old"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_time_window_from_only_and_to_only() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (a1, _) = ctx
        .create_article(&session, "Old", "s", "#t", "1.0.0", "n")
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (a2, _) = ctx
        .create_article(&session, "New", "s", "#t", "1.0.0", "n")
        .await;
    let t1 = common::time::uuidv7_timestamp_secs(&a1).expect("a1 时间");
    let t2 = common::time::uuidv7_timestamp_secs(&a2).expect("a2 时间");
    assert!(t1 < t2, "间隔必须让两篇落在不同秒");

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: None,
            fields: vec!["title".to_string()],
            from: Some(t2),
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(ids(&outcome.docs), vec![a2.clone()], "from=t2 只含新文章");

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: None,
            fields: vec!["title".to_string()],
            from: None,
            to: Some(t1),
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(ids(&outcome.docs), vec![a1.clone()], "to=t1 只含旧文章");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_sort_keys_apply_multi_key_order() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_a1, _) = ctx
        .create_article(&session, "Bravo", "s", "#t", "1.0.0", "n")
        .await;
    let (a2, _) = ctx
        .create_article(&session, "Alpha", "s", "#t", "1.0.0", "n")
        .await;

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: None,
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: vec![("ts".to_string(), true)],
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(ids(&outcome.docs)[0], a2, "时间降序应 a2（新）在前");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sync_article_updates_and_removes_indexed_document() {
    let ctx = TestCtx::new().await;
    let (_user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    crate::repo::article::create_article(
        &ctx.state.db,
        &article_id,
        &_user_id,
        "Initial Title",
        "initial summary",
        &["#t".to_string()],
        &version_id,
        "1.0.0",
        &common::hash::pdf(b"sync-a"),
        "n",
    )
    .await
    .expect("建文");

    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("initial".to_string()),
            fields: vec!["title".to_string(), "summary".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 0, "未同步不可搜");

    sync_article(&ctx.state.search, &ctx.state.db, &article_id)
        .await
        .expect("同步");
    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("initial".to_string()),
            fields: vec!["title".to_string(), "summary".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 1);

    crate::repo::article::update_article(
        &ctx.state.db,
        &article_id,
        &_user_id,
        "Renamed Title",
        "initial summary",
        &["#t".to_string()],
    )
    .await
    .expect("改文");
    sync_article(&ctx.state.search, &ctx.state.db, &article_id)
        .await
        .expect("同步");
    let old = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("initial".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(old.total, 0, "旧标题不可搜");
    let new = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("renamed".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(new.total, 1, "新标题可搜");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rebuild_index_is_idempotent_and_self_healing() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (a1, _) = ctx
        .create_article(&session, "Persisted", "s", "#t", "1.0.0", "n")
        .await;
    let _ = a1;

    let n1 = crate::repo::search::rebuild_index(&ctx.state.search, &ctx.state.db)
        .await
        .expect("重建");
    let n2 = crate::repo::search::rebuild_index(&ctx.state.search, &ctx.state.db)
        .await
        .expect("重建");
    assert_eq!(n1, 1);
    assert_eq!(n2, 1);
    let outcome = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("persisted".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(outcome.total, 1, "重建后仍恰好一篇（无重复文档）");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rebuild_index_clears_stale_docs_not_in_graph() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    ctx.create_article(&session, "Real", "s", "#t", "1.0.0", "n")
        .await;

    let phantom = serde_json::json!({
        "id": "phantom-article",
        "title": "Phantom Ghost Doc",
        "summary": "",
        "author": "",
        "note": "",
        "tag": [],
        "comment": [],
        "ts": 0,
    })
    .as_object()
    .expect("object")
    .clone()
    .into_iter()
    .collect();
    ctx.state
        .search
        .index_document(phantom, seekstorm::index::FileType::None)
        .await;
    ctx.state.search.commit().await;

    let before = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("phantom".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(before.total, 1, "残留文档重建前应可搜到");

    let rebuilt = crate::repo::search::rebuild_index(&ctx.state.search, &ctx.state.db)
        .await
        .expect("重建");
    assert_eq!(rebuilt, 1, "重建应恰好重灌 agdb 里的一篇文章");
    let after = search_articles(
        &ctx.state.search,
        &ctx.state.db,
        SearchQuery {
            q: Some("phantom".to_string()),
            fields: vec!["title".to_string()],
            from: None,
            to: None,
            sort: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .expect("搜索");
    assert_eq!(after.total, 0, "重建必须清掉 agdb 已不存在的残留文档");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropping_test_ctx_releases_search_index_memory_and_dirs() {
    fn open_fds() -> usize {
        std::fs::read_dir("/proc/self/fd")
            .map(|d| d.count())
            .unwrap_or(0)
    }
    fn search_dirs() -> usize {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return 0;
        };
        entries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("nail_search_index_"))
            })
            .count()
    }

    drop(TestCtx::new().await);
    let fds_before = open_fds();
    let dirs_before = search_dirs();
    for _ in 0..20 {
        drop(TestCtx::new().await);
    }
    let fds_after = open_fds();
    let dirs_after = search_dirs();
    eprintln!(
        "fd delta {}; search dirs {dirs_before} -> {dirs_after}",
        fds_after.saturating_sub(fds_before)
    );
    assert!(
        fds_after.saturating_sub(fds_before) < 100,
        "dropping TestCtx leaks search index fds: {fds_before} -> {fds_after}"
    );
    assert!(
        dirs_after <= dirs_before + 2,
        "TestCtx drop must clean its search index dirs: {dirs_before} -> {dirs_after}"
    );
}
