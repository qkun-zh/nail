
use std::time::Duration;

use common::hash;
use uuid::Uuid;

use crate::logic::error::LogicError;
use crate::logic::user::{
    handle_deregister_confirm, handle_deregister_request, handle_logout, handle_read_name,
    handle_update_name,
};
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn logout_requires_valid_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work("random-nonce");
    let err = handle_logout(&ctx.state, &pow, &ctx.ghost_session())
        .await
        .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn logout_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work("random-nonce");
    let err = handle_logout(&ctx.state, &pow, &session)
        .await
        .expect_err("未记账 challenge 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn logout_deletes_only_calling_session() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let other_session = ctx.session_for(&user_id);
    let pow = ctx.issued_proof_of_work("random-nonce");
    handle_logout(&ctx.state, &pow, &session)
        .await
        .expect("登出必须成功");

    assert!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &session)
            .is_none(),
        "被登出的 session 必须删除"
    );
    assert!(
        crate::repo::token::session::find_user_id_by_session_token(
            &ctx.state.cache,
            &other_session
        )
        .is_some(),
        "其他 session 不受影响"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_request_rejects_email_mismatch() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("bob@qq.com");
    let err = handle_deregister_request(&ctx.state, &pow, &session)
        .await
        .expect_err("邮箱不匹配必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_request_smtp_failure_rolls_back_token() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("alice@qq.com");
    let err = handle_deregister_request(&ctx.state, &pow, &session)
        .await
        .expect_err("SMTP 关闭（127.0.0.1:1）时发信必失败");
    assert!(
        matches!(err, LogicError::Internal(_)),
        "SMTP 基础设施失败 → Internal(500)，实际: {err:?}"
    );
    assert!(
        ctx.state.cache.deregister_by_user.get(&user_id).is_none(),
        "SMTP 失败后不得残留 deregister token"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_requires_session() {
    let ctx = TestCtx::new().await;
    let pow = ctx.issued_proof_of_work(&Uuid::now_v7().to_string());
    let err = handle_deregister_confirm(&ctx.state, &pow, &ctx.ghost_session())
        .await
        .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_rejects_unissued_challenge() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.client_proof_of_work(&Uuid::now_v7().to_string());
    let err = handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect_err("未记账 challenge 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_rejects_unknown_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let token = Uuid::now_v7().to_string();
    let pow = ctx.issued_proof_of_work(&token);
    let err = handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect_err("无对应 deregister token → 拒绝");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "无效/过期能力 token → BadRequest(400)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_rejects_token_of_other_user() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (other_id, _other_session) = ctx.register("bob@qq.com").await;
    let token = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &token,
        &other_id,
        &hash::email("bob@qq.com"),
    );
    let pow = ctx.issued_proof_of_work(&token);
    let err = handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect_err("token 不属于当前 session 用户必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_transfers_assets_to_recycler() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let comment_id = {
        let (status, body) = ctx
            .post(
                &format!("/version/{version_id}/comments"),
                serde_json::json!({"content": "a comment"}),
                Some(&session),
            )
            .await;
        ctx.created(status);
        body["comment_id"].as_str().expect("comment_id").to_string()
    };

    let token = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &token,
        &user_id,
        &hash::email("alice@qq.com"),
    );

    let pow = ctx.issued_proof_of_work(&token);
    handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect("注销确认必须成功");

    assert!(
        crate::repo::user::read_user(&ctx.state.db, &user_id)
            .await
            .expect("查询")
            .is_none(),
        "账号必须删除"
    );
    let article = crate::repo::article::read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询")
        .expect("文章必须保留");
    assert_eq!(
        article.get("title").and_then(|v| v.as_str()),
        Some("seed title"),
        "文章内容必须保留"
    );
    let user_zero_id = crate::repo::user::find_user_by_email_address_hash(
        &ctx.state.db,
        &common::hash::email(&ctx.state.config.server.user_zero_email),
    )
    .await
    .expect("查询")
    .expect("user zero 必须存在");
    let owner = ctx
        .incoming_edge_from_id(
            crate::repo::types::ENTITY_TYPE_ARTICLE,
            crate::repo::types::EDGE_USER_TO_ARTICLE,
            &article_id,
        )
        .await;
    assert_eq!(
        owner.as_deref(),
        Some(user_zero_id.as_str()),
        "文章所有权边必须转移到回收者"
    );
    let _version = crate::repo::article::read_version(&ctx.state.db, &version_id)
        .await
        .expect("查询")
        .expect("版本必须保留");
    let comment = crate::repo::comment::read_comments_by_version(
        &ctx.state.db,
        &version_id,
        ctx.state.config.server.max_comment_tree_depth as usize,
    )
    .await
    .expect("查询")
    .into_iter()
    .find(|row| row.get("comment_id").and_then(|v| v.as_str()) == Some(comment_id.as_str()))
    .expect("评论必须保留");
    let _ = comment;
    assert!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &session)
            .is_none(),
        "session 必须删除"
    );
    assert!(
        crate::repo::token::deregister::find_user_id_by_deregister_token(&ctx.state.cache, &token)
            .is_none(),
        "deregister token 必须消费"
    );
    let pdf_dir = std::path::Path::new(&ctx.state.config.server.pdf_storage_path);
    let version_files = find_pdf_files(pdf_dir);
    assert!(!version_files.is_empty(), "PDF 文件必须保留");
    let leftover =
        crate::repo::tag::find_tag_ids_by_names_contains(&ctx.state.db, &["seed".to_string()])
            .await
            .expect("查询 tags");
    assert!(!leftover.is_empty(), "tag 必须保留，文章仍在引用");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_token_consumed_after_db_commit_allows_retry_semantics() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;

    let token = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &token,
        &user_id,
        &hash::email("alice@qq.com"),
    );

    let pow = ctx.issued_proof_of_work(&token);
    handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect("第一次 confirm 成功");

    let pow2 = ctx.issued_proof_of_work(&token);
    let err = handle_deregister_confirm(&ctx.state, &pow2, &session)
        .await
        .expect_err("重复 confirm 必须失败");
    assert!(
        matches!(err, LogicError::Unauthorized(_)),
        "session 已删 → 401，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_name_returns_default_uuid_name() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let name = handle_read_name(&ctx.state, &session)
        .await
        .expect("获取名字");
    assert_eq!(
        name,
        user_id.replace('-', ""),
        "默认名必须是 uuidv7 无连字符"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_validates_and_persists() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("Alice-01");
    let name = handle_update_name(&ctx.state, &pow, &session)
        .await
        .expect("设置名字");
    assert_eq!(name, "Alice-01");
    let entry = crate::repo::user::read_user(&ctx.state.db, &_user_id)
        .await
        .expect("查询")
        .expect("用户存在");
    assert_eq!(entry.name, "Alice-01");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_rejects_forbidden_chars() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let pow = ctx.issued_proof_of_work("Alice 01");
    let err = handle_update_name(&ctx.state, &pow, &session)
        .await
        .expect_err("非法字符必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_name_rejects_taken_name() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let pow_alice = ctx.issued_proof_of_work("Alice-01");
    handle_update_name(&ctx.state, &pow_alice, &alice_session)
        .await
        .expect("alice 占名");
    let pow_bob = ctx.issued_proof_of_work("Alice-01");
    let err = handle_update_name(&ctx.state, &pow_bob, &bob_session)
        .await
        .expect_err("重名必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expired_deregister_token_is_rejected() {
    let mut ctx = TestCtx::new().await;
    ctx.state.cache = crate::repo::TokenCaches::new(
        Duration::from_millis(80),
        Duration::from_secs(3600),
        Duration::from_secs(3600),
        Duration::from_secs(3600),
        100_000,
    );
    let (user_id, session) = ctx.register("alice@qq.com").await;

    let token = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &token,
        &user_id,
        &hash::email("alice@qq.com"),
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pow = ctx.issued_proof_of_work(&token);
    let err = handle_deregister_confirm(&ctx.state, &pow, &session)
        .await
        .expect_err("过期 deregister token 必须被拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_expired_token_keeps_live_session() {
    let mut ctx = TestCtx::new().await;
    ctx.state.cache = crate::repo::TokenCaches::new(
        Duration::from_millis(30),
        Duration::from_secs(600),
        Duration::from_secs(600),
        Duration::from_secs(600),
        100,
    );
    let (user_id, session) = ctx.register("alice@qq.com").await;

    let dtoken = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &ctx.state.cache,
        &dtoken,
        &user_id,
        &hash::email("alice@qq.com"),
    );

    tokio::time::sleep(Duration::from_millis(60)).await;

    let err = handle_deregister_confirm(&ctx.state, &ctx.issued_proof_of_work(&dtoken), &session)
        .await
        .expect_err("过期 deregister token 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    assert_eq!(
        crate::repo::token::session::find_user_id_by_session_token(&ctx.state.cache, &session),
        Some(user_id),
        "capability token 过期不烧 session"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_name_fails_when_user_vanishes() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    crate::unit_tests::context::delete_user(&ctx.state.db, &user_id)
        .await
        .expect("delete user");
    let err = handle_read_name(&ctx.state, &session)
        .await
        .expect_err("账号已注销但 session 仍活 → 必须拒绝");
    assert!(
        matches!(err, LogicError::Unauthorized(_)),
        "user vanishes → Unauthorized(401)，实际: {err:?}"
    );
}

fn find_pdf_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    collect_pdf(dir, &mut out);
    out
}

fn collect_pdf(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pdf(&path, out);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            out.push(path);
        }
    }
}
