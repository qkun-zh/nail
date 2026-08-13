
use std::time::Duration;

use common::hash;
use uuid::Uuid;

use crate::logic::authenticate::{
    authenticate_session, generate_challenge, handle_email_auth_request, handle_session_verify,
    handle_token_exchange, normalize_email, normalize_token, validate_email, verify_issued_pow,
};
use crate::logic::error::LogicError;
use crate::unit_tests::context::TestCtx;

fn short_time_to_live_cache(ctx: &mut TestCtx, ttl: Duration) {
    ctx.state.cache = crate::repo::TokenCaches::new(ttl, ttl, ttl, ttl, 100_000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn challenge_is_recorded_in_cache_on_issue() {
    let ctx = TestCtx::new().await;
    let challenge = generate_challenge(&ctx.state.config.server, &ctx.state.cache);
    assert_eq!(challenge.difficulty, ctx.difficulty());
    assert!(
        ctx.state
            .cache
            .challenge
            .get(&challenge.id.to_string())
            .is_some(),
        "签发即记账：challenge id 必须出现在缓存里"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_issued_proof_of_work_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let err = verify_issued_pow(&ctx.state, &pow).expect_err("未记账的 challenge 必须被拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "未签发 challenge → BadRequest，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_issued_proof_of_work_accepts_issued_valid_proof() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    verify_issued_pow(&ctx.state, &pow).expect("已记账 + 合法 proof 必须通过");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_issued_proof_of_work_rejects_tampered_proof_and_burns_challenge() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    let tampered = ctx.tampered(&pow);
    let err = verify_issued_pow(&ctx.state, &tampered).expect_err("篡改 solution 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let err2 = verify_issued_pow(&ctx.state, &pow).expect_err("challenge 已被消费");
    assert!(matches!(err2, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_issued_proof_of_work_burns_challenge_on_any_submission() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    verify_issued_pow(&ctx.state, &pow).expect("第一次通过");
    let err = verify_issued_pow(&ctx.state, &pow).expect_err("重放同一 challenge 必须失败");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_issued_proof_of_work_rejects_expired_challenge() {
    let mut ctx = TestCtx::new().await;
    short_time_to_live_cache(&mut ctx, Duration::from_millis(80));
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let err = verify_issued_pow(&ctx.state, &pow).expect_err("过期 challenge 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_authentication_request_rejects_disallowed_domain() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("alice@evil.com");
    let err = handle_email_auth_request(&ctx.state, pow)
        .await
        .expect_err("白名单外域名必须被拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "非法域名 → BadRequest，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_authentication_request_smtp_failure_leaves_no_token() {
    let ctx = TestCtx::new().await;
    let email = "alice@qq.com";
    let pow = ctx.issued_proof_of_work(email);
    let err = handle_email_auth_request(&ctx.state, pow)
        .await
        .expect_err("SMTP 关闭（127.0.0.1:1）时发信必失败");
    assert!(
        matches!(err, LogicError::Internal(_)),
        "SMTP 基础设施失败 → Internal(500)，实际: {err:?}"
    );
    assert!(
        ctx.state
            .cache
            .authenticate_by_email_hash
            .get(&hash::email(email))
            .is_none(),
        "SMTP 失败后不得残留认证 token"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_authentication_request_rate_limited_within_window_returns_400() {
    let ctx = TestCtx::new().await;
    let email = "alice@qq.com";
    let err1 = handle_email_auth_request(&ctx.state, ctx.issued_proof_of_work(email))
        .await
        .expect_err("SMTP 关闭时首次发信必失败");
    assert!(matches!(err1, LogicError::Internal(_)));

    let err2 = handle_email_auth_request(&ctx.state, ctx.issued_proof_of_work(email))
        .await
        .expect_err("窗口内重复请求必须被限速拒绝");
    assert!(
        matches!(err2, LogicError::BadRequest(_)),
        "限速 → BadRequest(400)，实际: {err2:?}"
    );
    assert!(
        err2.to_string().contains("recently"),
        "限速 reason 应含 'recently'，实际: {err2}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_exchange_rejects_unknown_token() {
    let ctx = TestCtx::new().await;
    let token = Uuid::now_v7().to_string();
    let pow = ctx.issued_proof_of_work(&token);
    let err = handle_token_exchange(&ctx.state, &pow)
        .await
        .expect_err("无对应未消费认证 token → 拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "无效/过期能力 token → BadRequest(400)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_exchange_rejects_malformed_token_payload() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("not-a-uuid");
    let err = handle_token_exchange(&ctx.state, &pow)
        .await
        .expect_err("payload 不是 UUID → 拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_exchange_happy_path_consumes_token_and_mints_session() {
    let ctx = TestCtx::new().await;
    let email = "alice@qq.com";
    let email_hash = hash::email(email);
    let token = Uuid::now_v7().to_string();
    crate::repo::token::authenticate::create_authenticate_token(
        &ctx.state.cache,
        &token,
        &email_hash,
        "subject",
    );

    let pow = ctx.issued_proof_of_work(&token);
    let session = handle_token_exchange(&ctx.state, &pow)
        .await
        .expect("token 兑换 session 必须成功");

    let user_id = crate::repo::user::find_user_by_email_address_hash(&ctx.state.db, &email_hash)
        .await
        .expect("db query")
        .expect("首次兑换必须创建账号");
    assert_eq!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &session)
            .as_deref(),
        Some(user_id.as_str()),
        "session 必须绑定新账号"
    );

    let pow2 = ctx.issued_proof_of_work(&token);
    let err = handle_token_exchange(&ctx.state, &pow2)
        .await
        .expect_err("token 已消费，重放必须失败");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_exchange_with_whitespace_wrapped_token_still_works() {
    let ctx = TestCtx::new().await;
    let email = "bob@qq.com";
    let token = Uuid::now_v7().to_string();
    crate::repo::token::authenticate::create_authenticate_token(
        &ctx.state.cache,
        &token,
        &hash::email(email),
        "subject",
    );
    let wrapped = format!("\n  {token}\t");
    let pow = ctx.issued_proof_of_work(&wrapped);
    handle_token_exchange(&ctx.state, &pow)
        .await
        .expect("token 规范化必须去除全部空白");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_session_rejects_empty_and_malformed_tokens() {
    let ctx = TestCtx::new().await;
    let err = authenticate_session(&ctx.state, "").expect_err("空 token → 拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "空串不是 UUID → 400"
    );
    let err = authenticate_session(&ctx.state, "not-a-uuid").expect_err("畸形 token → 拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)), "畸形 token → 400");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_session_rejects_unknown_token() {
    let ctx = TestCtx::new().await;
    let err = authenticate_session(&ctx.state, &ctx.ghost_session())
        .expect_err("格式合法但未注册 → 拒绝");
    assert!(
        matches!(err, LogicError::Unauthorized(_)),
        "无效/过期 session → Unauthorized(401)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_session_rejects_expired_session() {
    let mut ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    assert_eq!(
        authenticate_session(&ctx.state, &session).expect("有效 session 必须通过"),
        user_id
    );
    short_time_to_live_cache(&mut ctx, Duration::from_millis(80));
    let session2 = ctx.session_for(&user_id);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let err = authenticate_session(&ctx.state, &session2).expect_err("过期 session → 拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_session_returns_user_id_for_valid_session() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    assert_eq!(
        authenticate_session(&ctx.state, &session).expect("有效 session"),
        user_id
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_verify_mirrors_session_semantics() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    assert!(handle_session_verify(&ctx.state, &session).expect("有效 session"));
    assert!(!handle_session_verify(&ctx.state, &ctx.ghost_session()).expect("未注册 session"));
    let err = handle_session_verify(&ctx.state, "not-a-uuid").expect_err("畸形 token");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn normalize_email_trims_and_lowercases() {
    assert_eq!(normalize_email("  Alice@QQ.com "), "alice@qq.com");
    assert_eq!(normalize_email("BOB@163.COM"), "bob@163.com");
    assert_eq!(normalize_email("\t c@qq.com\n"), "c@qq.com");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn normalize_token_strips_whitespace_and_validates_uuid() {
    let uuid = Uuid::now_v7().to_string();
    assert_eq!(
        normalize_token(&format!("  {uuid}\n\t")),
        Some(uuid),
        "复制粘贴带空白/换行的 token 必须被清洗"
    );
    assert_eq!(normalize_token("not-a-uuid"), None);
    assert_eq!(normalize_token(""), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn validate_email_boundaries() {
    let domains = &["qq.com".to_string(), "163.com".to_string()];
    assert!(validate_email("alice@qq.com", domains));
    assert!(validate_email("alice@QQ.com", domains), "域名大小写不敏感");
    assert!(!validate_email("alice@evil.com", domains));
    assert!(!validate_email("no-at-sign", domains));
    let long_local = "a".repeat(300);
    assert!(!validate_email(&format!("{long_local}@qq.com"), domains));
    let local_ok = "a".repeat(63);
    assert!(validate_email(&format!("{local_ok}@qq.com"), domains));
    let local_long = "a".repeat(65);
    assert!(!validate_email(&format!("{local_long}@qq.com"), domains));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_exchange_keeps_existing_sessions() {
    let ctx = TestCtx::new().await;
    let email = "carol@qq.com";
    let email_hash = hash::email(email);
    let (user_id, _first_session) = ctx.register(email).await;
    let old_session = ctx.session_for(&user_id);

    let token = Uuid::now_v7().to_string();
    crate::repo::token::authenticate::create_authenticate_token(
        &ctx.state.cache,
        &token,
        &email_hash,
        &Uuid::now_v7().to_string(),
    );
    let pow = ctx.issued_proof_of_work(&token);
    let new_session = handle_token_exchange(&ctx.state, &pow)
        .await
        .expect("exchange must succeed");

    assert_eq!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &new_session),
        Some(user_id.clone())
    );
    assert_eq!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &old_session),
        Some(user_id)
    );
}
