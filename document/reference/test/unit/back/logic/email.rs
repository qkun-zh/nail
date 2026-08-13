
use common::hash;
use uuid::Uuid;

use crate::logic::email::{
    handle_email_check, handle_email_update_confirm, handle_email_update_send,
};
use crate::logic::error::LogicError;
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_requires_session_even_with_invalid_pow() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let err = handle_email_check(&ctx.state, &pow, &ctx.ghost_session())
        .await
        .expect_err("无 session 必须拒绝");
    assert!(
        matches!(err, LogicError::Unauthorized(_)),
        "session 门禁先于 PoW：无 session → 401，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("alice@qq.com");
    let err = handle_email_check(&ctx.state, &pow, &session)
        .await
        .expect_err("未记账 challenge 必须被拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "未签发 challenge → BadRequest，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_matches_case_variant_of_bound_email() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("Alice@QQ.com");
    let ok = handle_email_check(&ctx.state, &pow, &session)
        .await
        .expect("大小写变体必须视为同一邮箱");
    assert!(ok, "matches 必须为 true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn check_reports_mismatch_for_other_email() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("bob@qq.com");
    let ok = handle_email_check(&ctx.state, &pow, &session)
        .await
        .expect("非绑定邮箱返回 false，不报错");
    assert!(!ok, "matches 必须为 false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_send_requires_session() {
    let ctx = TestCtx::new().await;
    let old = ctx.client_proof_of_work("alice@qq.com");
    let new = ctx.client_proof_of_work("bob@qq.com");
    let err = handle_email_update_send(&ctx.state, &old, &new, &ctx.ghost_session())
        .await
        .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_send_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.client_proof_of_work("alice@qq.com");
    let new = ctx.client_proof_of_work("bob@qq.com");
    let err = handle_email_update_send(&ctx.state, &old, &new, &session)
        .await
        .expect_err("未记账 challenge 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_send_rejects_new_email_with_disallowed_domain() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("alice@qq.com");
    let new = ctx.issued_proof_of_work("x@evil.com");
    let err = handle_email_update_send(&ctx.state, &old, &new, &session)
        .await
        .expect_err("非法域名必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_send_rejects_old_email_not_matching_account() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("bob@qq.com");
    let new = ctx.issued_proof_of_work("bob2@qq.com");
    let err = handle_email_update_send(&ctx.state, &old, &new, &session)
        .await
        .expect_err("旧邮箱不一致必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_send_with_closed_smtp_returns_internal_and_rolls_back() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let old = ctx.issued_proof_of_work("alice@qq.com");
    let new = ctx.issued_proof_of_work("bob@qq.com");
    let err = handle_email_update_send(&ctx.state, &old, &new, &session)
        .await
        .expect_err("SMTP 关闭（127.0.0.1:1）时发信必失败");
    assert!(
        matches!(err, LogicError::Internal(_)),
        "SMTP 基础设施失败 → Internal(500)，实际: {err:?}"
    );
    let entry =
        crate::repo::token::email_update::read_email_update_token(&ctx.state.cache, &user_id);
    assert!(entry.is_none(), "SMTP 失败后不得残留 email_update 行");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_requires_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.client_proof_of_work("x");
    let err = handle_email_update_confirm(
        &ctx.state,
        &pow,
        &Uuid::now_v7().to_string(),
        &Uuid::now_v7().to_string(),
        &ctx.ghost_session(),
    )
    .await
    .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("x");
    let err = handle_email_update_confirm(
        &ctx.state,
        &pow,
        &Uuid::now_v7().to_string(),
        &Uuid::now_v7().to_string(),
        &session,
    )
    .await
    .expect_err("未记账 challenge 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_rejects_unseeded_token_pair() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();
    let payload = format!("{old_token}\n{new_token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let err = handle_email_update_confirm(&ctx.state, &pow, &old_token, &new_token, &session)
        .await
        .expect_err("未播种 token 对必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_rejects_malformed_old_email_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("x");
    let err = handle_email_update_confirm(
        &ctx.state,
        &pow,
        "not-a-uuid",
        &Uuid::now_v7().to_string(),
        &session,
    )
    .await
    .expect_err("畸形 token 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_swaps_email_and_rotates_session() {
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
    let new_session =
        handle_email_update_confirm(&ctx.state, &pow, &old_token, &new_token, &session)
            .await
            .expect("换绑成功必须返回新 session");

    let (status, _) = ctx
        .post(
            "/authenticate/verify",
            serde_json::json!({}),
            Some(&session),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
    let (status, _) = ctx
        .post(
            "/authenticate/verify",
            serde_json::json!({}),
            Some(&new_session),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let entry = crate::repo::user::read_user(&ctx.state.db, &user_id)
        .await
        .expect("查询用户")
        .expect("用户必须存在");
    assert_eq!(
        entry.email_address_hash, new_hash,
        "确认后账号邮箱必须换绑为新邮箱"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_compare_and_swap_rejects_stale_row_overwritten_by_newer_send() {
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

    let new_token2 = Uuid::now_v7().to_string();
    crate::repo::token::email_update::create_email_update_token(
        &ctx.state.cache,
        &user_id,
        &old_hash,
        &new_hash,
        &hash::token(&old_token),
        &hash::token(&new_token2),
    );

    let payload = format!("{old_token}\n{new_token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let err = handle_email_update_confirm(&ctx.state, &pow, &old_token, &new_token, &session)
        .await
        .expect_err("被覆盖的旧 token 对必须失效");
    assert!(matches!(err, LogicError::BadRequest(_)));
    assert!(
        err.to_string().contains("mismatch"),
        "reason 应含 mismatch，实际: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_rejects_same_token_pair() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let token = Uuid::now_v7().to_string();
    crate::repo::token::email_update::create_email_update_token(
        &ctx.state.cache,
        &user_id,
        &hash::email("alice@qq.com"),
        &hash::email("bob@qq.com"),
        &hash::token(&token),
        &hash::token(&token),
    );
    let payload = format!("{token}\n{token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let err = handle_email_update_confirm(&ctx.state, &pow, &token, &token, &session)
        .await
        .expect_err("old token 与 new token 相同必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_compare_and_swap_fails_when_email_already_changed() {
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

    crate::repo::user::update_user_email(&ctx.state.db, &user_id, &old_hash, &new_hash)
        .await
        .expect("update");

    let payload = format!("{old_token}\n{new_token}");
    let pow = ctx.issued_proof_of_work(&payload);
    let err = handle_email_update_confirm(&ctx.state, &pow, &old_token, &new_token, &session)
        .await
        .expect_err("email 已被改走 → CAS 失败");
    assert!(matches!(err, LogicError::BadRequest(_)));
    assert!(
        err.to_string().contains("already been changed"),
        "reason 应含 'already been changed'，实际: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_confirm_consumes_token_row_and_clears_capabilities() {
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
    handle_email_update_confirm(&ctx.state, &pow, &old_token, &new_token, &session)
        .await
        .expect("confirm 成功");

    assert!(
        crate::repo::token::email_update::read_email_update_token(&ctx.state.cache, &user_id)
            .is_none(),
        "confirm 后 email_update 行必须被消费"
    );
    assert!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &session)
            .is_none()
    );
    assert!(
        ctx.state
            .cache
            .authenticate_by_email_hash
            .get(&old_hash)
            .is_none()
    );
}
