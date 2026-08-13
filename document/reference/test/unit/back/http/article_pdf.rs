
use crate::unit_tests::context::TestCtx;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pdf_serve_requires_session() {
    let ctx = TestCtx::new().await;
    let (status, _, _) = ctx.download("/article/x/version/y/pdf", None).await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pdf_serve_with_404_for_missing_article_or_wrong_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_a, version_a) = ctx.seed_article(&session).await;
    let article_b = ctx
        .create_article(&session, "other", "s", "#b", "1.0.0", "n")
        .await
        .0;
    let (status, _, _) = ctx
        .download(
            &format!("/article/{}/version/{version_a}/pdf", Uuid::now_v7()),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
    let (status, _, _) = ctx
        .download(
            &format!("/article/{article_b}/version/{version_a}/pdf"),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pdf_serve_ok_returns_exact_pdf_bytes() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let (status, headers, bytes) = ctx
        .download(
            &format!("/article/{article_id}/version/{version_id}/pdf"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        headers.get("content-type").and_then(|v| v.to_str().ok()),
        Some("application/pdf"),
        "Content-Type 必须是 application/pdf"
    );
    assert_eq!(
        bytes,
        crate::unit_tests::context::test_pdf_variant("seed title|1.0.0"),
        "PDF 字节必须等于落盘的首版本 PDF"
    );
}
