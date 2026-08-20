use super::context::{TestCtx, unique_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops"];

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

async fn create_seeded_article(
    context: &TestCtx,
    actor_id: &str,
    title: &str,
    version: &str,
    note: &str,
) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    crate::logic::article::create_article(
        &context.state,
        actor_id,
        crate::logic::article::ArticleCreateInput {
            title,
            summary: "summary",
            tags: "rust",
            version,
            note,
            upload: context.upload(&unique_pdf(title)),
        },
    )
    .await
    .expect("create article")
}

#[test]
fn paginate_returns_the_page_slice_and_has_next() {
    let items: Vec<u64> = (0..10).collect();
    let (page, has_next) = crate::logic::pagination::paginate(items, 1, 4);
    assert_eq!(page, vec![0, 1, 2, 3]);
    assert!(has_next);

    let items: Vec<u64> = (0..10).collect();
    let (page, has_next) = crate::logic::pagination::paginate(items, 3, 4);
    assert_eq!(page, vec![8, 9]);
    assert!(!has_next);
}

#[test]
fn paginate_page_zero_behaves_like_page_one() {
    let items: Vec<u64> = (0..10).collect();
    let (page, has_next) = crate::logic::pagination::paginate(items, 0, 4);
    assert_eq!(page, vec![0, 1, 2, 3]);
    assert!(has_next);
}

#[test]
fn paginate_a_huge_page_is_empty_without_next() {
    let items: Vec<u64> = (0..10).collect();
    let (page, has_next) = crate::logic::pagination::paginate(items, u64::MAX, 4);
    assert!(page.is_empty());
    assert!(!has_next);
}

#[test]
fn paginate_an_empty_collection_has_no_next() {
    let (page, has_next) = crate::logic::pagination::paginate(Vec::<u64>::new(), 1, 4);
    assert!(page.is_empty());
    assert!(!has_next);
}

#[test]
fn page_offset_is_one_based() {
    assert_eq!(crate::logic::pagination::page_offset(1, 4), 0);
    assert_eq!(crate::logic::pagination::page_offset(2, 4), 4);
    assert_eq!(crate::logic::pagination::page_offset(0, 4), 0);
    assert_eq!(
        crate::logic::pagination::page_offset(u64::MAX, 200),
        u64::MAX
    );
}

#[test]
fn paginate_matches_the_legacy_offset_form_has_next() {
    for total in 0..=32u64 {
        for page in 1..=8u64 {
            for limit in 1..=5u64 {
                let offset = page.saturating_sub(1).saturating_mul(limit);
                let (_, has_next) =
                    crate::logic::pagination::paginate((0..total).collect(), page, limit);
                let legacy = total > offset + limit;
                assert_eq!(has_next, legacy, "total={total} page={page} limit={limit}");
            }
        }
    }
}

#[tokio::test]
async fn version_pages_tile_the_full_history_exactly_once() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Tiled Versions", "1.0.0", "one").await;
    for i in 2..=5 {
        crate::logic::version::create_version(
            &context.state,
            &actor,
            &article_id,
            &format!("1.0.{i}"),
            &format!("note {i}"),
            context.upload(&unique_pdf(&format!("v{i}"))),
        )
        .await
        .expect("create version");
    }

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=5 {
        let result =
            crate::logic::version::read_versions(&context.state, &actor, &article_id, page, 1)
                .await
                .expect("page");
        assert_eq!(result.items.len(), 1, "page {page} must hold one item");
        assert_eq!(
            result.has_next,
            page < 5,
            "has_next on page {page} must reflect the remaining tail"
        );
        seen.push(result.items[0].version.clone());
    }
    assert_eq!(seen.len(), 5, "five pages must yield five versions");
    assert_eq!(
        seen.iter().filter(|v| **v == "1.0.3").count(),
        1,
        "no version may repeat"
    );
}

