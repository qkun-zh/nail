
use crate::unit_tests::context::TestCtx;
use serde_json::json;
use uuid::Uuid;

#[allow(dead_code)]
async fn tag_names_of_article(ctx: &TestCtx, article_id: &str) -> Vec<String> {
    ctx.article_tag_names(article_id).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let body = json!({"title": "t", "summary": "s", "tags": "#x"});
    let (status, _) = ctx.post("/article/whatever", body.clone(), None).await;
    ctx.unauth(status);
    let (status, _) = ctx
        .post("/article/whatever", body, Some(&ctx.malformed_session()))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_rejects_non_author_and_missing_article() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    let article_id = ctx
        .create_article(&alice, "title", "summary", "#a", "1.0.0", "n")
        .await
        .0;
    let (status, _) = ctx
        .post(
            &format!("/article/{article_id}"),
            json!({"title": "t", "summary": "s", "tags": "#x"}),
            Some(&bob),
        )
        .await;
    ctx.forbidden(status);
    let (status, _) = ctx
        .post(
            &format!("/article/{}", Uuid::now_v7()),
            json!({"title": "t", "summary": "s", "tags": "#x"}),
            Some(&alice),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_rejects_blank_title() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let article_id = ctx
        .create_article(&session, "title", "summary", "#x", "1.0.0", "n")
        .await
        .0;
    let (status, _) = ctx
        .post(
            &format!("/article/{article_id}"),
            json!({"title": "   ", "summary": "s", "tags": "#x"}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_ok_replaces_fields_and_tags() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let article_id = ctx
        .create_article(&session, "old title", "old summary", "#old", "1.0.0", "n")
        .await
        .0;
    let (status, body) = ctx
        .post(
            &format!("/article/{article_id}"),
            json!({"title": "new title", "summary": "new summary", "tags": "#x#y"}),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["article_id"].as_str().unwrap(), article_id);
    let (status, body) = ctx
        .get(&format!("/article/{article_id}"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        body["title"].as_str().unwrap(),
        "new title",
        "回读 title 必须已更新"
    );
    assert_eq!(
        body["summary"].as_str().unwrap(),
        "new summary",
        "回读 summary 必须已更新"
    );
    let tag_names: Vec<&str> = body["tags"]
        .as_array()
        .expect("详情必须含 tags 数组")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(
        tag_names,
        vec!["#x", "#y"],
        "详情 tags 必须是全量替换后的新标签"
    );
    assert_eq!(
        tag_names_of_article(&ctx, &article_id).await,
        vec!["#x".to_string(), "#y".to_string()],
        "tags 必须是全量替换（#old 消失）"
    );
}
