
use crate::unit_tests::context::TestCtx;
use serde_json::json;
use uuid::Uuid;

async fn seed_author_scene(ctx: &TestCtx) -> (String, String, String, String) {
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, version_id) = ctx
        .create_article(
            &alice_session,
            "author title",
            "author summary",
            "#author",
            "1.0.0",
            "v1 note",
        )
        .await;
    (alice_session, bob_session, article_id, version_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_author_requires_session() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            "/author/check",
            json!({"article_id": Uuid::now_v7().to_string()}),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_author_requires_exactly_one_target() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx.post("/author/check", json!({}), Some(&session)).await;
    ctx.bad(status);
    let (status, _) = ctx
        .post(
            "/author/check",
            json!({
                "article_id": Uuid::now_v7().to_string(),
                "version_id": Uuid::now_v7().to_string(),
            }),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .post(
            "/author/check",
            json!({
                "article_id": Uuid::now_v7().to_string(),
                "comment_id": Uuid::now_v7().to_string(),
            }),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_author_article_target_flags_author_only() {
    let ctx = TestCtx::new().await;
    let (alice, bob, article_id, _version_id) = seed_author_scene(&ctx).await;

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"article_id": &article_id}),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["is_author"], true,
        "作者查询自己的文章必须 is_author:true"
    );

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"article_id": &article_id}),
            Some(&bob),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["is_author"], false,
        "路人查询他人文章必须 is_author:false"
    );

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"article_id": Uuid::now_v7().to_string()}),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["is_author"], false, "不存在的文章返回 false 而非错误");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_author_version_target_resolves_through_article_author() {
    let ctx = TestCtx::new().await;
    let (alice, bob, _article_id, version_id) = seed_author_scene(&ctx).await;

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"version_id": &version_id}),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["is_author"], true, "作者自己的版本必须 is_author:true");

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"version_id": &version_id}),
            Some(&bob),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["is_author"], false,
        "路人的版本视角必须 is_author:false"
    );

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({"version_id": Uuid::now_v7().to_string()}),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["is_author"], false, "不存在的版本返回 false 而非错误");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_author_comment_target_flags_author_only() {
    let ctx = TestCtx::new().await;
    let (alice, bob, _article_id, version_id) = seed_author_scene(&ctx).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                json!({ "content": "mine" }),
                Some(&alice),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().unwrap().to_string()
    };

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({ "comment_id": &comment_id }),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["is_author"], true,
        "评论作者查自己的评论必须 is_author:true"
    );

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({ "comment_id": &comment_id }),
            Some(&bob),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["is_author"], false,
        "路人查他人评论必须 is_author:false"
    );

    let (status, body) = ctx
        .post(
            "/author/check",
            json!({ "comment_id": Uuid::now_v7().to_string() }),
            Some(&alice),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["is_author"], false, "不存在的评论返回 false 而非错误");
}
