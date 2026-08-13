
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
async fn check_email_requires_session_even_with_invalid_pow() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let (status, _) = ctx
        .post("/email/check", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_email_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let (status, _) = ctx
        .post(
            "/email/check",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_email_matches_case_variant_of_bound_email() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("Alice@QQ.com");
    let (status, body) = ctx
        .post(
            "/email/check",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["matches"], true, "大小写变体必须视为同一邮箱");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_email_reports_mismatch_for_other_email() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("bob@qq.com");
    let (status, body) = ctx
        .post(
            "/email/check",
            json!({"pow": pow_json(&pow)}),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["matches"], false, "非绑定邮箱必须 matches:false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_update_requires_session() {
    let ctx = TestCtx::new().await;
    let old = ctx.client_proof_of_work("alice@qq.com");
    let new = ctx.client_proof_of_work("bob@qq.com");
    let (status, _) = ctx
        .post(
            "/email/update/send",
            json!({"old_email_pow": pow_json(&old), "new_email_pow": pow_json(&new)}),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_update_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.client_proof_of_work("alice@qq.com");
    let new = ctx.client_proof_of_work("bob@qq.com");
    let (status, _) = ctx
        .post(
            "/email/update/send",
            json!({"old_email_pow": pow_json(&old), "new_email_pow": pow_json(&new)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_update_rejects_new_email_with_disallowed_domain() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("alice@qq.com");
    let new = ctx.issued_proof_of_work("x@evil.com");
    let (status, _) = ctx
        .post(
            "/email/update/send",
            json!({"old_email_pow": pow_json(&old), "new_email_pow": pow_json(&new)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_update_rejects_old_email_not_matching_account() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("bob@qq.com");
    let new = ctx.issued_proof_of_work("bob2@qq.com");
    let (status, body) = ctx
        .post(
            "/email/update/send",
            json!({"old_email_pow": pow_json(&old), "new_email_pow": pow_json(&new)}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("match") || ctx.reason(&body).contains("bound"),
        "旧邮箱不一致的 reason 应含语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_update_with_closed_smtp_returns_500() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("alice@qq.com");
    let new = ctx.issued_proof_of_work("bob@qq.com");
    let (status, _) = ctx
        .post(
            "/email/update/send",
            json!({"old_email_pow": pow_json(&old), "new_email_pow": pow_json(&new)}),
            Some(&session),
        )
        .await;
    ctx.expect(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "全部合法 + SMTP 关闭必须 500",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_update_requires_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("x");
    let (status, _) = ctx
        .post(
            "/email/update/confirm",
            json!({
                "pow": pow_json(&pow),
                "old_email_token": Uuid::now_v7().to_string(),
                "new_email_token": Uuid::now_v7().to_string(),
            }),
            None,
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_update_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("x");
    let (status, _) = ctx
        .post(
            "/email/update/confirm",
            json!({
                "pow": pow_json(&pow),
                "old_email_token": Uuid::now_v7().to_string(),
                "new_email_token": Uuid::now_v7().to_string(),
            }),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_update_rejects_unseeded_token_pair() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();
    let payload = format!("{old_token}\n{new_token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let (status, body) = ctx
        .post(
            "/email/update/confirm",
            json!({"pow": pow_json(&pow), "old_email_token": old_token, "new_email_token": new_token}),
            Some(&session),
        )
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("invalid") || ctx.reason(&body).contains("expired"),
        "未播种 token 对的 reason 应含 invalid/expired 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_update_rejects_malformed_old_email_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("x");
    let (status, _) = ctx
        .post(
            "/email/update/confirm",
            json!({
                "pow": pow_json(&pow),
                "old_email_token": "not-a-uuid",
                "new_email_token": Uuid::now_v7().to_string(),
            }),
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn confirm_update_swaps_email_and_rotates_session() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;

    let old_hash = hash::email("alice@qq.com");
    let new_hash = hash::email("bob@qq.com");
    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();
    crate::repo::token::email_update::create_email_update_token(
        &ctx.state.cache,
        &user_id,
        &old_hash,
        &new_hash,
        &hash::token(&old_token),
        &hash::token(&new_token),
    );

    let payload = format!("{old_token}\n{new_token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let (status, body) = ctx
        .post(
            "/email/update/confirm",
            json!({"pow": pow_json(&pow), "old_email_token": old_token, "new_email_token": new_token}),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    let new_session = body["session_token"]
        .as_str()
        .expect("换绑成功必须返回新 session")
        .to_string();

    let (status, _) = ctx
        .post("/authenticate/verify", json!({}), Some(&session))
        .await;
    ctx.unauth(status);
    let (status, _) = ctx
        .post("/authenticate/verify", json!({}), Some(&new_session))
        .await;
    ctx.ok(status);

    let entry = crate::repo::user::read_user(&ctx.state.db, &user_id)
        .await
        .expect("查询用户")
        .expect("用户必须存在");
    assert_eq!(
        entry.email_address_hash, new_hash,
        "确认后账号邮箱必须换绑为新邮箱"
    );
}
