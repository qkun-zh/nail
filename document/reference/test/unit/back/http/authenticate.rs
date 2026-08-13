
use crate::unit_tests::context::TestCtx;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use common::pow::Pow;
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;
use uuid::Uuid;

fn pow_json(pow: &Pow) -> Value {
    serde_json::to_value(pow).expect("pow 必须可序列化")
}

async fn submit_malformed_pow(ctx: &TestCtx, body: Value) -> StatusCode {
    let mut req = Request::builder()
        .method("POST")
        .uri("/authenticate/pow")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&body).expect("serialize body"),
        ))
        .expect("build request");
    req.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:3000".parse::<SocketAddr>().expect("static addr"),
    ));
    ctx.app
        .clone()
        .oneshot(req)
        .await
        .expect("oneshot")
        .status()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn issue_challenge_returns_uuidv7_id_and_configured_difficulty() {
    let ctx = TestCtx::new().await;
    let (status, body) = ctx.get("/authenticate/challenge", None).await;
    ctx.ok(status);
    let id = body["id"].as_str().expect("challenge id 必须是字符串");
    assert_eq!(
        Uuid::parse_str(id)
            .expect("challenge id 必须是合法 uuid")
            .get_version(),
        Some(uuid::Version::SortRand),
        "challenge id 必须是 uuidv7 形状"
    );
    assert_eq!(
        body["difficulty"].as_u64(),
        Some(ctx.difficulty()),
        "difficulty 必须等于服务端配置值"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("someone@qq.com");
    let (status, body) = ctx.post("/authenticate/pow", pow_json(&pow), None).await;
    ctx.bad(status);
    assert_eq!(body["ok"], false, "未记账 challenge 必须 ok:false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_rejects_tampered_solution() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("someone@qq.com");
    let (status, _) = ctx
        .post("/authenticate/pow", pow_json(&ctx.tampered(&pow)), None)
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_rejects_disallowed_domain() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("x@evil.com");
    let (status, _) = ctx.post("/authenticate/pow", pow_json(&pow), None).await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_with_closed_smtp_returns_500() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("someone@qq.com");
    let (status, body) = ctx.post("/authenticate/pow", pow_json(&pow), None).await;
    ctx.expect(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "合法邮箱 + SMTP 关闭必须 500",
    );
    assert_eq!(body["ok"], false, "500 响应 ok 必须为 false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_rejects_overlong_email() {
    let ctx = TestCtx::new().await;
    let long_email = format!("{}@qq.com", "a".repeat(260));
    let pow = ctx.issued_proof_of_work(&long_email);
    let (status, _) = ctx.post("/authenticate/pow", pow_json(&pow), None).await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn submit_proof_of_work_rejects_malformed_body() {
    let ctx = TestCtx::new().await;
    let status = submit_malformed_pow(
        &ctx,
        json!({
            "challenge": {"id": Uuid::now_v7().to_string(), "difficulty": ctx.difficulty()},
            "payload": "someone@qq.com",
        }),
    )
    .await;
    ctx.expect(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "畸形 Pow body 必须被 axum 反序列化拒绝（422）",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redeem_token_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work(&Uuid::now_v7().to_string());
    let (status, _) = ctx
        .post("/authenticate/token", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redeem_token_rejects_unknown_authentication_token() {
    let ctx = TestCtx::new().await;
    let unknown = Uuid::now_v7().to_string();
    let pow = ctx.issued_proof_of_work(&unknown);
    let (status, body) = ctx
        .post("/authenticate/token", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.bad(status);
    assert!(
        ctx.reason(&body).contains("token"),
        "未知认证 token 的 reason 应含 token 语义，实际: {}",
        ctx.reason(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redeem_token_rejects_malformed_token_payload() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("not-a-uuid");
    let (status, _) = ctx
        .post("/authenticate/token", json!({"pow": pow_json(&pow)}), None)
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_requires_nail_token_header() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.post("/authenticate/verify", json!({}), None).await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_rejects_malformed_session_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            "/authenticate/verify",
            json!({}),
            Some(&ctx.malformed_session()),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_rejects_wellformed_unregistered_session() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx
        .post(
            "/authenticate/verify",
            json!({}),
            Some(&ctx.ghost_session()),
        )
        .await;
    ctx.unauth(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_accepts_registered_session_and_does_not_consume_it() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx
        .post("/authenticate/verify", json!({}), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(body["ok"], true, "已注册 session 校验必须 ok:true");
    let (status, _) = ctx
        .post("/authenticate/verify", json!({}), Some(&session))
        .await;
    ctx.ok(status);
}