#[tokio::test]
async fn version_pages_with_limit_two_tile_exactly() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Tiled Two", "1.0.0", "one").await;
    for i in 2..=6 {
        crate::logic::version::create_version(
            &context.state,
            &actor,
            &article_id,
            &format!("1.0.{i}"),
            "note",
            context.upload(&unique_pdf(&format!("v{i}"))),
        )
        .await
        .expect("create version");
    }

    let page_one = crate::logic::version::read_versions(&context.state, &actor, &article_id, 1, 2)
        .await
        .expect("page 1");
    let page_two = crate::logic::version::read_versions(&context.state, &actor, &article_id, 2, 2)
        .await
        .expect("page 2");
    let page_three =
        crate::logic::version::read_versions(&context.state, &actor, &article_id, 3, 2)
            .await
            .expect("page 3");
    assert_eq!(page_one.items.len(), 2);
    assert!(page_one.has_next);
    assert_eq!(page_two.items.len(), 2);
    assert!(page_two.has_next);
    assert_eq!(page_three.items.len(), 2, "last page is partial");
    assert!(!page_three.has_next);
}

#[tokio::test]
async fn version_page_beyond_the_end_is_empty() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Beyond End", "1.0.0", "one").await;

    let page = crate::logic::version::read_versions(&context.state, &actor, &article_id, 9, 10)
        .await
        .expect("far page");
    assert!(page.items.is_empty());
    assert!(!page.has_next);
}

#[tokio::test]
async fn version_limit_larger_than_total_returns_everything_without_next() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Wide Limit", "1.0.0", "one").await;
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "two",
        context.upload(&unique_pdf("wv2")),
    )
    .await
    .expect("create v2");

    let page = crate::logic::version::read_versions(&context.state, &actor, &article_id, 1, 100)
        .await
        .expect("wide page");
    assert_eq!(page.items.len(), 2);
    assert!(!page.has_next);
}

#[tokio::test]
async fn version_pages_are_stable_across_repeated_reads() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Stable Pages", "1.0.0", "one").await;
    for i in 2..=4 {
        crate::logic::version::create_version(
            &context.state,
            &actor,
            &article_id,
            &format!("1.0.{i}"),
            "note",
            context.upload(&unique_pdf(&format!("sv{i}"))),
        )
        .await
        .expect("create version");
    }

    let first = crate::logic::version::read_versions(&context.state, &actor, &article_id, 1, 2)
        .await
        .expect("first read");
    let second = crate::logic::version::read_versions(&context.state, &actor, &article_id, 1, 2)
        .await
        .expect("second read");
    let ids = |page: &nail_common::response::ListPage<
        nail_common::response::version::VersionListItem,
    >|
     -> Vec<String> { page.items.iter().map(|item| item.id.clone()).collect() };
    assert_eq!(ids(&first), ids(&second), "identical read must not drift");
}

#[tokio::test]
async fn version_pages_tile_exactly_when_a_middle_version_is_soft_deleted() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Gapped Versions", "1.0.0", "one").await;
    let mut middle = String::new();
    for i in 2..=4 {
        let version_id = crate::logic::version::create_version(
            &context.state,
            &actor,
            &article_id,
            &format!("1.0.{i}"),
            "note",
            context.upload(&unique_pdf(&format!("gv{i}"))),
        )
        .await
        .expect("create version");
        if i == 3 {
            middle = version_id;
        }
    }
    crate::logic::version::delete_version(
        &context.state,
        &actor,
        &middle,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete middle");

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=3 {
        let result =
            crate::logic::version::read_versions(&context.state, &actor, &article_id, page, 1)
                .await
                .expect("page");
        assert_eq!(result.items.len(), 1, "page {page}");
        seen.push(result.items[0].version.clone());
    }
    assert_eq!(seen, vec!["1.0.4", "1.0.2", "1.0.0"]);
}

