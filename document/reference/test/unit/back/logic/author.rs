
use uuid::Uuid;

use crate::logic::author::handle_is_author;
use crate::logic::error::LogicError;
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn author_check_requires_session() {
    let ctx = TestCtx::new().await;
    let err = handle_is_author(&ctx.state, &ctx.ghost_session(), Some("a"), None, None)
        .await
        .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn author_check_requires_exactly_one_target() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_is_author(&ctx.state, &session, Some("a"), Some("v"), None)
        .await
        .expect_err("双目标必须 400");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = handle_is_author(&ctx.state, &session, Some("a"), None, Some("c"))
        .await
        .expect_err("双目标必须 400");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = handle_is_author(&ctx.state, &session, None, None, None)
        .await
        .expect_err("无目标必须 400");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn author_check_article_target() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let article_id = ctx
        .create_article(&alice_session, "t", "s", "#tag", "1.0.0", "n")
        .await
        .0;

    assert!(
        handle_is_author(&ctx.state, &alice_session, Some(&article_id), None, None)
            .await
            .expect("作者查询"),
        "作者本人 → true"
    );
    assert!(
        !handle_is_author(&ctx.state, &bob_session, Some(&article_id), None, None)
            .await
            .expect("非作者查询"),
        "非作者 → false"
    );
    assert!(
        !handle_is_author(
            &ctx.state,
            &alice_session,
            Some(&Uuid::now_v7().to_string()),
            None,
            None
        )
        .await
        .expect("不存在文章"),
        "不存在 → false"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn author_check_version_target_resolves_via_article() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&alice_session).await;

    assert!(
        handle_is_author(&ctx.state, &alice_session, None, Some(&version_id), None)
            .await
            .expect("作者查询"),
        "版本作者（归属文章作者）→ true"
    );
    assert!(
        !handle_is_author(&ctx.state, &bob_session, None, Some(&version_id), None)
            .await
            .expect("非作者查询"),
        "非作者 → false"
    );
    assert!(
        !handle_is_author(
            &ctx.state,
            &alice_session,
            None,
            Some(&Uuid::now_v7().to_string()),
            None
        )
        .await
        .expect("不存在版本"),
        "不存在版本 → false"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn author_check_comment_target_flags_comment_author() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&alice_session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({ "content": "mine" }),
                Some(&alice_session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().unwrap().to_string()
    };

    assert!(
        handle_is_author(&ctx.state, &alice_session, None, None, Some(&comment_id))
            .await
            .expect("作者查询"),
        "评论作者本人 → true"
    );
    assert!(
        !handle_is_author(&ctx.state, &bob_session, None, None, Some(&comment_id))
            .await
            .expect("非作者查询"),
        "非作者 → false"
    );
    assert!(
        !handle_is_author(
            &ctx.state,
            &alice_session,
            None,
            None,
            Some(&Uuid::now_v7().to_string())
        )
        .await
        .expect("不存在评论"),
        "不存在评论 → false"
    );
}
