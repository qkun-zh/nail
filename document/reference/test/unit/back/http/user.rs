
use crate::unit_tests::context::TestCtx;
use axum::http::StatusCode;
use common::hash;
use common::pow::Pow;
use serde_json::{Value, json};
use uuid::Uuid;

fn pow_json(pow: &Pow) -> Value {
    serde_json::to_value(pow).expect("pow 必须可序列化")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_name_requires_session_and_rejects_malformed_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.get("/user/name", None).await;
    ctx.unauth(status);
    let (status, _) = ctx.get("/user/name", Some(&ctx.malformed_session())).await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_name_returns_default_name_derived_from_user_id() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx.get("/user/name", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(
        body["name"].as_str().unwrap(),
        user_id.replace('-', ""),
        "默认名必须是 user_id 去横线"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_requires_session_and_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("alice");
    let (status, _) = ctx
        .post("/user/name", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.unauth(status);

    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("alice");
    let (status, _) = ctx
        .post("/user/name", json!({"pow": pow_json(&pow)}), Some(&session))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_rejects_invalid_char_and_length() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    for bad_name in ["", "a b", "a@b", "a!b", &"a".repeat(33)] {
        let pow = ctx.issued_proof_of_work(bad_name);
        let (status, _) = ctx
            .post(
                "/user/name",
                json!({"name": bad_name, "pow": pow_json(&pow)}),
                Some(&session),
            )
            .await;
        ctx.bad(status);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_updates_and_is_visible_on_read() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("newname");
    let (status, _) = ctx
        .post(
            "/user/name",
            json!({"name": "newname", "pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);

    let (status, body) = ctx.get("/user/name", Some(&session)).await;
    ctx.ok(status);
    assert_eq!(body["name"].as_str().unwrap(), "newname");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_rejects_duplicate() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("taken");
    let (status, _) = ctx
        .post(
            "/user/name",
            json!({"name": "taken", "pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);

    let (_user_id2, session2) = ctx.register("bob@qq.com").await;
    let pow = ctx.issued_proof_of_work("taken");
    let (status, body) = ctx
        .post(
            "/user/name",
            json!({"name": "taken", "pow": pow_json(&pow)}),
            Some(&session2),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("taken"),
        "重名 reason 应含被占用名字，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn logout_kills_session_and_rejects_replay() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("random");
    let (status, _) = ctx
        .post(
            "/user/logout",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);

    let (status, _) = ctx
        .post("/authenticate/verify", json!({}), Some(&session))
        .await;
    ctx.unauth(status);

    let pow = ctx.issued_proof_of_work("random");
    let (status, _) = ctx
        .post(
            "/user/logout",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.unauth(status);

    let session2 = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&ctx.state.cache, &session2, &user_id);
    let (status, body) = ctx.get("/user/name", Some(&session2)).await;
    ctx.ok(status);
    assert!(!body["name"].as_str().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_requires_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    let (status, _) = ctx
        .post("/user/deregister", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let (status, _) = ctx
        .post(
            "/user/deregister",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_rejects_email_not_matching_account() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("bob@qq.com");
    let (status, body) = ctx
        .post(
            "/user/deregister",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("match"),
        "邮箱不一致 reason 应含 match 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_with_closed_smtp_returns_500() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    let (status, _) = ctx
        .post(
            "/user/deregister",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.expect(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "账号邮箱 + SMTP 关闭必须 500",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_deregister_requires_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work(&Uuid::now_v7().to_string());
    let (status, _) = ctx
        .post(
            "/user/deregister/confirm",
            json!({"pow": pow_json(&pow)}),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_deregister_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work(&Uuid::now_v7().to_string());
    let (status, _) = ctx
        .post(
            "/user/deregister/confirm",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_deregister_rejects_unseeded_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work(&Uuid::now_v7().to_string());
    let (status, body) = ctx
        .post(
            "/user/deregister/confirm",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("invalid") || ctx.reason(&body).contains("expired"),
        "未播种确认 token 的 reason 应含 invalid/expired 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_deregister_transfers_assets_and_clears_session() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let email_hash = hash::email("alice@qq.com");

    let token = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &token,
        &user_id,
        &email_hash,
    );
    let pow = ctx.issued_proof_of_work(&token);
    let (status, _) = ctx
        .post(
            "/user/deregister/confirm",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);

    let (status, _) = ctx
        .post("/authenticate/verify", json!({}), Some(&session))
        .await;
    ctx.unauth(status);

    let found = crate::repo::user::find_user_by_email_address_hash(&ctx.state.db, &email_hash)
        .await
        .expect("查询用户");
    assert!(found.is_none(), "注销后用户记录必须删除");
}
