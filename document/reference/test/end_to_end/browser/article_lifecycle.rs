
use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

fn unique(prefix: &str) -> String {
    format!("{prefix}{}", uuid::Uuid::now_v7())
}

async fn replace_input(ctx: &EndToEndBrowserContext, selector: &str, text: &str) {
    let js = format!(
        "(() => {{ const el = document.querySelector({selector:?}); if (!el) {{ return false; }} el.value = {text:?}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"
    );
    let ok: bool = ctx
        .page
        .evaluate(js)
        .await
        .expect("evaluate replace input")
        .into_value()
        .expect("replace input result");
    assert!(ok, "input not found: {selector}");
}

async fn create_article_via_ui(
    ctx: &EndToEndBrowserContext,
    title: &str,
    summary: &str,
    tags: &str,
    version: &str,
    note: &str,
) {
    ctx.page
        .goto(format!("{}/public/article/create", ctx.base_url))
        .await
        .expect("goto create");
    ctx.wait_for_text("create article", 10).await;
    ctx.type_retry("input[placeholder=\"title\"]", title, "create title")
        .await;
    ctx.type_retry(
        "textarea[placeholder=\"summary\"]",
        summary,
        "create summary",
    )
    .await;
    ctx.type_retry("textarea[placeholder=\"tag\"]", tags, "create tags")
        .await;
    ctx.type_retry("input[placeholder=\"version\"]", version, "create version")
        .await;
    ctx.type_retry(
        "textarea[placeholder=\"note: what changed in this version\"]",
        note,
        "create note",
    )
    .await;
    let pdf = ctx.write_pdf_temp(title).await;
    ctx.set_file_input("#article_pdf", &pdf).await;
    ctx.click_retry("form button[type=\"submit\"]", "create submit")
        .await;
    ctx.wait_for_text("article created", 10).await;
}

#[tokio::test]
async fn article_full_lifecycle_via_ui() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = format!("lifecycle_{}@qq.com", uuid::Uuid::now_v7());
    let session = ctx.login_via_ui(&email).await;

    let title = unique("Lifecycle");
    let summary = "lifecycle summary";
    create_article_via_ui(&ctx, &title, summary, "#lif", "1.0.0", "first version").await;
    let article_id = ctx.search_article_id_by_title(&session, &title).await;

    ctx.page
        .goto(format!("{}/public/article/{article_id}", ctx.base_url))
        .await
        .expect("goto detail");
    ctx.wait_for_text(&format!("title){title}"), 10).await;
    let text = ctx.body_text().await;
    assert!(
        text.contains(&format!("summary){summary}")),
        "detail summary missing"
    );
    assert!(text.contains("tag)#lif"), "detail tag missing");
    assert!(text.contains("version"), "detail version link missing");

    let version_id = ctx.first_version_id(&session, &article_id).await;
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version",
            ctx.base_url
        ))
        .await
        .expect("goto version list");
    ctx.wait_for_text("create", 10).await;
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/{version_id}",
            ctx.base_url
        ))
        .await
        .expect("goto version page");
    ctx.wait_for_text("download", 10).await;
    let vtext = ctx.body_text().await;
    assert!(vtext.contains("first version"), "version note missing");
    assert!(vtext.contains("comment"), "comment link missing");

    let v2note = "second version added";
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/create",
            ctx.base_url
        ))
        .await
        .expect("goto version create");
    ctx.wait_for_text("create version", 10).await;
    ctx.type_retry("input[placeholder=\"version\"]", "2.0.0", "v2 number")
        .await;
    ctx.type_retry(
        "textarea[placeholder=\"note: what changed in this version\"]",
        v2note,
        "v2 note",
    )
    .await;
    let pdf2 = ctx.write_pdf_temp(&format!("{title}v2")).await;
    ctx.set_file_input("#version_pdf", &pdf2).await;
    ctx.click_retry("form button[type=\"submit\"]", "version submit")
        .await;
    ctx.wait_for_text("version created", 10).await;

    let new_title = unique("Updated");
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/update",
            ctx.base_url
        ))
        .await
        .expect("goto update");
    ctx.wait_for_text("save", 10).await;
    replace_input(&ctx, "input[placeholder=\"title\"]", &new_title).await;
    replace_input(&ctx, "textarea[placeholder=\"summary\"]", "updated summary").await;
    replace_input(&ctx, "textarea[placeholder=\"tag (#a #b)\"]", "#upd").await;
    ctx.click_retry("form button[type=\"submit\"]", "update submit")
        .await;
    ctx.wait_for_text("article updated", 10).await;
    ctx.page
        .goto(format!("{}/public/article/{article_id}", ctx.base_url))
        .await
        .expect("goto detail after update");
    ctx.wait_for_text(&format!("title){new_title}"), 10).await;
    let after = ctx.body_text().await;
    assert!(
        after.contains("summary)updated summary"),
        "updated summary missing"
    );
    assert!(after.contains("tag)#upd"), "updated tag missing");

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/delete",
            ctx.base_url
        ))
        .await
        .expect("goto delete");
    ctx.wait_for_text("delete", 10).await;
    ctx.click_retry("form button[type=\"submit\"]", "delete submit")
        .await;
    ctx.wait_for_text("article deleted", 10).await;
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/delete",
            ctx.base_url
        ))
        .await
        .expect("goto delete again");
    ctx.wait_for_text("you are denied", 10).await;
}
