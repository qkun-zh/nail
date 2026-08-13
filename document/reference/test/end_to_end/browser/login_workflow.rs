
use std::time::{Duration, Instant};

use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

fn unique_email(prefix: &str) -> String {
    format!("{prefix}_{}@qq.com", uuid::Uuid::now_v7())
}

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

#[tokio::test]
async fn probe_authenticate_email_updates_url_live() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;

    ctx.page
        .goto(format!("{}/private/authenticate", ctx.base_url))
        .await
        .expect("goto authenticate");
    ctx.wait_for_text("send", 10).await;

    ctx.type_retry("input[type=\"email\"]", "probe@qq.com", "email input")
        .await;
    wait_for_url_fragment(&ctx, "email=probe%40qq.com").await;

    ctx.page.reload().await.expect("reload authenticate");
    ctx.wait_for_text("send", 10).await;
    assert_eq!(
        input_value(&ctx, "input[type=\"email\"]").await,
        "probe@qq.com",
        "authenticate email input lost value after reload"
    );
}

#[tokio::test]
async fn home_page_renders_login_guide_when_anonymous() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    ctx.page
        .goto(ctx.base_url.clone())
        .await
        .expect("goto home");
    ctx.wait_for_text("who are you ?", 10).await;
    assert!(ctx.body_text().await.contains("authenticate"));
}

#[tokio::test]
async fn email_login_stores_session_in_local_storage() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_ui(&unique_email("login_ui")).await;
    let storage = ctx.local_storage().await;
    assert_eq!(
        EndToEndBrowserContext::session_token_from_storage(&storage).unwrap_or_default(),
        session
    );
}

#[tokio::test]
async fn private_landing_shows_account_links_after_login() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let _session = ctx.login_via_ui(&unique_email("login_landing")).await;
    ctx.page
        .goto(format!("{}/private", ctx.base_url))
        .await
        .expect("goto private");
    ctx.wait_for_text("deregister", 10).await;
    let text = ctx.body_text().await;
    for link in ["name", "email", "logout", "deregister"] {
        assert!(text.contains(link), "private landing missing {link:?}");
    }
}

#[tokio::test]
async fn name_page_shows_default_greeting() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let _session = ctx.login_via_ui(&unique_email("login_name")).await;
    ctx.page
        .goto(format!("{}/private/name", ctx.base_url))
        .await
        .expect("goto name");
    ctx.wait_for_text("hi, ", 10).await;
    let text = ctx.body_text().await;
    assert!(text.contains("!"), "greeting must end with !");
    assert!(!text.contains("who are you"), "must be logged in");
}
