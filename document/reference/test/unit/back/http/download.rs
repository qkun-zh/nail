
use crate::unit_tests::context::TestCtx;
use uuid::Uuid;

fn version_download_uri(article_id: &str, version_id: &str) -> String {
    format!("/article/{article_id}/version/{version_id}/download")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_and_consume_require_session() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .get(
            &version_download_uri(&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string()),
            None,
        )
        .await;
    ctx.unauth(status);
    let (status, _, _) = ctx
        .download(&format!("/article/download?token={}", Uuid::now_v7()), None)
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_returns_with_404_for_missing_article_or_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let uri = version_download_uri(&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string());
    let (status, _) = ctx.get(&uri, Some(&session)).await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_rejects_version_not_belonging_to_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_a, version_a) = ctx.seed_article(&session).await;
    let article_b = ctx
        .create_article(
            &session,
            "other title",
            "other summary",
            "#other",
            "1.0.0",
            "n",
        )
        .await
        .0;
    let uri = version_download_uri(&article_b, &version_a);
    let (status, _) = ctx.get(&uri, Some(&session)).await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_and_consume_roundtrip_is_single_use() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let (status, body) = ctx
        .get(
            &version_download_uri(&article_id, &version_id),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    let url = body["url"].as_str().unwrap().to_string();
    assert!(
        url.starts_with("/api/article/download?token="),
        "mint 必须返回携带 token 的站内单次消费 URL，实际: {url}"
    );

    let consume_uri = url.strip_prefix("/api").unwrap().to_string();
    let (status, headers, bytes) = ctx.download(&consume_uri, Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        headers.get("content-type").and_then(|v| v.to_str().ok()),
        Some("application/pdf"),
        "PDF 响应必须是 application/pdf"
    );
    assert_eq!(
        bytes,
        crate::unit_tests::context::test_pdf_variant("seed title|1.0.0"),
        "下载字节必须等于落盘的首版本 PDF"
    );

    let (status, _, _) = ctx.download(&consume_uri, Some(&session)).await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_rejects_unminted_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _, _) = ctx
        .download(
            &format!("/article/download?token={}", Uuid::now_v7()),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_rejects_token_minted_for_another_user() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&alice).await;

    let (status, body) = ctx
        .get(
            &version_download_uri(&article_id, &version_id),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    let consume_uri = body["url"]
        .as_str()
        .unwrap()
        .strip_prefix("/api")
        .unwrap()
        .to_string();

    let (status, _, _) = ctx.download(&consume_uri, Some(&bob)).await;
    ctx.bad(status);
}
