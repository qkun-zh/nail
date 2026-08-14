use super::context::{BrowserContext, TestBackend, session_token_from_storage};
use super::smtp_sink;
use std::time::Duration;

async fn body_text(context: &BrowserContext) -> String {
    context
        .page
        .evaluate("document.body.innerText")
        .await
        .expect("evaluate innerText")
        .into_value()
        .expect("innerText value")
}

async fn wait_for_text(context: &BrowserContext, needle: &str, timeout_secs: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let text = body_text(context).await;
        if text.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "page never contained {needle:?}; body so far: {:?}",
            text.chars().take(500).collect::<String>()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_absent(context: &BrowserContext, needle: &str, timeout_secs: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let text = body_text(context).await;
        if !text.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "page still contained {needle:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn pathname(context: &BrowserContext) -> String {
    context
        .page
        .evaluate("location.pathname")
        .await
        .expect("evaluate pathname")
        .into_value()
        .expect("pathname value")
}

async fn wait_for_href(context: &BrowserContext, needle: &str, timeout_secs: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let raw: String = context
            .page
            .evaluate("location.href")
            .await
            .expect("evaluate href")
            .into_value()
            .expect("href value");
        if raw.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "href never contained {needle:?}; href={raw:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn goto(context: &BrowserContext, path: &str) {
    context
        .page
        .goto(format!("{}{path}", context.frontend_url))
        .await
        .expect("goto page");
}

async fn click_link_with_text(context: &BrowserContext, text: &str) {
    let literal = serde_json::to_string(text).expect("json string literal");
    let script = format!(
        "(() => {{ const a = Array.from(document.querySelectorAll('a')).find(x => x.textContent.trim() === {literal}); if (a) {{ a.click(); return true; }} return false; }})()"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(value) = context.page.evaluate(script.as_str()).await
            && value.into_value::<bool>().unwrap_or(false)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never able to click link {text:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn click_button_with_text(context: &BrowserContext, text: &str) {
    let literal = serde_json::to_string(text).expect("json string literal");
    let script = format!(
        "(() => {{ const b = Array.from(document.querySelectorAll('button')).find(x => x.textContent.trim() === {literal}); if (b) {{ b.click(); return true; }} return false; }})()"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(value) = context.page.evaluate(script.as_str()).await
            && value.into_value::<bool>().unwrap_or(false)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never able to click button {text:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn click_submit(context: &BrowserContext, form_index: u32) {
    let selector = format!("form:nth-of-type({form_index}) button[type=submit]");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(element) = context.page.find_element(&selector).await
            && element.click().await.is_ok()
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never able to click submit of form {form_index}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn fill_input(context: &BrowserContext, selector: &str, text: &str) {
    let selector_literal = serde_json::to_string(selector).expect("json selector literal");
    let text_literal = serde_json::to_string(text).expect("json text literal");
    let script = format!(
        "(() => {{ const el = document.querySelector({selector_literal}); if (!el) return false; el.value = {text_literal}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); el.dispatchEvent(new Event('change', {{ bubbles: true }})); return true; }})()"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(value) = context.page.evaluate(script.as_str()).await
            && value.into_value::<bool>().unwrap_or(false)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never able to fill {selector:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn upload_pdf(context: &BrowserContext, path: &std::path::Path) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(element) = context.page.find_element("input[type=file]").await {
            let node_id = element.node_id;
            let params = chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams::builder()
                .file(path.to_string_lossy().to_string())
                .node_id(node_id)
                .build()
                .expect("build set file input files");
            if context.page.execute(params).await.is_ok() {
                return;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never able to upload pdf"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn unique_pdf(seed: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("pages_e2e_{seed}_{}.pdf", uuid::Uuid::now_v7()));
    std::fs::write(&path, format!("%PDF-1.4\n{seed}\n%%EOF\n")).expect("write pdf");
    path
}

async fn login(context: &BrowserContext, email: &str) -> String {
    goto(context, "/private/authenticate").await;
    wait_for_text(context, "authenticate", 20).await;
    fill_input(context, "form:nth-of-type(1) input", email).await;
    click_submit(context, 1).await;
    let mail_body = context.backend.wait_for_mail(email, 20).await;
    let token = smtp_sink::extract_token(&mail_body);
    fill_input(context, "form:nth-of-type(2) input", &token).await;
    click_submit(context, 2).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(session) = session_token_from_storage(&context.local_storage().await) {
            return session;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "session_token never landed in localStorage"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn inject_session(context: &BrowserContext, token: &str) {
    let literal = serde_json::to_string(token).expect("json token literal");
    let script = format!("localStorage.setItem('session_token', JSON.stringify({literal}))");
    context
        .page
        .evaluate(script.as_str())
        .await
        .expect("evaluate set session");
}

async fn logout_via_button(context: &BrowserContext) {
    goto(context, "/private/logout").await;
    wait_for_text(context, "logout", 20).await;
    click_button_with_text(context, "logout").await;
    wait_for_text(context, "who are you", 20).await;
    assert!(session_token_from_storage(&context.local_storage().await).is_none());
}

#[tokio::test]
async fn anonymous_pages_and_gates() {
    let context = BrowserContext::start().await;

    goto(&context, "/").await;
    wait_for_text(&context, "who are you", 20).await;
    wait_for_text(&context, "authenticate", 20).await;

    goto(&context, "/public").await;
    wait_for_text(&context, "who are you", 20).await;

    goto(&context, "/public/article").await;
    wait_for_text(&context, "who are you", 20).await;

    goto(&context, "/public/article/create").await;
    wait_for_text(&context, "who are you", 20).await;

    goto(&context, "/private").await;
    wait_for_text(&context, "who are you", 20).await;

    goto(&context, "/no/such/page").await;
    wait_for_text(&context, "not found", 20).await;
}

#[tokio::test]
async fn authenticate_form_and_session() {
    let context = BrowserContext::start().await;

    goto(&context, "/private/authenticate").await;
    wait_for_text(&context, "authenticate", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "enter your email", 20).await;

    fill_input(&context, "form:nth-of-type(1) input", "alice@example.com").await;
    click_submit(&context, 1).await;
    let mail_body = context.backend.wait_for_mail("alice@example.com", 20).await;
    let token = smtp_sink::extract_token(&mail_body);

    click_submit(&context, 2).await;
    wait_for_text(&context, "paste the emailed token", 20).await;

    fill_input(&context, "form:nth-of-type(2) input", &token).await;
    click_submit(&context, 2).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if session_token_from_storage(&context.local_storage().await).is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "session_token never landed"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    goto(&context, "/private").await;
    wait_for_text(&context, "name", 20).await;
    wait_for_text(&context, "logout", 20).await;
}

#[tokio::test]
async fn private_name_and_email_flows() {
    let context = BrowserContext::start().await;
    let session = login(&context, "alice@example.com").await;
    assert!(!session.is_empty());

    click_link_with_text(&context, "name").await;
    wait_for_href(&context, "/private/name", 20).await;
    wait_for_text(&context, "hi,", 20).await;

    click_link_with_text(&context, "update").await;
    wait_for_href(&context, "/private/name/update", 20).await;
    wait_for_text(&context, "update name", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "name cannot be empty", 20).await;

    fill_input(&context, "input[placeholder=\"name\"]", "alice-one").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "alice-one", 20).await;

    goto(&context, "/private/email").await;
    wait_for_text(&context, "send", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "enter both the old and the new email", 20).await;

    fill_input(&context, "input[placeholder=\"email(old)\"]", "same@example.com").await;
    fill_input(&context, "input[placeholder=\"email(new)\"]", "same@example.com").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "the new email must differ from the old one", 20).await;

    tokio::time::sleep(Duration::from_secs(2)).await;
    fill_input(&context, "input[placeholder=\"email(old)\"]", "alice@example.com").await;
    fill_input(&context, "input[placeholder=\"email(new)\"]", "alice-new@example.com").await;
    click_submit(&context, 1).await;
    let old_mail = context.backend.wait_for_mail("alice@example.com", 20).await;
    let new_mail = context.backend.wait_for_mail("alice-new@example.com", 20).await;
    let old_token = smtp_sink::extract_token(&old_mail);
    let new_token = smtp_sink::extract_token(&new_mail);

    wait_for_text(&context, "update", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "paste both emailed tokens", 20).await;

    fill_input(&context, "input[placeholder=\"token(old)\"]", &old_token).await;
    fill_input(&context, "input[placeholder=\"token(new)\"]", &old_token).await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "the two tokens must differ", 20).await;

    fill_input(&context, "input[placeholder=\"token(new)\"]", &new_token).await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "logout", 20).await;

    logout_via_button(&context).await;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let session_again = login(&context, "alice-new@example.com").await;
    assert!(!session_again.is_empty());
}

#[tokio::test]
async fn article_version_comment_flows() {
    let context = BrowserContext::start().await;
    let session = login(&context, "alice@example.com").await;
    assert!(!session.is_empty());

    goto(&context, "/public/article/create").await;
    wait_for_text(&context, "create article", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "text cannot be empty", 20).await;

    let pdf_a = unique_pdf("seed-a").await;
    fill_input(&context, "input[placeholder=\"title\"]", "probe title").await;
    fill_input(&context, "textarea[placeholder=\"summary\"]", "probe summary").await;
    fill_input(&context, "textarea[placeholder=\"tag (#a #b)\"]", "#probe").await;
    fill_input(&context, "input[placeholder=\"version\"]", "1.0.0").await;
    fill_input(&context, "textarea[placeholder=\"note: what changed in this version\"]", "initial note").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "select a PDF file", 20).await;

    upload_pdf(&context, &pdf_a).await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "probe title", 20).await;
    wait_for_text(&context, "version", 20).await;
    let article_href = pathname(&context).await;
    assert!(article_href.starts_with("/public/article/"), "{article_href}");

    click_link_with_text(&context, "version").await;
    wait_for_href(&context, "/version", 20).await;
    wait_for_text(&context, "create", 20).await;

    click_link_with_text(&context, "create").await;
    wait_for_text(&context, "create version", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "version is required", 20).await;

    let pdf_b = unique_pdf("seed-b").await;
    fill_input(&context, "input[placeholder=\"version\"]", "2.0.0").await;
    fill_input(&context, "textarea[placeholder=\"note: what changed in this version\"]", "second note").await;
    upload_pdf(&context, &pdf_b).await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "2.0.0", 20).await;

    click_link_with_text(&context, "2.0.0").await;
    wait_for_text(&context, "download", 20).await;
    wait_for_text(&context, "second note", 20).await;
    let version_2_detail = pathname(&context).await;

    click_link_with_text(&context, "download").await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    click_link_with_text(&context, "comment").await;
    wait_for_text(&context, "comment", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "text cannot be empty", 20).await;

    fill_input(&context, "form textarea:nth-of-type(1)", "first comment").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "comment created", 20).await;
    let comment_index = format!("{version_2_detail}/comment");
    goto(&context, &comment_index).await;
    wait_for_text(&context, "first comment", 20).await;

    click_link_with_text(&context, "reply").await;
    wait_for_href(&context, "/comment/", 20).await;
    wait_for_text(&context, "reply", 20).await;
    fill_input(&context, "form textarea:nth-of-type(1)", "a reply").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "reply created", 20).await;
    wait_for_text(&context, "a reply", 20).await;

    let reply_delete_href = reply_delete_href(&context).await;
    assert!(!reply_delete_href.is_empty());
    goto(&context, &reply_delete_href).await;
    wait_for_text(&context, "delete", 20).await;
    click_button_with_text(&context, "delete").await;
    wait_for_text(&context, "comment deleted", 20).await;
    wait_for_text(&context, "first comment", 20).await;
    wait_absent(&context, "a reply", 20).await;

    std::fs::remove_file(&pdf_a).ok();
    std::fs::remove_file(&pdf_b).ok();
}

async fn reply_delete_href(context: &BrowserContext) -> String {
    let raw: String = context
        .page
        .evaluate(
            "(() => { const del = Array.from(document.querySelectorAll('a[href$=\"/delete\"]')).find(a => a.parentElement && a.parentElement.parentElement && a.parentElement.parentElement.innerText.includes('a reply')); return del ? del.getAttribute('href') : ''; })()",
        )
        .await
        .expect("evaluate reply delete link")
        .into_value()
        .expect("reply delete link value");
    raw
}

#[tokio::test]
async fn search_pagination_and_author_gate() {
    let context = BrowserContext::start().await;
    let session = login(&context, "alice@example.com").await;
    assert!(!session.is_empty());

    for index in 0..9 {
        let pdf = unique_pdf(&format!("seed-{index}")).await;
        create_article_via_api(&context.backend, &session, &format!("bulk title {index}"), &pdf)
            .await;
        std::fs::remove_file(&pdf).ok();
    }
    let bulk_pdf = unique_pdf("bulk").await;
    let (article_id, _version_id) = create_article_via_api(
        &context.backend,
        &session,
        "probe unique searchable title",
        &bulk_pdf,
    )
    .await;
    std::fs::remove_file(&bulk_pdf).ok();

    goto(&context, "/public/article/search").await;
    wait_for_text(&context, "probe unique searchable title", 20).await;
    wait_for_text(&context, "next", 20).await;

    click_button_with_text(&context, "next").await;
    wait_for_href(&context, "page=2", 20).await;
    wait_for_text(&context, "prev", 20).await;

    click_button_with_text(&context, "prev").await;
    wait_for_href(&context, "page=1", 20).await;

    fill_input(&context, "form input[type=text]:nth-of-type(1)", "searchable").await;
    click_submit(&context, 1).await;
    wait_for_text(&context, "probe unique searchable title", 20).await;
    wait_for_text(&context, "searchable", 20).await;

    let bob_session = context.backend.authenticate("bob@example.com").await;
    inject_session(&context, &bob_session).await;
    goto(&context, &format!("/public/article/{article_id}/update")).await;
    wait_for_text(&context, "you are denied!", 20).await;

    inject_session(&context, &session).await;
    goto(&context, &format!("/public/article/{article_id}/delete")).await;
    wait_for_text(&context, "delete", 20).await;
    click_button_with_text(&context, "delete").await;
    wait_for_text(&context, "create", 20).await;
    wait_absent(&context, "probe unique searchable title", 20).await;
}

async fn create_article_via_api(
    backend: &TestBackend,
    session: &str,
    title: &str,
    pdf: &std::path::PathBuf,
) -> (String, String) {
    let form = reqwest::multipart::Form::new()
        .text("title", title.to_string())
        .text("summary", "api summary".to_string())
        .text("tags", "#bulk".to_string())
        .text("version", "1.0.0".to_string())
        .text("note", "api note".to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(std::fs::read(pdf).expect("read pdf"))
                .file_name("bulk.pdf"),
        );
    let response = backend
        .client
        .post(format!("{}/article/create", backend.base_url))
        .header("session-token", session)
        .multipart(form)
        .send()
        .await
        .expect("POST article/create");
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let json = response
        .json::<serde_json::Value>()
        .await
        .expect("article json");
    (
        json["data"]["article_id"].as_str().expect("article_id").to_string(),
        json["data"]["version_id"].as_str().expect("version_id").to_string(),
    )
}

#[tokio::test]
async fn deregister_flow() {
    let context = BrowserContext::start().await;
    let session = login(&context, "alice@example.com").await;
    assert!(!session.is_empty());

    goto(&context, "/private/deregister").await;
    wait_for_text(&context, "deregister", 20).await;

    click_submit(&context, 1).await;
    wait_for_text(&context, "enter your account email", 20).await;

    tokio::time::sleep(Duration::from_secs(2)).await;
    fill_input(&context, "input[placeholder=\"email\"]", "alice@example.com").await;
    click_submit(&context, 1).await;
    let mail_body = context.backend.wait_for_mail("alice@example.com", 20).await;
    let token = smtp_sink::extract_token(&mail_body);

    fill_input(&context, "input[placeholder=\"token\"]", &token).await;
    click_submit(&context, 2).await;
    wait_for_text(&context, "who are you", 20).await;
    assert!(session_token_from_storage(&context.local_storage().await).is_none());
}

