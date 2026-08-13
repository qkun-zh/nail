
use crate::unit_tests::context::TestCtx;
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

fn separate_creation() {
    std::thread::sleep(Duration::from_millis(2));
}

fn version_numbers(list: &Value) -> Vec<String> {
    list["version_list"]
        .as_array()
        .expect("响应必须有 version_list 数组")
        .iter()
        .filter_map(|v| v["version"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_versions_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.get("/article/x/version", None).await;
    ctx.unauth(status);
    let (status, _) = ctx
        .get("/article/x/version", Some(&ctx.malformed_session()))
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_versions_missing_article_is_empty_200() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, body) = ctx
        .get(
            &format!("/article/{}/version", Uuid::now_v7()),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        body["version_list"].as_array().map(Vec::len),
        Some(0),
        "文章不存在必须 200 空列表"
    );
    assert_eq!(body["total"].as_u64(), Some(0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_versions_shape_and_newest_first() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v1) = ctx.seed_article(&session).await;
    separate_creation();
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("v2");
    let v3_pdf = crate::unit_tests::context::test_pdf_variant("v3");
    let _v2 = ctx
        .add_version(&session, &article_id, "2.0.0", "second", Some(&v2_pdf))
        .await;
    separate_creation();
    let v3 = ctx
        .add_version(&session, &article_id, "3.0.0", "third", Some(&v3_pdf))
        .await;

    let (status, body) = ctx
        .get(&format!("/article/{article_id}/version"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body),
        vec!["3.0.0", "2.0.0", "1.0.0"],
        "版本列表必须按创建倒序（最新在前）"
    );
    assert_eq!(body["total"].as_u64(), Some(3));
    let entry = &body["version_list"][0];
    assert_eq!(entry["id"].as_str().unwrap(), v3, "id 必须是裸 version_id");
    assert!(entry.get("note").is_none(), "列表条目不得含 note");
    assert!(entry.get("file_path").is_none(), "列表条目不得含 file_path");
    assert!(
        entry["created_at"].as_u64().is_some(),
        "created_at 必须是秒时间戳"
    );
    assert!(
        entry["created_at"].as_u64().unwrap()
            >= body["version_list"][1]["created_at"].as_u64().unwrap(),
        "最新版本 created_at 不得早于次新版本"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_versions_pagination_clamps() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v1) = ctx.seed_article(&session).await;
    separate_creation();
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("two");
    let v3_pdf = crate::unit_tests::context::test_pdf_variant("three");
    ctx.add_version(&session, &article_id, "2.0.0", "two", Some(&v2_pdf))
        .await;
    separate_creation();
    ctx.add_version(&session, &article_id, "3.0.0", "three", Some(&v3_pdf))
        .await;

    let (status, body) = ctx
        .get(
            &format!("/article/{article_id}/version?limit=1&page=1"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body),
        vec!["3.0.0"],
        "第 1 页必须是最新版本"
    );
    assert_eq!(body["has_more"].as_bool(), Some(true));
    let (status, body) = ctx
        .get(
            &format!("/article/{article_id}/version?limit=1&page=2"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body),
        vec!["2.0.0"],
        "第 2 页必须是次新版本"
    );
    let (status, body) = ctx
        .get(
            &format!("/article/{article_id}/version?limit=1&page=4"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(version_numbers(&body).len(), 0, "越界页必须空列表而非报错");
    assert_eq!(body["total"].as_u64(), Some(3));
    let (status, body_l0) = ctx
        .get(
            &format!("/article/{article_id}/version?limit=0"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(version_numbers(&body_l0).len(), 1, "limit=0 必须钳到 1");
    let (status, body_p0) = ctx
        .get(
            &format!("/article/{article_id}/version?page=0"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    let (status, body_p1) = ctx
        .get(
            &format!("/article/{article_id}/version?page=1"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body_p0),
        version_numbers(&body_p1),
        "page=0 必须与 page=1 同页"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_version_requires_session_and_404_missing() {
    let ctx = TestCtx::new().await;
    let (status, _) = ctx.get(&format!("/version/{}", Uuid::now_v7()), None).await;
    ctx.unauth(status);
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (status, _) = ctx
        .get(&format!("/version/{}", Uuid::now_v7()), Some(&session))
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_version_ok_shape_and_article_mismatch_with_404() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_a, version_a) = ctx.seed_article(&session).await;
    let article_b = ctx
        .create_article(&session, "b", "s", "#b", "1.0.0", "n")
        .await
        .0;
    let (status, body) = ctx
        .get(&format!("/version/{version_a}"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(body["id"].as_str().unwrap(), version_a);
    assert_eq!(body["version"].as_str().unwrap(), "1.0.0");
    assert_eq!(body["note"].as_str().unwrap(), "initial");
    assert!(body["created_at"].as_u64().is_some());
    let (status, _) = ctx
        .get(
            &format!("/version/{version_a}?article_id={article_b}"),
            Some(&session),
        )
        .await;
    ctx.not_found(status);
    let (status, _) = ctx
        .get(
            &format!("/version/{version_a}?article_id={article_a}"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_requires_session_and_rejects_bad_token() {
    let ctx = TestCtx::new().await;
    let fields = [
        ("version", "1.0.0".as_bytes().to_vec()),
        ("note", "n".as_bytes().to_vec()),
        ("file", ctx.test_pdf()),
    ];
    let (status, _) = ctx
        .multipart("POST", "/article/x/version", &fields, None)
        .await;
    ctx.unauth(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            "/article/x/version",
            &fields,
            Some(&ctx.malformed_session()),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_forbidden_non_author_and_404_with_missing_article() {
    let ctx = TestCtx::new().await;
    let (_alice_id, alice) = ctx.register("alice@qq.com").await;
    let (_bob_id, bob) = ctx.register("bob@qq.com").await;
    let (article_id, _v) = ctx.seed_article(&alice).await;
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&bob),
        )
        .await;
    ctx.forbidden(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{}/version", Uuid::now_v7()),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&alice),
        )
        .await;
    ctx.not_found(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_invalid_and_non_monotonic_semver() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx.seed_article(&session).await;
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "abc".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "1.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "0.9.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_rejects_bad_note() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx.seed_article(&session).await;
    let long_note = "n".repeat(1025);
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", long_note.as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "你好".as_bytes().to_vec()),
                ("file", ctx.test_pdf()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_requires_file_field() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v) = ctx.seed_article(&session).await;
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_version_ok_and_invalid_pdf_no_leftover() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let (article_id, _v1) = ctx.seed_article(&session).await;
    let (status, _) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "n".as_bytes().to_vec()),
                ("file", b"not a pdf at all".to_vec()),
            ],
            Some(&session),
        )
        .await;
    ctx.bad(status);
    let (status, body) = ctx
        .get(&format!("/article/{article_id}/version"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body),
        vec!["1.0.0"],
        "非法 PDF 不得留下版本记录（rollback）"
    );
    let v2_pdf = crate::unit_tests::context::test_pdf_variant("second");
    let (status, body) = ctx
        .multipart(
            "POST",
            &format!("/article/{article_id}/version"),
            &[
                ("version", "2.0.0".as_bytes().to_vec()),
                ("note", "second".as_bytes().to_vec()),
                ("file", v2_pdf),
            ],
            Some(&session),
        )
        .await;
    ctx.created(status);
    let version_id = body["version_id"]
        .as_str()
        .expect("必须返回 version_id")
        .to_string();
    let (status, body) = ctx
        .get(
            &format!("/version/{version_id}?article_id={article_id}"),
            Some(&session),
        )
        .await;
    ctx.ok(status);
    assert_eq!(body["version"].as_str().unwrap(), "2.0.0");
    assert_eq!(body["note"].as_str().unwrap(), "second");
    let (status, body) = ctx
        .get(&format!("/article/{article_id}/version"), Some(&session))
        .await;
    ctx.ok(status);
    assert_eq!(
        version_numbers(&body),
        vec!["2.0.0", "1.0.0"],
        "合法版本必须追加且最新在前"
    );
}
