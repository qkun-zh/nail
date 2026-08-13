
use std::time::{Duration, Instant};

use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

async fn wait_for_url_fragment(ctx: &EndToEndBrowserContext, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let url = ctx
            .page
            .url()
            .await
            .expect("read page url")
            .unwrap_or_default();
        if url.contains(needle) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "URL never contained {needle:?}; url so far: {url}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn input_value(ctx: &EndToEndBrowserContext, selector: &str) -> String {
    let raw: String = ctx
        .page
        .evaluate(format!("document.querySelector({selector:?}).value"))
        .await
        .expect("evaluate input value")
        .into_value()
        .expect("input value");
    raw
}
fn unique(prefix: &str) -> String {
    format!("{prefix}{}", uuid::Uuid::now_v7())
}

async fn seed_one(ctx: &EndToEndBrowserContext) -> (String, String, String) {
    let email = format!("seed_{}@qq.com", uuid::Uuid::now_v7());
    let session = ctx.login_via_ui(&email).await;
    let title = unique("Browser Title ");
    let article_id = ctx
        .create_article(
            &session,
            &title,
            "browser summary",
            "#browser",
            "1.0.0",
            "initial",
        )
        .await
        .0;
    (session, article_id, title)
}

#[tokio::test]
async fn article_list_page_renders_seeded_title() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let (_session, _article_id, title) = seed_one(&ctx).await;

    ctx.page
        .goto(format!("{}/public/article/search", ctx.base_url))
        .await
        .expect("goto search");
    ctx.wait_for_text(&title, 10).await;
}

#[tokio::test]
async fn article_detail_page_renders_metadata() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let (_session, article_id, title) = seed_one(&ctx).await;

    ctx.page
        .goto(format!("{}/public/article/{article_id}", ctx.base_url))
        .await
        .expect("goto detail");
    ctx.wait_for_text(&format!("title){title}"), 10).await;
    let text = ctx.body_text().await;
    assert!(
        text.contains("summary)browser summary"),
        "detail missing summary marker"
    );
}

#[tokio::test]
async fn search_box_filters_by_title() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let (_session, _article_id, title) = seed_one(&ctx).await;

    ctx.page
        .goto(format!("{}/public/article/search", ctx.base_url))
        .await
        .expect("goto search");
    ctx.type_retry("input[type=text]", &title, "search input")
        .await;
    ctx.press_enter_retry("input[type=text]", "search").await;

    ctx.wait_for_text(&title, 10).await;

    let cleared: bool = ctx
        .page
        .evaluate(
            "(() => { const el = document.querySelector('input[type=text]'); if (!el) return false; el.value=''; el.dispatchEvent(new Event('input',{bubbles:true})); return true; })()",
        )
        .await
        .expect("clear search box")
        .into_value()
        .expect("clear search box value");
    assert!(cleared, "search input not found");
    ctx.type_retry("input[type=text]", "NoSuchTermXYZ", "search input")
        .await;
    ctx.press_enter_retry("input[type=text]", "search").await;
    ctx.wait_for_text("none", 10).await;
}

#[tokio::test]
async fn pagination_shows_when_many_articles() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = format!("seed_paging_{}@qq.com", uuid::Uuid::now_v7());
    let session = ctx.login_via_ui(&email).await;
    for i in 0..9 {
        ctx.create_article(
            &session,
            &unique(&format!("Paging Title {i} ")),
            "paging summary",
            "#paging",
            "1.0.0",
            "initial",
        )
        .await;
    }

    ctx.page
        .goto(format!("{}/public/article/search", ctx.base_url))
        .await
        .expect("goto search");
    ctx.wait_for_text("/ 2", 10).await;
    let text = ctx.body_text().await;
    assert!(text.contains("next"), "pagination next button missing");
    assert!(text.contains("prev"), "pagination prev button missing");
}

#[tokio::test]
async fn probe_search_title_updates_url_live() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;

    ctx.login_via_ui(&(unique("probe_search_") + "@qq.com"))
        .await;

    ctx.page
        .goto(format!("{}/public/article/search", ctx.base_url))
        .await
        .expect("goto search");
    ctx.wait_for_text("search", 10).await;

    ctx.type_retry("input[type=text]", "hello-live", "search input")
        .await;

    wait_for_url_fragment(&ctx, "q=hello-live").await;

    ctx.page.reload().await.expect("reload search");
    ctx.wait_for_text("search", 10).await;
    assert_eq!(
        input_value(&ctx, "input[type=text]").await,
        "hello-live",
        "search input lost value after reload"
    );
}

#[tokio::test]
async fn probe_create_article_inputs_update_url_live() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = format!("probe_create_{}@qq.com", uuid::Uuid::now_v7());
    ctx.login_via_ui(&email).await;

    ctx.page
        .goto(format!("{}/public/article/create", ctx.base_url))
        .await
        .expect("goto create");
    ctx.wait_for_text("create article", 10).await;

    ctx.type_retry("input[placeholder=\"title\"]", "Live Title", "title input")
        .await;
    wait_for_url_fragment(&ctx, "title=Live%20Title").await;

    ctx.type_retry("input[placeholder=\"version\"]", "2.1.0", "version input")
        .await;
    wait_for_url_fragment(&ctx, "version=2.1.0").await;

    ctx.page.reload().await.expect("reload create");
    ctx.wait_for_text("create article", 10).await;
    assert_eq!(
        input_value(&ctx, "input[placeholder=\"title\"]").await,
        "Live Title",
        "create title not restored after reload"
    );
}
