
use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

fn unique_email(prefix: &str) -> String {
    format!("{prefix}_{}@qq.com", uuid::Uuid::now_v7())
}

#[tokio::test]
async fn non_author_is_denied_on_write_pages() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;

    let author_email = unique_email("auth");
    let session_a = ctx.login_via_ui(&author_email).await;
    let title = format!("AuthorGate{}", uuid::Uuid::now_v7());
    let (article_id, _) = ctx
        .create_article(
            &session_a,
            &title,
            "gate summary",
            "#gate",
            "1.0.0",
            "initial",
        )
        .await;
    let version_id = ctx.first_version_id(&session_a, &article_id).await;
    let comment_content = format!("gate comment {}", uuid::Uuid::now_v7());
    let resp = ctx
        .client
        .post(format!(
            "{}/api/version/{}/comments",
            ctx.base_url, version_id
        ))
        .header("nail-token", &session_a)
        .json(&serde_json::json!({ "content": comment_content }))
        .send()
        .await
        .expect("post comment");
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let comment_id = ctx
        .first_comment_id(&session_a, &version_id, &comment_content)
        .await;

    let session_b = ctx.login_via_api(&unique_email("other")).await;
    ctx.set_session_token(&session_b).await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/update",
            ctx.base_url
        ))
        .await
        .expect("goto update");
    ctx.wait_for_text("you are denied", 10).await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/delete",
            ctx.base_url
        ))
        .await
        .expect("goto delete");
    ctx.wait_for_text("you are denied", 10).await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/create",
            ctx.base_url
        ))
        .await
        .expect("goto version create");
    ctx.wait_for_text("you are denied", 10).await;

    ctx.page
        .goto(format!(
            "{}/public/article/{article_id}/version/{version_id}/comment/{comment_id}/delete",
            ctx.base_url
        ))
        .await
        .expect("goto comment delete");
    ctx.wait_for_text("you are denied", 10).await;
}
