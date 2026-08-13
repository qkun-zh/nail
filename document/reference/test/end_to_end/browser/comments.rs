
use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

#[tokio::test]
async fn comment_post_reply_delete_via_ui() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let email = format!("cmt_{}@qq.com", uuid::Uuid::now_v7());
    let session = ctx.login_via_ui(&email).await;

    let title = format!("CommentTest{}", uuid::Uuid::now_v7());
    let (article_id, _version_id) = ctx
        .create_article(&session, &title, "cmt summary", "#cmt", "1.0.0", "initial")
        .await;
    let version_id = ctx.first_version_id(&session, &article_id).await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/{version_id}/comment",
            ctx.base_url
        ))
        .await
        .expect("goto version comment page");
    ctx.wait_for_text("comment", 10).await;

    let comment_body = "hello from browser e2e";
    ctx.type_retry(
        "textarea[placeholder=\"comment\"]",
        comment_body,
        "comment textarea",
    )
    .await;
    ctx.click_retry("form.cmt-form button[type=\"submit\"]", "comment submit")
        .await;
    ctx.wait_for_text("comment posted", 10).await;
    ctx.wait_for_text(comment_body, 10).await;

    let comment_id = ctx
        .first_comment_id(&session, &version_id, comment_body)
        .await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/{version_id}/comment/{comment_id}",
            ctx.base_url
        ))
        .await
        .expect("goto single comment page");
    ctx.wait_for_text(comment_body, 10).await;

    let reply_body = "a reply from browser e2e";
    ctx.type_retry(
        "textarea[placeholder=\"comment\"]",
        reply_body,
        "reply textarea",
    )
    .await;
    ctx.click_retry("form.cmt-form button[type=\"submit\"]", "reply submit")
        .await;
    ctx.wait_for_text("reply posted", 10).await;
    ctx.wait_for_text(reply_body, 10).await;

    let reply_id = ctx
        .first_comment_id(&session, &version_id, reply_body)
        .await;
    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/{version_id}/comment/{reply_id}/delete",
            ctx.base_url
        ))
        .await
        .expect("goto delete reply");
    ctx.wait_for_text("delete", 10).await;
    ctx.click_retry("form button[type=\"submit\"]", "delete reply submit")
        .await;
    ctx.wait_for_text("comment deleted", 10).await;
}
