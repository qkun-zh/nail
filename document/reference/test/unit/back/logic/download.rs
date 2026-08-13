
use uuid::Uuid;

use crate::logic::download::{handle_consume_download, handle_mint_download_url};
use crate::logic::error::LogicError;
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_requires_session() {
    let ctx = TestCtx::new().await;
    let err = handle_mint_download_url(&ctx.state, &ctx.ghost_session(), "a", "v")
        .await
        .expect_err("无 session 必须拒绝");
    assert!(
        matches!(err, LogicError::Unauthorized(_)),
        "mint 需要 session，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_rejects_missing_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_mint_download_url(
        &ctx.state,
        &session,
        "ghost-article",
        &Uuid::now_v7().to_string(),
    )
    .await
    .expect_err("版本不存在必须 404");
    assert!(
        matches!(err, LogicError::NotFound(_)),
        "版本缺失 → NotFound(404)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_requires_version_to_belong_to_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let err = handle_mint_download_url(
        &ctx.state,
        &session,
        &Uuid::now_v7().to_string(),
        &version_id,
    )
    .await
    .expect_err("版本不属于该文章必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_returns_url_with_token_and_creates_token() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let url = handle_mint_download_url(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("mint 必须成功");
    let token = url
        .strip_prefix("/api/article/download?token=")
        .expect("URL 必须携带 token");
    assert!(!token.is_empty(), "token 不得为空");
    let entry = crate::repo::token::download::find_download_token(&ctx.state.cache, token)
        .expect("token 必须已铸造");
    assert_eq!(entry.version_id, version_id);
    assert_eq!(entry.user_id, user_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_requires_session() {
    let ctx = TestCtx::new().await;
    let err = handle_consume_download(
        &ctx.state,
        &ctx.ghost_session(),
        &Uuid::now_v7().to_string(),
    )
    .await
    .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_without_valid_token_returns_with_400() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_consume_download(&ctx.state, &session, &Uuid::now_v7().to_string())
        .await
        .expect_err("无 token 必须 400");
    assert!(
        matches!(err, LogicError::BadRequest(_)),
        "无 token → BadRequest(400)，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_happy_path_returns_existing_pdf_path_and_consumes_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let url = handle_mint_download_url(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("mint");
    let token = url.strip_prefix("/api/article/download?token=").unwrap();

    let path = handle_consume_download(&ctx.state, &session, token)
        .await
        .expect("consume 必须成功");
    assert!(
        std::path::Path::new(&path).is_file(),
        "返回的路径必须指向已落盘的 PDF 文件: {path}"
    );
    assert!(
        path.contains(&ctx.state.config.server.pdf_storage_path),
        "路径必须在 pdf 存储目录内"
    );

    let err = handle_consume_download(&ctx.state, &session, token)
        .await
        .expect_err("token 已消费，重放必须失败");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_after_version_deleted_returns_with_404_without_burning_token() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("keep");
    ctx.add_version(&session, &article_id, "2.0.0", "keep", Some(&v2_pdf))
        .await;
    let url = handle_mint_download_url(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("mint");
    let token = url.strip_prefix("/api/article/download?token=").unwrap();

    {
        let mut db = ctx.state.db.write().await;
        let id = crate::repo::db::resolve_node_id_sync(
            &db,
            crate::repo::types::ENTITY_TYPE_VERSION,
            &version_id,
        )
        .expect("resolve version")
        .expect("version must exist");
        db.exec_mut(agdb::QueryBuilder::remove().ids([id]).query())
            .expect("删除版本");
    }

    let err = handle_consume_download(&ctx.state, &session, token)
        .await
        .expect_err("版本已删除必须 404");
    assert!(
        matches!(err, LogicError::NotFound(_)),
        "版本缺失 → NotFound，实际: {err:?}"
    );
    assert!(
        crate::repo::token::download::find_download_token(&ctx.state.cache, token).is_some(),
        "DB 解析失败不得烧 token"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn consume_rejects_token_minted_for_another_user() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&alice_session).await;

    let url = handle_mint_download_url(&ctx.state, &alice_session, &article_id, &version_id)
        .await
        .expect("alice mint");
    let token = url.strip_prefix("/api/article/download?token=").unwrap();

    let err = handle_consume_download(&ctx.state, &bob_session, token)
        .await
        .expect_err("bob 消费 alice 的 token 必须失败");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let path = handle_consume_download(&ctx.state, &alice_session, token)
        .await
        .expect("alice 消费自己的 token");
    assert!(std::path::Path::new(&path).is_file());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mint_multiple_tokens_are_independent_and_each_single_use() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;
    let u1 = handle_mint_download_url(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("mint 1");
    let u2 = handle_mint_download_url(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("mint 2");
    let t1 = u1
        .strip_prefix("/api/article/download?token=")
        .unwrap()
        .to_string();
    let t2 = u2
        .strip_prefix("/api/article/download?token=")
        .unwrap()
        .to_string();
    assert_ne!(t1, t2, "两次 mint 必须是不同的 token");
    assert!(
        crate::repo::token::download::find_download_token(&ctx.state.cache, &t1).is_some(),
        "第一个 token 仍有效（不被覆盖）"
    );
    assert!(
        crate::repo::token::download::find_download_token(&ctx.state.cache, &t2).is_some(),
        "第二个 token 已铸造"
    );

    for t in [&t1, &t2] {
        let path = handle_consume_download(&ctx.state, &session, t)
            .await
            .expect("consume");
        assert!(std::path::Path::new(&path).is_file());
    }
    assert!(
        crate::repo::token::download::find_download_token(&ctx.state.cache, &t1).is_none()
            && crate::repo::token::download::find_download_token(&ctx.state.cache, &t2).is_none()
    );
}