#[tokio::test]
async fn version_soft_deleted_first_version_does_not_shift_later_pages() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, first_version_id) =
        create_seeded_article(&context, &actor, "Headless Version", "1.0.0", "one").await;
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "two",
        context.upload(&unique_pdf("hv2")),
    )
    .await
    .expect("create v2");

    crate::logic::version::delete_version(
        &context.state,
        &actor,
        &first_version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete first");

    let page = crate::logic::version::read_versions(&context.state, &actor, &article_id, 1, 10)
        .await
        .expect("page");
    assert_eq!(page.items.len(), 1, "only the live version remains");
    assert_eq!(page.items[0].version, "2.0.0");
}

#[tokio::test]
async fn comment_pages_tile_all_top_level_comments_exactly_once() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Comment Tiling", "1.0.0", "one").await;
    let mut expected: Vec<String> = Vec::new();
    for i in 1..=4 {
        let comment_id = crate::logic::comment::create_comment(
            &context.state,
            &actor,
            &version_id,
            &format!("top comment {i}"),
        )
        .await
        .expect("comment");
        expected.push(comment_id);
    }

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=4 {
        let result =
            crate::logic::comment::read_comments(&context.state, &actor, &version_id, page, 1)
                .await
                .expect("page");
        assert_eq!(result.items.len(), 1, "page {page} holds one comment");
        assert_eq!(result.has_next, page < 4, "has_next on page {page}");
        seen.push(result.items[0].id.clone());
    }
    expected.sort_unstable();
    seen.sort_unstable();
    assert_eq!(seen, expected, "single-item pages must tile every comment");
}

#[tokio::test]
async fn comment_page_beyond_the_end_is_empty() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Empty Tail", "1.0.0", "one").await;
    crate::logic::comment::create_comment(&context.state, &actor, &version_id, "only comment")
        .await
        .expect("comment");

    let page = crate::logic::comment::read_comments(&context.state, &actor, &version_id, 9, 10)
        .await
        .expect("far page");
    assert!(page.items.is_empty());
    assert!(!page.has_next);
}

#[tokio::test]
async fn comment_pages_tile_only_live_comments_when_one_is_soft_deleted() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Gapped Comments", "1.0.0", "one").await;
    let mut ids: Vec<String> = Vec::new();
    for i in 1..=4 {
        let comment_id = crate::logic::comment::create_comment(
            &context.state,
            &actor,
            &version_id,
            &format!("gap comment {i}"),
        )
        .await
        .expect("comment");
        ids.push(comment_id);
    }
    crate::logic::comment::delete_comment(
        &context.state,
        &actor,
        &ids[1],
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete one");

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=3 {
        let result =
            crate::logic::comment::read_comments(&context.state, &actor, &version_id, page, 1)
                .await
                .expect("page");
        assert_eq!(result.items.len(), 1, "page {page}");
        seen.push(result.items[0].id.clone());
    }
    let mut expected = ids.clone();
    expected.remove(1);
    expected.sort_unstable();
    seen.sort_unstable();
    assert_eq!(
        seen, expected,
        "soft-deleted comment must not consume a page slot"
    );
}

#[tokio::test]
async fn reply_pages_tile_the_full_thread_exactly_once() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Reply Tiling", "1.0.0", "one").await;
    let top = crate::logic::comment::create_comment(&context.state, &actor, &version_id, "top")
        .await
        .expect("top");
    let mut expected: Vec<String> = Vec::new();
    for i in 1..=4 {
        let reply_id = crate::logic::comment::create_reply(
            &context.state,
            &actor,
            &top,
            &format!("reply {i}"),
        )
        .await
        .expect("reply");
        expected.push(reply_id);
    }

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=4 {
        let result =
            crate::logic::comment::read_comment_children(&context.state, &actor, &top, page, 1)
                .await
                .expect("page");
        assert_eq!(result.items.len(), 1, "page {page}");
        assert_eq!(result.has_next, page < 4, "has_next on page {page}");
        seen.push(result.items[0].id.clone());
    }
    expected.sort_unstable();
    seen.sort_unstable();
    assert_eq!(seen, expected, "single-item pages must tile every reply");
}

