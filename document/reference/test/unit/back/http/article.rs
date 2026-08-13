
use crate::unit_tests::context::TestCtx;
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

fn separate_creation() {
    std::thread::sleep(Duration::from_millis(2));
}

fn article_ids(list: &Value) -> Vec<String> {
    list["article_list"]
        .as_array()
        .expect("响应必须有 article_list 数组")
        .iter()
        .filter_map(|a| a["id"].as_str().map(str::to_string))
        .collect()
}

fn create_fields(title: &str, summary: &str, tags: &str) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("title", title.as_bytes().to_vec()),
        ("summary", summary.as_bytes().to_vec()),
        ("tags", tags.as_bytes().to_vec()),
        ("version", "1.0.0".as_bytes().to_vec()),
        ("note", "initial".as_bytes().to_vec()),
        ("file", crate::unit_tests::context::test_pdf()),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.get("/article", None).await;
    ctx.unauth(status);
    let (status, _) = ctx.get("/article", Some(&ctx.malformed_session())).await;
    ctx.bad(status);
    let (status, _) = ctx.get("/article", Some(&ctx.ghost_session())).await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_empty_db_returns_empty_array() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx.get("/article", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        body["article_list"].as_array().map(Vec::len),
        Some(0),
        "空库列表必须是空数组"
    );
    assert_eq!(body["total"].as_u64(), Some(0), "空库 total 必须为 0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_returns_enriched_articles_newest_first() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let a = ctx
        .create_article(&session, "first", "s1", "#a", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let b = ctx
        .create_article(&session, "second", "s2", "#b#c", "1.0.0", "n")
        .await
        .0;
    separate_creation();
    let c = ctx
        .create_article(&session, "third", "s3", "#c", "1.0.0", "n")
        .await
        .0;

    let (status, body) = ctx.get("/article", Some(&session)).await;
    ctx.ok(status);
    let ids = article_ids(&body);
    assert_eq!(
        ids,
        vec![c.clone(), b.clone(), a.clone()],
        "列表必须按创建倒序（最新在前）"
    );
    assert_eq!(body["total"].as_u64(), Some(3));

    let first = &body["article_list"][0];
    assert!(
        !first["id"].as_str().unwrap().contains(':'),
        "id 必须剥表前缀"
    );
    assert_eq!(first["author_id"].as_str().unwrap(), user_id);
    assert_eq!(
        first["author_name"].as_str().unwrap(),
        user_id.replace('-', ""),
        "未设名时 author_name = 默认名（user_id 去横线）"
    );
    assert_eq!(first["title"].as_str().unwrap(), "third");
    assert_eq!(first["summary"].as_str().unwrap(), "s3");
    assert!(
        first.get("created_at").is_none(),
        "列表条目不含 created_at（仅详情含）"
    );
    let tag_names: Vec<&str> = first["tags"]
        .as_array()
        .expect("列表条目必须含 tags 数组")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(tag_names, vec!["#c"], "列表条目 tags 必须含创建的标签名");
    assert!(first.get("versions").is_none(), "列表条目不得内嵌版本列表");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_uses_default_page_size_and_clamps_pagination() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    for i in 0..9 {
        ctx.create_article(&session, &format!("article {i}"), "s", "#x", "1.0.0", "n")
            .await;
        separate_creation();
    }
    let (status, body) = ctx.get("/article", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        article_ids(&body).len(),
        8,
        "limit 缺省必须 = search_page_size(8)"
    );
    assert_eq!(body["has_more"].as_bool(), Some(true), "第 1 页后还有 1 条");
    assert_eq!(body["total"].as_u64(), Some(9), "total 必须为 9");
    let (status, body) = ctx.get("/article?limit=10000", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        article_ids(&body).len(),
        9,
        "limit 超过上限按 200 封顶而非报错"
    );
    let (status, body) = ctx.get("/article?limit=0", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(article_ids(&body).len(), 1, "limit=0 必须钳到 1");
    let (status, body_p0) = ctx.get("/article?page=0", Some(&session)).await;
    ctx.ok(status);
    let (status, body_p1) = ctx.get("/article?page=1", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        article_ids(&body_p0),
        article_ids(&body_p1),
        "page=0 必须与 page=1 同页"
    );
    let (status, body) = ctx.get("/article?page=999999", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        article_ids(&body).len(),
        0,
        "page 超 max_page 被 clamp 后返回空页而非错误"
    );
    assert_eq!(body["total"].as_u64(), Some(9), "空页仍带正确 total");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let fields = create_fields("t", "s", "#x");
    let (status, _) = ctx.multipart("POST", "/article", &fields, None).await;
    ctx.unauth(status);
    let (status, _) = ctx
        .multipart("POST", "/article", &fields, Some(&ctx.malformed_session()))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_missing_or_blank_title() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#x".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "initial".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("   ", "s", "#x"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_overlong_and_newline_fields() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let long_title = "a".repeat(201);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields(&long_title, "s", "#x"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let long_summary = "b".repeat(2001);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("t", &long_summary, "#x"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("bad\ntitle", "s", "#x"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_invalid_tags() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let too_many = "#a#b#c#d#e#f#g#h#i";
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("t", "s", too_many),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("t", "s", "lebron"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields("t", "s", ""),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("at least one tag"),
        "空 tags 的 reason 应含至少一个 tag 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_title_over_text_field_bytes() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let huge = "a".repeat(1_048_577);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &create_fields(&huge, "s", "#x"),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_missing_version_or_note_or_file() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "t".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#x".as_bytes().to_vec()),
                ("note", "initial".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("version"),
        "缺 version 的 reason 应含 version 语义，实际: {}",
        ctx.reason(&body)
    );
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "t".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#x".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "t".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#x".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "initial".as_bytes().to_vec()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("PDF"),
        "缺 file 的 reason 应含 PDF 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_ok_and_detail_roundtrip_trims_fields() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "  hi  ".as_bytes().to_vec()),
                ("summary", "  world  ".as_bytes().to_vec()),
                ("tags", "#rust#tokio".as_bytes().to_vec()),
                ("version", "  1.0.0  ".as_bytes().to_vec()),
                ("note", "  initial  ".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.created(status);
    let article_id = body["article_id"]
        .as_str()
        .expect("必须返回 article_id")
        .to_string();
    let version_id = body["version_id"]
        .as_str()
        .expect("create 响应必须带 version_id（建文即带首版本）")
        .to_string();
    assert!(!article_id.contains(':'), "article_id 必须是裸 uuid");
    assert!(!version_id.contains(':'), "version_id 必须是裸 uuid");

    let (status, body) = ctx
        .get(&format!("/article/{article_id}"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(body["id"].as_str().unwrap(), article_id);
    assert_eq!(body["author_id"].as_str().unwrap(), user_id);
    assert_eq!(body["title"].as_str().unwrap(), "hi", "title 必须 trim");
    assert_eq!(
        body["summary"].as_str().unwrap(),
        "world",
        "summary 必须 trim"
    );
    assert!(
        body["created_at"].as_u64().is_some(),
        "详情必须含 created_at 秒时间戳"
    );
    let tags = body["tags"].as_array().expect("详情必须含 tags 数组");
    let tag_names: Vec<&str> = tags.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(
        tag_names,
        vec!["#rust", "#tokio"],
        "详情 tags 必须含创建时的两个标签名（name 带 # 前缀）"
    );
    assert!(body.get("versions").is_none(), "详情响应不得含版本列表");
    let (status, body) = ctx
        .get(&format!("/version/{version_id}"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(body["version"].as_str().unwrap(), "1.0.0");
    assert_eq!(body["note"].as_str().unwrap(), "initial", "note 必须 trim");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn detail_with_404_for_missing_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .get(&format!("/article/{}", Uuid::now_v7()), Some(&session))
        .await;
    ctx.not_found(status);
}

fn count_storage(admin: &std::path::Path) -> (usize, usize) {
    let mut placed = 0usize;
    let mut tmp_children = 0usize;
    let mut pending: Vec<std::path::PathBuf> = vec![admin.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().map(|n| n == ".tmp").unwrap_or(false) {
                    tmp_children = std::fs::read_dir(&p).map(|r| r.count()).unwrap_or(0);
                } else {
                    pending.push(p);
                }
            } else if p.extension().map(|n| n == "pdf").unwrap_or(false) {
                placed += 1;
            }
        }
    }
    (placed, tmp_children)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_invalid_pdf_with_no_file_leftover() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let admin = ctx.pdf_storage_path().to_path_buf();

    let (placed0, tmp0) = count_storage(&admin);
    assert_eq!((placed0, tmp0), (0, 0), "初始必须有 0 落盘、0 临时文件");

    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "bad pdf create".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#bad".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", b"this file is certainly not a pdf".to_vec()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        status.as_u16() < 500,
        "非法 PDF 必须是 4xx 而非 5xx: {body}"
    );

    let (placed1, tmp1) = count_storage(&admin);
    assert_eq!((placed1, tmp1), (0, 0), "非法 PDF 建文不得留下任何文件");

    let (status, list) = ctx.get("/article", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        list["article_list"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        0,
        "非法 PDF 建文不得创建任何文章: {list}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_duplicate_pdf_content_no_dup_file() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let admin = ctx.pdf_storage_path().to_path_buf();

    let content = crate::unit_tests::context::test_pdf_variant("dedup-seed");

    let (status, _body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "dedup first".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#dedup".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", content.clone()),
            ],
            Some(&session),
        )
        .await;
    ctx.created(status);
    let (placed1, tmp1) = count_storage(&admin);
    assert_eq!((placed1, tmp1), (1, 0), "首建文应落盘 1 份、`.tmp` 空");

    let (_user2, session2) = ctx.register("bob@qq.com").await;
    let (status, body) = ctx
        .multipart(
            "POST",
            "/article",
            &[
                ("title", "dedup second".as_bytes().to_vec()),
                ("summary", "s".as_bytes().to_vec()),
                ("tags", "#dedup".as_bytes().to_vec()),
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", content),
            ],
            Some(&session2),
        )
        .await;
    ctx.bad(status);
    let reason = body["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("identical PDF already exists"),
        "去重应报 identical PDF: {body}"
    );

    let (placed2, tmp2) = count_storage(&admin);
    assert_eq!(
        (placed2, tmp2),
        (1, 0),
        "同内容去重后不得重复落盘、不得残留临时文件 (user_id={user_id})"
    );

    let (status, list) = ctx.get("/article", Some(&session2)).await;
    ctx.ok(status);
    let titles: Vec<&str> = list["article_list"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v["title"].as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        !titles.iter().any(|t| *t == "dedup second"),
        "被判重内容不得另建文章: {list}"
    );
    assert_eq!(
        list["total"].as_u64().unwrap_or(0),
        1,
        "判重后全局仍应只有 1 篇文章: {list}"
    );
}
