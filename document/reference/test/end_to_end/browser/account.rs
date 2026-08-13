
use std::time::{Duration, Instant};

use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

fn unique_email(prefix: &str) -> String {
    format!("{prefix}_{}@qq.com", uuid::Uuid::now_v7())
}

async fn wait_session_cleared(ctx: &EndToEndBrowserContext) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let storage = ctx.local_storage().await;
        if EndToEndBrowserContext::session_token_from_storage(&storage).is_none() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "session_token never cleared; storage: {storage}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn name_update_email_check_logout_via_ui() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = unique_email("acct");
    ctx.login_via_ui(&email).await;

    let short = format!("{}", uuid::Uuid::now_v7().simple());
    let new_name = format!("nm_{}", &short[..16]);
    ctx.page
        .goto(format!("{}/private/name/update", ctx.base_url))
        .await
        .expect("goto name update");
    ctx.wait_for_text("update name", 10).await;
    ctx.type_retry(
        "input[placeholder=\"new name\"]",
        &new_name,
        "new name input",
    )
    .await;
    ctx.click_retry("form button[type=\"submit\"]", "name update submit")
        .await;
    ctx.wait_for_text(&format!("name updated to {new_name}"), 10)
        .await;

    ctx.page
        .goto(format!("{}/private/name", ctx.base_url))
        .await
        .expect("goto name");
    ctx.wait_for_text(&format!("hi, {new_name}!"), 10).await;

    ctx.page
        .goto(format!("{}/private/email", ctx.base_url))
        .await
        .expect("goto email index");
    ctx.wait_for_text("check", 10).await;
    assert!(
        ctx.body_text().await.contains("update"),
        "email index missing update link"
    );

    ctx.page
        .goto(format!("{}/private/email/check", ctx.base_url))
        .await
        .expect("goto email check");
    ctx.wait_for_text("check", 10).await;
    ctx.type_retry("input[type=\"email\"]", &email, "check email input")
        .await;
    ctx.click_retry("form button[type=\"submit\"]", "check submit")
        .await;
    ctx.wait_for_text("email matches", 10).await;

    ctx.page
        .goto(format!("{}/private/logout", ctx.base_url))
        .await
        .expect("goto logout");
    ctx.wait_for_text("logout", 10).await;
    ctx.click_retry("button.logout-action", "logout button")
        .await;
    wait_session_cleared(&ctx).await;
    ctx.wait_for_text("who are you", 10).await;
}

#[tokio::test]
async fn email_update_via_ui() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let old_email = unique_email("upd");
    let new_email = unique_email("new");
    ctx.login_via_ui(&old_email).await;

    ctx.page
        .goto(format!("{}/private/email/update", ctx.base_url))
        .await
        .expect("goto email update");
    ctx.wait_for_text("send", 10).await;
    ctx.type_retry(
        "input[placeholder=\"email(old)\"]",
        &old_email,
        "old email input",
    )
    .await;
    ctx.type_retry(
        "input[placeholder=\"email(new)\"]",
        &new_email,
        "new email input",
    )
    .await;
    ctx.click_retry("form button[type=\"submit\"]", "email send submit")
        .await;

    let old_mail = ctx.wait_for_mail(&old_email, 10).await;
    let new_mail = ctx.wait_for_mail(&new_email, 10).await;
    let old_token = super::super::extract_token(&old_mail);
    let new_token = super::super::extract_token(&new_mail);

    ctx.type_retry(
        "input[placeholder=\"token(old)\"]",
        &old_token,
        "old token input",
    )
    .await;
    ctx.type_retry(
        "input[placeholder=\"token(new)\"]",
        &new_token,
        "new token input",
    )
    .await;
    ctx.click_retry(
        "form:nth-of-type(2) button[type=\"submit\"]",
        "email confirm submit",
    )
    .await;
    ctx.wait_for_text("email updated", 10).await;
}

#[tokio::test]
async fn deregister_via_ui() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = unique_email("dereg");
    ctx.login_via_ui(&email).await;

    ctx.page
        .goto(format!("{}/private/deregister", ctx.base_url))
        .await
        .expect("goto deregister");
    ctx.wait_for_text("send", 10).await;
    ctx.type_retry(
        "input[placeholder=\"email\"]",
        &email,
        "deregister email input",
    )
    .await;
    ctx.click_retry("form button[type=\"submit\"]", "deregister send submit")
        .await;
    let mail = ctx.wait_for_mail(&email, 10).await;
    let token = super::super::extract_token(&mail);

    ctx.type_retry(
        "input[placeholder=\"token\"]",
        &token,
        "deregister token input",
    )
    .await;
    ctx.click_retry(
        "form:nth-of-type(2) button[type=\"submit\"]",
        "deregister confirm submit",
    )
    .await;
    ctx.wait_for_text("account deregistered", 10).await;
    wait_session_cleared(&ctx).await;
    ctx.wait_for_text("who are you", 10).await;
}
