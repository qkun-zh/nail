
use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{
    PERMISSION_ARTICLE_DELETE, PERMISSION_ARTICLE_READ, PERMISSION_ARTICLE_UPDATE,
    PERMISSION_COMMENT_DELETE, PERMISSION_PDF_DOWNLOAD, PERMISSION_VERSION_CREATE, authorize,
    is_allowed,
};
use crate::repo;
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn permission_points_are_seeded_and_indexed() {
    let ctx = TestCtx::new().await;
    let db = ctx.state.db.read().await;
    for permission in repo::authorization::ALL_PERMISSIONS {
        let hits =
            crate::repo::db::find_by_index_sync(&db, repo::types::KEY_PERMISSION_NAME, permission)
                .expect("索引查询");
        assert_eq!(
            hits.len(),
            1,
            "权限点 {permission} 必须恰好一个（种子幂等）"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn role_grant_hold_and_scope_edges_are_persisted() {
    let ctx = TestCtx::new().await;
    let (_user_id, _session) = ctx.register("alice@qq.com").await;

    repo::authorization::create_role(&ctx.state.db, "editor")
        .await
        .expect("建角色");
    repo::authorization::grant_permission_to_role(
        &ctx.state.db,
        "editor",
        repo::authorization::PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("授予");
    repo::authorization::hold_role(&ctx.state.db, &_user_id, "editor")
        .await
        .expect("持有");
    repo::authorization::apply_tag_to_role(&ctx.state.db, "editor", "#docs")
        .await
        .expect("作用域 tag");

    repo::authorization::create_role(&ctx.state.db, "editor")
        .await
        .expect("重复建角色幂等");
    repo::authorization::grant_permission_to_role(
        &ctx.state.db,
        "editor",
        repo::authorization::PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("重复授予幂等");
    repo::authorization::hold_role(&ctx.state.db, &_user_id, "editor")
        .await
        .expect("重复持有幂等");

    let auth = repo::authorization::read_user_authorization(&ctx.state.db, &_user_id)
        .await
        .expect("读取授权图");
    assert_eq!(auth.roles.len(), 1);
    assert_eq!(auth.roles[0].role_name, "editor");
    assert_eq!(
        auth.roles[0].permissions,
        vec![repo::authorization::PERMISSION_ARTICLE_UPDATE.to_string()]
    );
    assert_eq!(auth.roles[0].scopes, vec!["#docs".to_string()]);
    assert!(!auth.has_global_role, "带 tag 作用域的角色不是全局角色");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn global_role_has_no_scope_restriction() {
    let ctx = TestCtx::new().await;
    let (_user_id, _session) = ctx.register("alice@qq.com").await;
    repo::authorization::create_role(&ctx.state.db, "moderator")
        .await
        .expect("建角色");
    repo::authorization::hold_role(&ctx.state.db, &_user_id, "moderator")
        .await
        .expect("持有");
    let auth = repo::authorization::read_user_authorization(&ctx.state.db, &_user_id)
        .await
        .expect("读取");
    assert!(auth.has_global_role, "无 tag 边的角色 = 全局角色");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn owner_can_read_update_and_delete_own_article() {
    let ctx = TestCtx::new().await;
    let (user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;

    assert!(
        is_allowed(
            &ctx.state,
            &user_id,
            PERMISSION_ARTICLE_READ,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        is_allowed(
            &ctx.state,
            &user_id,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        is_allowed(
            &ctx.state,
            &user_id,
            PERMISSION_ARTICLE_DELETE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        is_allowed(
            &ctx.state,
            &user_id,
            PERMISSION_PDF_DOWNLOAD,
            &Resource::Article(article_id),
        )
        .await
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stranger_is_denied_owner_actions_fail_closed() {
    let ctx = TestCtx::new().await;
    let (_alice, alice_session) = ctx.register("alice@qq.com").await;
    let (bob, _bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _) = ctx.seed_article(&alice_session).await;

    assert!(
        !is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        !is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_ARTICLE_DELETE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        !is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_VERSION_CREATE,
            &Resource::Article(article_id),
        )
        .await
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn public_articles_are_readable_by_anyone() {
    let ctx = TestCtx::new().await;
    let (_alice, alice_session) = ctx.register("alice@qq.com").await;
    let (bob, _bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _) = ctx.seed_article(&alice_session).await;

    assert!(
        is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_ARTICLE_READ,
            &Resource::Article(article_id.clone()),
        )
        .await
    );
    assert!(
        is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_PDF_DOWNLOAD,
            &Resource::Article(article_id),
        )
        .await
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn version_and_comment_resources_inherit_article_visibility() {
    let ctx = TestCtx::new().await;
    let (_alice, alice_session) = ctx.register("alice@qq.com").await;
    let (bob, _bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&alice_session).await;

    assert!(
        is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_ARTICLE_READ,
            &Resource::Version(version_id.clone()),
        )
        .await
    );

    let comment_id = ctx
        .post(
            &format!("/version/{version_id}/comments"),
            serde_json::json!({ "content": "hi" }),
            Some(&alice_session),
        )
        .await;
    assert_eq!(comment_id.0, axum::http::StatusCode::CREATED);
    let cid = comment_id.1["comment_id"].as_str().unwrap().to_string();
    assert!(
        is_allowed(
            &ctx.state,
            &_alice,
            PERMISSION_COMMENT_DELETE,
            &Resource::Comment(cid.clone()),
        )
        .await
    );
    assert!(
        is_allowed(
            &ctx.state,
            &_alice,
            PERMISSION_ARTICLE_READ,
            &Resource::Version(version_id),
        )
        .await
    );
    let _ = article_id;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn role_grant_crosses_ownership_with_scope_intersection() {
    let ctx = TestCtx::new().await;
    let (_alice, alice_session) = ctx.register("alice@qq.com").await;
    let (bob, _bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _) = ctx.seed_article(&alice_session).await;
    ctx.post(
        &format!("/article/{article_id}"),
        serde_json::json!({ "title": "seed title", "summary": "seed summary", "tags": "#seed#docs" }),
        Some(&alice_session),
    )
    .await;

    repo::authorization::create_role(&ctx.state.db, "editor")
        .await
        .expect("建角色");
    repo::authorization::grant_permission_to_role(
        &ctx.state.db,
        "editor",
        repo::authorization::PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("授予");
    repo::authorization::apply_tag_to_role(&ctx.state.db, "editor", "#docs")
        .await
        .expect("作用域");
    repo::authorization::hold_role(&ctx.state.db, &bob, "editor")
        .await
        .expect("持有");

    assert!(
        is_allowed(
            &ctx.state,
            &bob,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );

    let (carol, _carol_session) = ctx.register("carol@qq.com").await;
    let (charlie, _charlie_session) = ctx.register("charlie@qq.com").await;
    repo::authorization::create_role(&ctx.state.db, "editor-other")
        .await
        .expect("建角色");
    repo::authorization::grant_permission_to_role(
        &ctx.state.db,
        "editor-other",
        repo::authorization::PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("授予");
    repo::authorization::apply_tag_to_role(&ctx.state.db, "editor-other", "#other")
        .await
        .expect("作用域");
    repo::authorization::hold_role(&ctx.state.db, &charlie, "editor-other")
        .await
        .expect("持有");
    assert!(
        !is_allowed(
            &ctx.state,
            &charlie,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id.clone()),
        )
        .await
    );

    let (dave, _dave_session) = ctx.register("dave@qq.com").await;
    repo::authorization::create_role(&ctx.state.db, "super-editor")
        .await
        .expect("建角色");
    repo::authorization::grant_permission_to_role(
        &ctx.state.db,
        "super-editor",
        repo::authorization::PERMISSION_ARTICLE_UPDATE,
    )
    .await
    .expect("授予");
    repo::authorization::hold_role(&ctx.state.db, &dave, "super-editor")
        .await
        .expect("持有");
    assert!(
        is_allowed(
            &ctx.state,
            &dave,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(article_id),
        )
        .await
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authorize_returns_not_found_for_missing_resource() {
    let ctx = TestCtx::new().await;
    let (_user_id, _session) = ctx.register("alice@qq.com").await;
    let err = authorize(
        &ctx.state,
        &_user_id,
        PERMISSION_ARTICLE_UPDATE,
        &Resource::Article(uuid::Uuid::now_v7().to_string()),
    )
    .await
    .expect_err("资源缺失必须 NotFound");
    assert!(
        matches!(err, crate::logic::error::LogicError::NotFound(_)),
        "实际: {err:?}"
    );
    assert!(
        !is_allowed(
            &ctx.state,
            &_user_id,
            PERMISSION_ARTICLE_UPDATE,
            &Resource::Article(uuid::Uuid::now_v7().to_string()),
        )
        .await
    );
}
