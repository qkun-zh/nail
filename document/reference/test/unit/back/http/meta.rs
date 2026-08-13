
use crate::unit_tests::context::TestCtx;

const SENSITIVE_KEYS: [&str; 13] = [
    "password",
    "listen_addr",
    "db_path",
    "pdf_storage_path",
    "smtp",
    "username",
    "host",
    "port",
    "from_email",
    "db_namespace",
    "db_database",
    "email",
    "allowed_domains",
];

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fetch_limits_matches_server_config_without_session() {
    let ctx = TestCtx::new().await;
    let (status, body) = ctx.get("/meta/limits", None).await;
    ctx.ok(status);
    let s = &ctx.state.config.server;
    assert_eq!(
        body["max_tags_per_article"].as_u64().unwrap() as usize,
        s.max_tags_per_article,
        "max_tags_per_article 必须与 conf 一致"
    );
    assert_eq!(
        body["max_comment_body_chars"].as_u64().unwrap(),
        s.max_comment_body_chars,
        "max_comment_body_chars 必须与 conf 一致"
    );
    assert_eq!(
        body["max_version_note_chars"].as_u64().unwrap(),
        s.max_version_note_chars,
        "max_version_note_chars 必须与 conf 一致"
    );
    assert_eq!(
        body["max_title_chars"].as_u64().unwrap(),
        s.max_title_chars,
        "max_title_chars 必须与 conf 一致"
    );
    assert_eq!(
        body["max_summary_chars"].as_u64().unwrap(),
        s.max_summary_chars,
        "max_summary_chars 必须与 conf 一致"
    );
    assert_eq!(
        body["max_pdf_size_bytes"].as_u64().unwrap(),
        s.max_pdf_size_bytes,
        "max_pdf_size_bytes 必须与 conf 一致"
    );
    assert_eq!(
        body["search_page_size"].as_u64().unwrap(),
        s.search_page_size,
        "search_page_size 必须与 conf 一致"
    );
    assert_eq!(
        body["max_search_pages"].as_u64().unwrap(),
        s.max_search_pages,
        "max_search_pages 必须与 conf 一致"
    );
    assert_eq!(
        body["max_page"].as_u64().unwrap(),
        s.max_page,
        "max_page 必须与 conf 一致"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fetch_limits_does_not_leak_sensitive_config() {
    let ctx = TestCtx::new().await;
    let (status, body) = ctx.get("/meta/limits", None).await;
    ctx.ok(status);
    let obj = body.as_object().expect("limits 必须是 JSON 对象");
    for key in SENSITIVE_KEYS {
        assert!(!obj.contains_key(key), "响应不得包含敏感字段 {key}");
    }
    let raw = serde_json::to_string(&body)
        .expect("serialize")
        .to_lowercase();
    assert!(!raw.contains("127.0.0.1"), "不得泄露 listen_addr");
    assert!(
        !raw.contains(&ctx.state.config.server.pdf_storage_path.to_lowercase()),
        "不得泄露 pdf_storage_path"
    );
}
