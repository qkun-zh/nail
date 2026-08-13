
use uuid::Uuid;

use crate::repo::article::{
    CreateArticleError, CreateVersionError, create_article, create_version,
    find_article_id_by_version, find_version_by_hash, read_article, read_article_versions,
    read_version, update_article, version_belongs_to_article,
};
use crate::unit_tests::context::TestCtx;

fn v7() -> String {
    Uuid::now_v7().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_requires_existing_author() {
    let ctx = TestCtx::new().await;
    let err = create_article(
        &ctx.state.db,
        &v7(),
        &v7(),
        "t",
        "s",
        &[],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect_err("作者不存在必须失败");
    assert!(
        matches!(err, CreateArticleError::AuthorNotFound),
        "错误必须是 AuthorNotFound（作者不存在语义），实际: {err:?}"
    );
    assert!(
        read_article(&ctx.state.db, &v7())
            .await
            .expect("查询")
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_persists_node_edges_and_creates_tags() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "Title",
        "Summary",
        &["#a".to_string(), "#b".to_string()],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");

    let row = read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询")
        .expect("存在");
    assert_eq!(row.get("title").and_then(|v| v.as_str()), Some("Title"));
    assert_eq!(row.get("summary").and_then(|v| v.as_str()), Some("Summary"));

    assert_eq!(
        crate::repo::article::edge::find_article_author_id(&ctx.state.db, &article_id)
            .await
            .expect("查询")
            .as_deref(),
        Some(user_id.as_str())
    );
    let tags = crate::repo::tag::read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("tags");
    assert_eq!(tags.len(), 2);
    let (versions, total) = read_article_versions(&ctx.state.db, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(versions.len(), 1, "建文必须带首版本");
    assert_eq!(total, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_dedupes_tag_names() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &["#a".to_string(), "#a".to_string()],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");
    let tags = crate::repo::tag::read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("tags");
    assert_eq!(tags.len(), 1, "重复 tag 名必须去重");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_article_replaces_tag_edges() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &["#old".to_string()],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");

    update_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "New Title",
        "New Summary",
        &["#new".to_string(), "#other".to_string()],
    )
    .await
    .expect("update");

    let row = read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询")
        .expect("存在");
    assert_eq!(row.get("title").and_then(|v| v.as_str()), Some("New Title"));
    let tags = crate::repo::tag::read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("tags");
    assert_eq!(tags.len(), 2);
    let names: Vec<String> = tags
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(names.contains(&"#new".to_string()));
    assert!(names.contains(&"#other".to_string()));
    assert!(!names.contains(&"#old".to_string()), "旧标签边必须删除");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_enforces_monotonicity_inside_transaction() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &[],
        &v7(),
        "0.0.1",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");

    let v1 = v7();
    create_version(
        &ctx.state.db,
        &article_id,
        &v1,
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v1),
        "note",
    )
    .await
    .expect("1.0.0");

    let err = create_version(
        &ctx.state.db,
        &article_id,
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "n",
    )
    .await
    .expect_err("重复版本必须拒绝");
    assert!(matches!(err, CreateVersionError::VersionNotGreater));
    let err = create_version(
        &ctx.state.db,
        &article_id,
        &v7(),
        "0.9.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "n",
    )
    .await
    .expect_err("更小版本必须拒绝");
    assert!(matches!(err, CreateVersionError::VersionNotGreater));
    let v2 = v7();
    create_version(
        &ctx.state.db,
        &article_id,
        &v2,
        "2.0.0",
        &crate::unit_tests::context::content_hash_for(&v2),
        "n",
    )
    .await
    .expect("2.0.0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_missing_article_returns_article_not_found() {
    let ctx = TestCtx::new().await;
    let err = create_version(
        &ctx.state.db,
        &v7(),
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "n",
    )
    .await
    .expect_err("文章缺失必须失败");
    assert!(matches!(err, CreateVersionError::ArticleNotFound));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_non_uuidv7_version_id() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &[],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");
    let err = create_version(
        &ctx.state.db,
        &article_id,
        "not-uuid",
        "1.0.0",
        &crate::unit_tests::context::content_hash_for("not-uuid"),
        "n",
    )
    .await
    .expect_err("非 uuidv7 id 必须拒绝");
    assert!(matches!(err, CreateVersionError::Db(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_invalid_semver() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &[],
        &v7(),
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "initial",
    )
    .await
    .expect("create");
    let err = create_version(
        &ctx.state.db,
        &article_id,
        &v7(),
        "not-semver",
        &crate::unit_tests::context::content_hash_for(&v7()),
        "n",
    )
    .await
    .expect_err("非法 semver 必须拒绝");
    assert!(matches!(err, CreateVersionError::InvalidVersion));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_article_versions_newest_first_with_pagination() {
    let ctx = TestCtx::new().await;
    let (user_id, _session) = ctx.register("alice@qq.com").await;
    let article_id = v7();
    let v1 = v7();
    create_article(
        &ctx.state.db,
        &article_id,
        &user_id,
        "t",
        "s",
        &[],
        &v1,
        "1.0.0",
        &crate::unit_tests::context::content_hash_for(&v1),
        "n1",
    )
    .await
    .expect("create");
    let (v2, v3) = (v7(), v7());
    create_version(
        &ctx.state.db,
        &article_id,
        &v2,
        "2.0.0",
        &crate::unit_tests::context::content_hash_for(&v2),
        "n2",
    )
    .await
    .expect("v2");
    create_version(
        &ctx.state.db,
        &article_id,
        &v3,
        "3.0.0",
        &crate::unit_tests::context::content_hash_for(&v3),
        "n3",
    )
    .await
    .expect("v3");

    let (list, total) = read_article_versions(&ctx.state.db, &article_id, 10, 0)
        .await
        .expect("全量");
    assert_eq!(total, 3);
    let ids: Vec<String> = list
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
        .collect();
    assert_eq!(
        ids,
        vec![v3, v2, v1.clone()],
        "最新在前（业务 id / uuidv7 字典序降序）"
    );

    let (list2, _) = read_article_versions(&ctx.state.db, &article_id, 2, 2)
        .await
        .expect("第二页");
    let ids2: Vec<String> = list2
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
        .collect();
    assert_eq!(ids2, vec![v1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn edge_queries_belonging_and_reverse_lookup() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    assert!(
        version_belongs_to_article(&ctx.state.db, &version_id, &article_id)
            .await
            .expect("查询"),
        "版本必须归属文章"
    );
    assert!(
        !version_belongs_to_article(&ctx.state.db, &version_id, &v7())
            .await
            .expect("查询"),
        "归属其他文章必须 false"
    );
    assert_eq!(
        find_article_id_by_version(&ctx.state.db, &version_id)
            .await
            .expect("查询")
            .as_deref(),
        Some(article_id.as_str())
    );
    assert!(
        find_article_id_by_version(&ctx.state.db, &v7())
            .await
            .expect("查询")
            .is_none()
    );
    assert!(
        read_version(&ctx.state.db, &v7())
            .await
            .expect("查询")
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_version_roundtrips_note_and_content_hash() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, version_id) = ctx.seed_article(&session).await;
    let entry = read_version(&ctx.state.db, &version_id)
        .await
        .expect("查询")
        .expect("存在");
    assert_eq!(entry.version_number, "1.0.0");
    assert_eq!(entry.note, "initial");
    let seed_pdf = crate::unit_tests::context::test_pdf_variant("seed title|1.0.0");
    assert_eq!(entry.content_hash, common::hash::pdf(&seed_pdf));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_version_by_hash_hit_returns_version_id_and_article_title() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, version_id) = ctx.seed_article(&session).await;

    let hash = common::hash::pdf(&crate::unit_tests::context::test_pdf_variant(
        "seed title|1.0.0",
    ));
    let (vid, title) = find_version_by_hash(&ctx.state.db, &hash)
        .await
        .expect("查询")
        .expect("seed 版本的 hash 必须命中");
    assert_eq!(vid, version_id, "返回第一个（唯一）版本 id");
    let row = read_article(&ctx.state.db, &article_id)
        .await
        .expect("查询");
    let title_in_db = row.and_then(|r| r.get("title").and_then(|t| t.as_str()).map(String::from));
    assert_eq!(title, title_in_db.expect("文章存在"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_version_by_hash_miss_returns_none() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (_article_id, _version_id) = ctx.seed_article(&session).await;
    let other = common::hash::pdf(b"%PDF-1.4 never uploaded\n");
    assert!(
        find_version_by_hash(&ctx.state.db, &other)
            .await
            .expect("查询")
            .is_none(),
        "未上传内容必须查不到"
    );
}