#[tokio::test]
async fn reply_pages_beyond_the_end_are_empty() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Reply Tail", "1.0.0", "one").await;
    let top = crate::logic::comment::create_comment(&context.state, &actor, &version_id, "top")
        .await
        .expect("top");
    crate::logic::comment::create_reply(&context.state, &actor, &top, "single reply")
        .await
        .expect("reply");

    let page = crate::logic::comment::read_comment_children(&context.state, &actor, &top, 9, 10)
        .await
        .expect("far page");
    assert!(page.items.is_empty());
    assert!(!page.has_next);
}

#[tokio::test]
async fn reply_pages_reject_a_soft_deleted_parent() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Dead Parent", "1.0.0", "one").await;
    let top = crate::logic::comment::create_comment(&context.state, &actor, &version_id, "top")
        .await
        .expect("top");
    crate::logic::comment::create_reply(&context.state, &actor, &top, "reply")
        .await
        .expect("reply");
    crate::logic::comment::delete_comment(
        &context.state,
        &actor,
        &top,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete top");

    let error = crate::logic::comment::read_comment_children(&context.state, &actor, &top, 1, 10)
        .await
        .expect_err("soft-deleted parent must reject children reads");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn comment_pages_reject_a_soft_deleted_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &actor, "Dead Version", "1.0.0", "one").await;
    crate::logic::comment::create_comment(&context.state, &actor, &version_id, "orphan comment")
        .await
        .expect("comment");
    crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete version");

    let error = crate::logic::comment::read_comments(&context.state, &actor, &version_id, 1, 10)
        .await
        .expect_err("soft-deleted version must reject comment reads");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn search_pages_tile_all_matches_with_limit_two() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let mut expected: Vec<String> = Vec::new();
    for i in 0..5 {
        let (article_id, _) = create_seeded_article(
            &context,
            &actor,
            &format!("Page Match {i}"),
            "1.0.0",
            "note",
        )
        .await;
        expected.push(article_id);
    }

    let mut seen: Vec<String> = Vec::new();
    for page in 1..=3 {
        let result = crate::logic::search::search_articles(
            &context.state,
            &actor,
            &nail_common::request::ArticleSearchParams {
                q: Some("match".to_string()),
                ranges: Some(
                    "title,summary,author_name,comment,note,tag,version_number".to_string(),
                ),
                from: None,
                to: None,
                limit: Some(2),
                page: Some(page),
            },
        )
        .await
        .expect("page");
        assert_eq!(result.items.len(), if page < 3 { 2 } else { 1 });
        assert_eq!(result.has_next, page < 3);
        for item in result.items {
            seen.push(item.article_id.clone());
        }
    }
    expected.sort_unstable();
    seen.sort_unstable();
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn pagination_page_zero_is_treated_as_page_one() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &actor, "Zero Page", "1.0.0", "one").await;

    let page = crate::logic::version::read_versions(&context.state, &actor, &article_id, 0, 10)
        .await
        .expect("page zero");
    assert_eq!(page.items.len(), 1, "page 0 must behave like page 1");
}

#[tokio::test]
async fn search_page_zero_is_treated_as_page_one() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    create_seeded_article(&context, &actor, "Zero Search Page", "1.0.0", "one").await;

    let page = crate::logic::search::search_articles(
        &context.state,
        &actor,
        &nail_common::request::ArticleSearchParams {
            q: Some("zero".to_string()),
            ranges: Some("title".to_string()),
            from: None,
            to: None,
            limit: Some(10),
            page: Some(0),
        },
    )
    .await
    .expect("page zero");
    assert_eq!(page.items.len(), 1, "page 0 must behave like page 1");
}
