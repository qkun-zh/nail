
use common::tag::parse_hashtag_tags;
use uuid::Uuid;

use crate::logic::article::{
    handle_create_article, handle_get_pdf_path, handle_read_article, handle_read_articles,
    handle_update_article,
};
use crate::logic::error::LogicError;
use crate::unit_tests::context::{TestCtx, stage_pdf};

fn create_pdf() -> Vec<u8> {
    crate::unit_tests::context::test_pdf()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_requires_session() {
    let ctx = TestCtx::new().await;
    let err = handle_create_article(
        &ctx.state,
        &ctx.ghost_session(),
        "t",
        "s",
        "#tag",
        "1.0.0",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("无 session 必须拒绝");
    assert!(matches!(err, LogicError::Unauthorized(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_validates_ascii_and_lengths() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;

    let err = handle_create_article(
        &ctx.state,
        &session,
        "中文标题",
        "summary",
        "#tag",
        "1.0.0",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("非 ASCII title 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let err = handle_create_article(
        &ctx.state,
        &session,
        "ti\ntle",
        "summary",
        "#tag",
        "1.0.0",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("title 含换行必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    let (article_id, _version_id) = handle_create_article(
        &ctx.state,
        &session,
        "t",
        "sum\nmary",
        "#tag",
        "1.0.0",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect("summary 可含换行");
    assert!(!article_id.is_empty());

    let tags = parse_hashtag_tags("#a#b#c", 8).expect("parse");
    assert_eq!(tags, vec!["#a", "#b", "#c"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_rejects_missing_tags() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    for raw_tags in ["", "   "] {
        let err = handle_create_article(
            &ctx.state,
            &session,
            "t",
            "s",
            raw_tags,
            "1.0.0",
            "n",
            stage_pdf(&create_pdf()),
        )
        .await
        .expect_err("空 tags 必须拒绝");
        assert!(
            matches!(err, LogicError::BadRequest(_)),
            "空 tags → BadRequest，实际: {err:?}"
        );
        assert!(
            err.to_string().contains("at least one tag"),
            "reason 应含至少一个 tag 语义，实际: {err}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_rejects_missing_version_or_note() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_create_article(
        &ctx.state,
        &session,
        "t",
        "s",
        "#tag",
        "",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("缺版本必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
    let err = handle_create_article(
        &ctx.state,
        &session,
        "t",
        "s",
        "#tag",
        "1.0.0",
        "",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("缺 note 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_rejects_invalid_version_semver() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_create_article(
        &ctx.state,
        &session,
        "t",
        "s",
        "#tag",
        "not-semver",
        "n",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect_err("非法版本号必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_persists_and_returns_identifiers() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = handle_create_article(
        &ctx.state,
        &session,
        "My Title",
        "My Summary",
        "#rust #web",
        "1.0.0",
        "initial",
        stage_pdf(&create_pdf()),
    )
    .await
    .expect("建文成功");
    assert!(!article_id.is_empty());
    assert!(!version_id.is_empty(), "create 响应必须带 version_id");
    let article = crate::repo::article::read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询")
        .expect("文章必须存在");
    assert_eq!(
        article.get("title").and_then(|v| v.as_str()),
        Some("My Title")
    );
    assert_eq!(
        article.get("summary").and_then(|v| v.as_str()),
        Some("My Summary")
    );
    let tag_rows = crate::repo::tag::read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("查询 tags");
    assert_eq!(tag_rows.len(), 2);
    let names: Vec<String> = tag_rows
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(names.contains(&"#rust".to_string()));
    assert!(names.contains(&"#web".to_string()));
    let (versions, total) =
        crate::repo::article::read_article_versions(&ctx.state.db, &article_id, 10, 0)
            .await
            .expect("查询版本");
    assert_eq!(total, 1, "建文必须带首版本");
    let ids: Vec<String> = versions
        .iter()
        .filter_map(|v| {
            v.get("id")
                .and_then(|i| i.as_str())
                .map(crate::repo::util::strip_record_id)
        })
        .collect();
    assert_eq!(ids, vec![version_id], "首版本 id 必须与响应一致");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_article_requires_author() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice_session) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob_session) = ctx.register("bob@qq.com").await;
    let (article_id, _v) = ctx
        .create_article(&alice_session, "t", "s", "#tag", "1.0.0", "n")
        .await;
    let err = handle_update_article(&ctx.state, &bob_session, &article_id, "new", "new", "#tag")
        .await
        .expect_err("非作者必须 403");
    assert!(matches!(err, LogicError::Forbidden(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_article_validates_same_rules_as_create() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx
        .create_article(&session, "t", "s", "#tag", "1.0.0", "n")
        .await;

    let err = handle_update_article(&ctx.state, &session, &article_id, "中文", "s", "#tag")
        .await
        .expect_err("非 ASCII title 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));

    handle_update_article(
        &ctx.state,
        &session,
        &article_id,
        "new",
        "sum\nmary",
        "#tag",
    )
    .await
    .expect("summary 可换行");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_article_rejects_missing_tags() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx
        .create_article(&session, "t", "s", "#tag", "1.0.0", "n")
        .await;
    let err = handle_update_article(&ctx.state, &session, &article_id, "t", "s", "")
        .await
        .expect_err("空 tags 必须拒绝");
    assert!(matches!(err, LogicError::BadRequest(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_article_replaces_tags() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx
        .create_article(&session, "t", "s", "#old", "1.0.0", "n")
        .await;

    handle_update_article(&ctx.state, &session, &article_id, "t", "s", "#new #other")
        .await
        .expect("改标签");

    let tag_rows = crate::repo::tag::read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("查询 tags");
    assert_eq!(tag_rows.len(), 2);
    let names: Vec<String> = tag_rows
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(names.contains(&"#new".to_string()));
    assert!(names.contains(&"#other".to_string()));
    let leftover =
        crate::repo::tag::find_tag_ids_by_names_contains(&ctx.state.db, &["#old".to_string()])
            .await
            .expect("查询");
    assert!(leftover.is_empty(), "旧 tag 边移除后标签已被清理");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_articles_paginates_and_reverse_scans() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a1 = ctx
        .create_article(&session, "t1", "s1", "#t1", "1.0.0", "n")
        .await
        .0;
    let a2 = ctx
        .create_article(&session, "t2", "s2", "#t2", "1.0.0", "n")
        .await
        .0;
    let a3 = ctx
        .create_article(&session, "t3", "s3", "#t3", "1.0.0", "n")
        .await
        .0;

    let (list, has_more, total) = handle_read_articles(&ctx.state, 2, 0)
        .await
        .expect("list page 1");
    assert_eq!(total, 3);
    assert_eq!(list.len(), 2);
    assert!(has_more);
    let ids: Vec<String> = list
        .iter()
        .filter_map(|v| {
            v.get("id")
                .and_then(|i| i.as_str())
                .map(|s| crate::repo::util::strip_record_id(s))
        })
        .collect();
    assert_eq!(ids[0], a3);
    assert_eq!(ids[1], a2);

    let (list2, has_more2, total2) = handle_read_articles(&ctx.state, 2, 2)
        .await
        .expect("list page 2");
    assert_eq!(total2, 3);
    assert_eq!(list2.len(), 1);
    assert!(!has_more2);
    let ids2: Vec<String> = list2
        .iter()
        .filter_map(|v| {
            v.get("id")
                .and_then(|i| i.as_str())
                .map(|s| crate::repo::util::strip_record_id(s))
        })
        .collect();
    assert_eq!(ids2[0], a1);

    let (list3, _, _) = handle_read_articles(&ctx.state, 2, 4)
        .await
        .expect("越界页");
    assert!(list3.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_article_returns_metadata_only() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx
        .create_article(&session, "My Title", "My Summary", "#tag", "1.0.0", "n")
        .await;
    let article = handle_read_article(&ctx.state, &session, &article_id)
        .await
        .expect("read 文章");
    assert_eq!(
        article
            .get("id")
            .and_then(|v| v.as_str())
            .map(crate::repo::util::strip_record_id),
        Some(article_id.clone())
    );
    assert_eq!(
        article.get("title").and_then(|v| v.as_str()),
        Some("My Title")
    );
    assert_eq!(
        article.get("summary").and_then(|v| v.as_str()),
        Some("My Summary")
    );
    assert!(
        article.get("version_list").is_none()
            || article
                .get("version_list")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_article_not_found_returns_with_404() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let err = handle_read_article(&ctx.state, &session, &Uuid::now_v7().to_string())
        .await
        .expect_err("不存在文章必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_pdf_path_requires_version_to_belong_to_article() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let path = handle_get_pdf_path(&ctx.state, &session, &article_id, &version_id)
        .await
        .expect("正确归属");
    assert!(std::path::Path::new(&path).is_file());

    let err = handle_get_pdf_path(
        &ctx.state,
        &session,
        &Uuid::now_v7().to_string(),
        &version_id,
    )
    .await
    .expect_err("归属不符必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_pdf_path_rejects_missing_version() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _) = ctx.seed_article(&session).await;
    let err = handle_get_pdf_path(
        &ctx.state,
        &session,
        &article_id,
        &Uuid::now_v7().to_string(),
    )
    .await
    .expect_err("版本不存在必须 404");
    assert!(matches!(err, LogicError::NotFound(_)));
}
