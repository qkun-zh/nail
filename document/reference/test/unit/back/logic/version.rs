
use uuid::Uuid;

use crate::logic::error::LogicError;
use crate::logic::version::{
    content_hash_to_rel_path, get_public_pdf_path, handle_create_version,
    handle_read_article_versions, handle_read_version, validate_content_hash, validate_version,
};
use crate::unit_tests::context::{TestCtx, stage_pdf};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn validate_version_normalizes_and_rejects() {
    assert_eq!(validate_version("1.0.0").expect("valid"), "1.0.0");
    assert_eq!(validate_version("  1.2.3  ").expect("trim"), "1.2.3");
    let err = validate_version("v1.2.3").expect_err("v 前缀必须拒绝（严格 semver）");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = validate_version("").expect_err("空版本必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = validate_version("  ").expect_err("纯空白必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = validate_version("not-a-version").expect_err("非法 semver 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = validate_version("1.0").expect_err("缺 patch 段必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_requires_author() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _) = ctx.seed_article(&alice_session).await;

    let err = handle_create_version(
        &ctx.state,
        &bob_session,
        &article_id,
        "2.0.0",
        "note",
        stage_pdf(&ctx.test_pdf()),
    )
    .await
    .expect_err("非作者加版本必须 403");
    assert!(matches!(err, LogicError::Forbidden(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_missing_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_create_version(
        &ctx.state,
        &session,
        &Uuid::now_v7().to_string(),
        "1.0.0",
        "note",
        stage_pdf(&ctx.test_pdf()),
    )
    .await
    .expect_err("文章不存在必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_enforces_strict_monotonicity() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;

    let err = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "1.0.0",
        "n",
        stage_pdf(&ctx.test_pdf()),
    )
    .await
    .expect_err("重复版本必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "0.9.0",
        "n",
        stage_pdf(&ctx.test_pdf()),
    )
    .await
    .expect_err("小于现有最大必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let v2_pdf = crate::unit_tests::context::test_pdf_variant("major bump");
    let version_id = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "major bump",
        stage_pdf(&v2_pdf),
    )
    .await
    .expect("更大版本必须成功");
    assert!(!version_id.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_validates_note() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;

    let err = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "",
        stage_pdf(&ctx.test_pdf()),
    )
    .await
    .expect_err("空 note 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let err = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "中文 note",
        stage_pdf(&crate::unit_tests::context::test_pdf_variant(
            "chinese note",
        )),
    )
    .await
    .expect_err("非 ASCII note 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_duplicate_pdf_content() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;

    let seed_pdf = crate::unit_tests::context::test_pdf_variant("seed title|1.0.0");
    let err = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "n",
        stage_pdf(&seed_pdf),
    )
    .await
    .expect_err("相同 PDF 内容必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let msg = err.to_string();
    assert!(
        msg.contains("identical PDF already exists"),
        "错误消息必须含去重语义: {msg}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_writes_pdf_file() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("note");
    handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "note",
        stage_pdf(&v2_pdf),
    )
    .await
    .expect("加版本");
    let hash = common::hash::pdf(&v2_pdf);
    let full = format!(
        "{}/{}/{}/{}.pdf",
        ctx.state.config.server.pdf_storage_path,
        &hash[0..2],
        &hash[2..4],
        hash
    );
    assert!(
        std::path::Path::new(&full).is_file(),
        "PDF 必须落盘: {full}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_article_versions_paginates_newest_first() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, v1) = ctx.seed_article(&session).await;
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("v2");
    let v3_pdf = crate::unit_tests::context::test_pdf_variant("v3");
    let v2 = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "2.0.0",
        "n2",
        stage_pdf(&v2_pdf),
    )
    .await
    .expect("v2");
    let v3 = handle_create_version(
        &ctx.state,
        &session,
        &article_id,
        "3.0.0",
        "n3",
        stage_pdf(&v3_pdf),
    )
    .await
    .expect("v3");

    let (list, total) = handle_read_article_versions(&ctx.state, &article_id, 10, 0)
        .await
        .expect("列表");
    assert_eq!(total, 3);
    let ids: Vec<String> = list
        .iter()
        .filter_map(|v| {
            v.get("id")
                .and_then(|i| i.as_str())
                .map(|s| crate::repo::util::strip_record_id(s))
        })
        .collect();
    assert_eq!(ids, vec![v3, v2, v1], "最新在前（含 seed 的 1.0.0）");
    for row in &list {
        assert!(
            row.get("version_number").is_some(),
            "列表行必须含 version_number"
        );
    }

    let (list2, total2) = handle_read_article_versions(&ctx.state, &article_id, 2, 2)
        .await
        .expect("第二页");
    assert_eq!(total2, 3);
    assert_eq!(list2.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_version_checks_article_ownership_when_requested() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let entry = handle_read_version(&ctx.state, &session, &version_id, None)
        .await
        .expect("查询")
        .expect("版本存在");
    assert_eq!(entry.version_number, "1.0.0");
    assert_eq!(entry.note, "initial");

    let entry = handle_read_version(&ctx.state, &session, &version_id, Some(&article_id))
        .await
        .expect("查询")
        .expect("版本存在");
    assert_eq!(entry.version_number, "1.0.0");

    let entry = handle_read_version(
        &ctx.state,
        &session,
        &version_id,
        Some(&Uuid::now_v7().to_string()),
    )
    .await
    .expect("查询");
    assert!(entry.is_none(), "归属不符必须 None");

    let err = handle_read_version(&ctx.state, &session, &Uuid::now_v7().to_string(), None)
        .await
        .expect_err("不存在的版本必须报错");
    assert!(matches!(err, LogicError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_public_pdf_path_resolves_content_addressed_path() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let path = get_public_pdf_path(&ctx.state, &article_id, &version_id)
        .await
        .expect("上传后必须放行");
    assert!(std::path::Path::new(&path).is_file());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn validate_content_hash_and_rel_path_matrix() {
    assert!(validate_content_hash("a1b2c3d4e5f60718293a4b5c6d7e8f90").is_ok());
    assert_eq!(
        content_hash_to_rel_path("a1b2c3d4e5f60718293a4b5c6d7e8f90").expect("valid"),
        "a1/b2/a1b2c3d4e5f60718293a4b5c6d7e8f90.pdf"
    );
    for bad in [
        "",
        "abc",
        "a1b2c3d4e5f60718293a4b5c6d7e8f9",
        "a1b2c3d4e5f60718293a4b5c6d7e8f900",
        "A1B2C3D4E5F60718293A4B5C6D7E8F90",
        "a1b2c3d4e5f60718293a4b5c6d7e8f9g",
        "a1b2c3d4e5f60718293a4b5c6d7e8f9!",
        "../a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "/etc/passwd",
        "a1/b2/c3d4e5f60718293a4b5c6d7e8f90",
    ] {
        assert!(
            validate_content_hash(bad).is_err(),
            "content hash guard must reject {bad:?}"
        );
        assert!(
            content_hash_to_rel_path(bad).is_err(),
            "rel path must reject {bad:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_public_pdf_path_checks_belonging_and_hash_path() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let path = get_public_pdf_path(&ctx.state, &article_id, &version_id)
        .await
        .expect("正确归属返回路径");
    let seed_pdf = crate::unit_tests::context::test_pdf_variant("seed title|1.0.0");
    let hash = common::hash::pdf(&seed_pdf);
    let expected = format!(
        "{}/{}/{}/{}.pdf",
        ctx.state.config.server.pdf_storage_path,
        &hash[0..2],
        &hash[2..4],
        hash
    );
    assert_eq!(path, expected, "路径必须由 content_hash 派生");
    assert!(std::path::Path::new(&path).is_file());

    let err = get_public_pdf_path(&ctx.state, &Uuid::now_v7().to_string(), &version_id)
        .await
        .expect_err("归属不符必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}
