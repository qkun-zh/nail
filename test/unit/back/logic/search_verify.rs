use nail_common::request::ArticleSearchParams;

use super::context::{TestCtx, unique_pdf, valid_pdf};
use crate::repository::role::{ROLE_ADMIN, ROLE_MEMBER, hold_role};

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops", "web"];

fn member(context: &TestCtx, email: &str) -> String {
    context.seed_tags(TEST_TAGS);
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_MEMBER).expect("member role");
    user_id
}

fn admin(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_ADMIN).expect("admin role");
    user_id
}

const ALL_RANGES: &str = "title,summary,author_name,comment,note,tag,version_number";

fn params(q: Option<&str>) -> ArticleSearchParams {
    ArticleSearchParams {
        q: q.map(str::to_string),
        ranges: Some(ALL_RANGES.to_string()),
        from: None,
        to: None,
        limit: None,
        page: None,
    }
}

async fn create_seeded_article(
    context: &TestCtx,
    actor_id: &str,
    title: &str,
    summary: &str,
    tags: &str,
    version: &str,
    note: &str,
) -> (String, String) {
    crate::logic::article::create_article(
        &context.state,
        actor_id,
        crate::logic::article::ArticleCreateInput {
            title,
            summary,
            tags,
            version,
            note,
            upload: context.upload(&unique_pdf(title)),
        },
    )
    .await
    .expect("create article")
}

fn strip_marks(text: &str) -> String {
    text.replace("<mark>", "").replace("</mark>", "")
}

#[tokio::test]
async fn search_hides_a_soft_deleted_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Softly Hidden Needle",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("needle")))
            .await
            .expect("before delete");
    assert_eq!(page.items.len(), 1, "indexed before delete");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("needle")))
            .await
            .expect("after delete");
    assert_eq!(
        page.items.len(),
        0,
        "soft-deleted article hidden from search (subtree hidden)"
    );
}

#[tokio::test]
async fn search_hides_the_versions_of_a_soft_deleted_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Versioned Public Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("1.0.0")))
            .await
            .expect("search version number");
    assert_eq!(
        page.items.len(),
        0,
        "the versions of a soft-deleted article are hidden from search"
    );
}

#[tokio::test]
async fn search_hides_the_comments_of_a_soft_deleted_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (_, version_id) = create_seeded_article(
        &context,
        &actor,
        "Commented Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::comment::create_comment(
        &context.state,
        &actor,
        &version_id,
        "a very distinct comment phrase",
    )
    .await
    .expect("comment");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id_of(&context, &version_id),
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("distinct")))
            .await
            .expect("search comment");
    assert_eq!(
        page.items.len(),
        0,
        "the comment of a soft-deleted article is hidden from search"
    );
}

fn article_id_of(context: &TestCtx, version_id: &str) -> String {
    crate::repository::version::parent_article_of(&context.state.database, version_id)
        .expect("parent")
        .expect("article")
}

#[tokio::test]
async fn search_hides_a_soft_deleted_version_but_keeps_siblings() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Dual Version Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "second note",
        context.upload(&unique_pdf("second-version")),
    )
    .await
    .expect("second version");

    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .expect("versions");
    let doomed = versions
        .iter()
        .find(|item| item.version_number == "1.0.0")
        .expect("first version")
        .id
        .clone();
    crate::logic::version::delete_version(
        &context.state,
        &actor,
        &doomed,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete version");

    let page = crate::logic::search::search_articles(&context.state, &actor, &params(Some("note")))
        .await
        .expect("search note");
    let item = page
        .items
        .iter()
        .find(|item| strip_marks(&item.title) == "Dual Version Title")
        .expect("article remains");
    assert_eq!(
        item.versions.len(),
        1,
        "only the live sibling version card remains"
    );
    assert_eq!(
        strip_marks(&item.versions[0].version_number),
        "2.0.0",
        "the survivor card is the sibling"
    );
}

#[tokio::test]
async fn search_hides_a_soft_deleted_version_and_its_comments() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (_, version_id) = create_seeded_article(
        &context,
        &actor,
        "Versioned Comment Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::comment::create_comment(
        &context.state,
        &actor,
        &version_id,
        "persistent comment marker",
    )
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

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("marker")))
            .await
            .expect("search marker");
    assert_eq!(
        page.items.len(),
        0,
        "comment doc of a soft-deleted version is hidden"
    );
    let page = crate::logic::search::search_articles(&context.state, &actor, &params(Some("note")))
        .await
        .expect("search note");
    assert!(
        page.items
            .iter()
            .all(|item| strip_marks(&item.title) != "Versioned Comment Title"),
        "the soft-deleted version's own doc must be gone"
    );
}

#[tokio::test]
async fn search_hides_a_soft_deleted_comment_and_its_replies() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (_, version_id) = create_seeded_article(
        &context,
        &actor,
        "Comment Tree Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    let top = crate::logic::comment::create_comment(
        &context.state,
        &actor,
        &version_id,
        "deleted top marker",
    )
    .await
    .expect("top");
    crate::logic::comment::create_reply(&context.state, &actor, &top, "kept reply marker")
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

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("deleted")))
            .await
            .expect("search deleted top");
    assert!(
        !page
            .items
            .iter()
            .any(|item| item.versions.iter().any(|version| {
                version
                    .comments
                    .iter()
                    .any(|comment| strip_marks(&comment.content).contains("deleted top marker"))
            })),
        "soft-deleted comment doc is gone"
    );
    let page = crate::logic::search::search_articles(&context.state, &actor, &params(Some("kept")))
        .await
        .expect("search kept reply");
    assert_eq!(
        page.items.len(),
        0,
        "reply doc of a soft-deleted parent is hidden"
    );
}

#[tokio::test]
async fn search_removes_a_hard_deleted_article() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com");
    let admin_id = admin(&context, "admin@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &owner,
        "Hard Removed Needle",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        &article_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete");

    let page =
        crate::logic::search::search_articles(&context.state, &owner, &params(Some("needle")))
            .await
            .expect("search");
    assert!(page.items.is_empty(), "hard delete clears the docs");
}

#[tokio::test]
async fn search_order_is_stable_across_repeated_queries() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    for title in ["Stable One", "Stable Two", "Stable Three"] {
        create_seeded_article(&context, &actor, title, "summary", "rust", "1.0.0", "note").await;
    }

    let first =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("stable")))
            .await
            .expect("first query");
    let second =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("stable")))
            .await
            .expect("second query");
    let ids = |page: &nail_common::response::ListPage<
        nail_common::response::search::SearchArticleItem,
    >|
     -> Vec<String> {
        page.items
            .iter()
            .map(|item| item.article_id.clone())
            .collect()
    };
    assert_eq!(ids(&first), ids(&second), "identical query must not drift");
    assert_eq!(first.items.len(), 3);
}

#[tokio::test]
async fn search_matches_keywords_case_insensitively() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "CaseProbe Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("CASEPROBE")))
            .await
            .expect("uppercase query");
    assert!(
        page.items
            .iter()
            .any(|item| strip_marks(&item.title) == "CaseProbe Title"),
        "uppercase query must match lowercase stored title"
    );
}

#[tokio::test]
async fn search_limits_results_to_a_single_range() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Range Title Word",
        "summary range word",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    create_seeded_article(
        &context,
        &actor,
        "Second Title",
        "summary range word",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let mut title_only = params(Some("range"));
    title_only.ranges = Some("title".to_string());
    let page = crate::logic::search::search_articles(&context.state, &actor, &title_only)
        .await
        .expect("title only");
    assert!(
        page.items
            .iter()
            .any(|item| strip_marks(&item.title) == "Range Title Word"),
        "the word in the title must hit the title range"
    );
    assert!(
        !page
            .items
            .iter()
            .any(|item| strip_marks(&item.title) == "Second Title"),
        "the same word in a summary must NOT hit the title range"
    );
}

#[tokio::test]
async fn search_summary_range_only_matches_the_summary_field() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Unique Summary Phrase",
        "summary phrase marker",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let mut summary_only = params(Some("marker"));
    summary_only.ranges = Some("summary".to_string());
    let page = crate::logic::search::search_articles(&context.state, &actor, &summary_only)
        .await
        .expect("summary only");
    assert_eq!(page.items.len(), 1, "summary-only hit");
    assert!(
        page.items[0]
            .article_hits
            .iter()
            .any(|hit| hit.label == "summary"),
        "hit label must be summary, got {:?}",
        page.items[0].article_hits
    );
}

#[tokio::test]
async fn search_note_range_only_matches_the_note_field() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Note Phrase Title",
        "summary",
        "rust",
        "1.0.0",
        "note marker phrase",
    )
    .await;

    let mut note_only = params(Some("marker"));
    note_only.ranges = Some("note".to_string());
    let page = crate::logic::search::search_articles(&context.state, &actor, &note_only)
        .await
        .expect("note only");
    assert_eq!(page.items.len(), 1, "note-only hit");
}

#[tokio::test]
async fn search_author_range_matches_the_author_name() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    crate::repository::user::update_user_name(&context.state.database, &actor, "probe-author")
        .expect("rename");
    create_seeded_article(
        &context,
        &actor,
        "Authored Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let mut author_only = params(Some("probe-author"));
    author_only.ranges = Some("author_name".to_string());
    let page = crate::logic::search::search_articles(&context.state, &actor, &author_only)
        .await
        .expect("author only");
    assert_eq!(page.items.len(), 1, "author-only hit");
}

#[tokio::test]
async fn search_author_name_refreshes_after_a_rename() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Renamed Author Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::repository::user::update_user_name(&context.state.database, &actor, "new-author-name")
        .expect("rename");
    crate::logic::search::sync_user_best_effort(&context.state, &actor).await;

    let page = crate::logic::search::search_articles(
        &context.state,
        &actor,
        &params(Some("new-author-name")),
    )
    .await
    .expect("new name");
    assert_eq!(page.items.len(), 1, "renamed author must be findable");
    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("renamed")))
            .await
            .expect("old name");
    assert_eq!(
        strip_marks(&page.items[0].author_name),
        "new-author-name",
        "stale author name must not persist"
    );
}

#[tokio::test]
async fn search_time_range_is_inclusive_at_the_boundaries() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let now = nail_common::time::now_ms().expect("now");
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Boundary Article",
            summary: "summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");
    let from = nail_common::time::format_rfc3339_utc(now).expect("from");
    let to = nail_common::time::format_rfc3339_utc(now + 60_000).expect("to");
    let request = ArticleSearchParams {
        q: Some("boundary".to_string()),
        ranges: Some(ALL_RANGES.to_string()),
        from: Some(from),
        to: Some(to),
        limit: None,
        page: None,
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("from==article time");
    assert_eq!(
        page.items.len(),
        1,
        "article at the exact from boundary must be included"
    );
}

#[tokio::test]
async fn search_excludes_articles_outside_the_time_range() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let now = nail_common::time::now_ms().expect("now");
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Modern Article",
            summary: "summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let old_from = nail_common::time::format_rfc3339_utc(now - 3_600_000).expect("from");
    let old_to = nail_common::time::format_rfc3339_utc(now - 1_800_000).expect("to");
    let request = ArticleSearchParams {
        q: Some("modern".to_string()),
        ranges: Some(ALL_RANGES.to_string()),
        from: Some(old_from),
        to: Some(old_to),
        limit: None,
        page: None,
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("old range");
    assert!(
        page.items.is_empty(),
        "article created now must not match an old time window"
    );
}

#[tokio::test]
async fn search_has_next_flips_only_after_the_last_page() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    for title in ["Next One", "Next Two", "Next Three"] {
        create_seeded_article(&context, &actor, title, "summary", "rust", "1.0.0", "note").await;
    }

    let request = ArticleSearchParams {
        q: Some("next".to_string()),
        ranges: Some(ALL_RANGES.to_string()),
        from: None,
        to: None,
        limit: Some(1),
        page: Some(1),
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("page 1");
    assert_eq!(page.items.len(), 1);
    assert!(page.has_next, "page 1 of 3 must have next");

    let request = ArticleSearchParams {
        page: Some(2),
        ..request.clone()
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("page 2");
    assert_eq!(page.items.len(), 1);
    assert!(page.has_next, "page 2 of 3 must have next");

    let request = ArticleSearchParams {
        page: Some(3),
        ..request.clone()
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("page 3");
    assert_eq!(page.items.len(), 1);
    assert!(!page.has_next, "page 3 of 3 must not have next");
}

#[tokio::test]
async fn search_pages_do_not_duplicate_or_skip_articles() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let mut expected: Vec<String> = Vec::new();
    for i in 0..5 {
        let (article_id, _) = create_seeded_article(
            &context,
            &actor,
            &format!("Tiled Article {i}"),
            "summary",
            "rust",
            "1.0.0",
            "note",
        )
        .await;
        expected.push(article_id);
    }

    let mut seen: Vec<String> = Vec::new();
    for page_number in 1..=5 {
        let request = ArticleSearchParams {
            q: Some("tiled".to_string()),
            ranges: Some(ALL_RANGES.to_string()),
            from: None,
            to: None,
            limit: Some(1),
            page: Some(page_number),
        };
        let page = crate::logic::search::search_articles(&context.state, &actor, &request)
            .await
            .expect("page");
        for item in &page.items {
            seen.push(item.article_id.clone());
        }
    }
    expected.sort_unstable();
    seen.sort_unstable();
    assert_eq!(
        seen, expected,
        "five single-item pages must tile all five articles exactly once"
    );
}

#[tokio::test]
async fn search_page_beyond_the_result_set_is_empty() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Single Hit Article",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let request = ArticleSearchParams {
        q: Some("single".to_string()),
        ranges: Some(ALL_RANGES.to_string()),
        from: None,
        to: None,
        limit: Some(10),
        page: Some(9),
    };
    let page = crate::logic::search::search_articles(&context.state, &actor, &request)
        .await
        .expect("far page");
    assert!(page.items.is_empty());
    assert!(!page.has_next, "far page must not promise more");
}

#[tokio::test]
async fn search_empty_index_returns_no_results() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let _ = actor;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("anything")))
            .await
            .expect("search on empty index");
    assert!(page.items.is_empty());
    assert!(!page.has_next);
}

#[tokio::test]
async fn search_summary_hit_reports_a_summary_label_but_title_does_not() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Multi Field Marker",
        "summary marker phrase",
        "rust",
        "1.0.0",
        "note marker phrase",
    )
    .await;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("marker")))
            .await
            .expect("search marker");
    assert_eq!(page.items.len(), 1);
    let labels: Vec<String> = page.items[0]
        .article_hits
        .iter()
        .map(|hit| hit.label.clone())
        .collect();
    assert!(
        labels.iter().any(|label| label == "summary"),
        "summary hit missing: {labels:?}"
    );
    let version_labels: Vec<String> = page.items[0]
        .versions
        .iter()
        .flat_map(|version| version.version_hits.iter().map(|hit| hit.label.clone()))
        .collect();
    assert!(
        version_labels.iter().any(|label| label == "note"),
        "note hit missing: {version_labels:?}"
    );
}

#[tokio::test]
async fn search_version_number_hit_shows_the_version_card() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Version Card Probe",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("1.0.0")))
            .await
            .expect("search version number");
    assert_eq!(page.items.len(), 1);
    let item = &page.items[0];
    assert_eq!(strip_marks(&item.title), "Version Card Probe");
    assert_eq!(item.versions.len(), 1);
    assert_eq!(strip_marks(&item.versions[0].version_number), "1.0.0");
}

#[tokio::test]
async fn search_reports_nothing_for_a_word_in_no_field() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    create_seeded_article(
        &context,
        &actor,
        "Present Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    let page = crate::logic::search::search_articles(
        &context.state,
        &actor,
        &params(Some("absent-phrase")),
    )
    .await
    .expect("search absent");
    assert!(page.items.is_empty(), "absent word must find nothing");
}

#[tokio::test]
async fn search_after_hard_delete_of_one_article_keeps_the_others() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com");
    let admin_id = admin(&context, "admin@example.com");
    let (first_id, _) = create_seeded_article(
        &context,
        &owner,
        "Keep One Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    let _ = create_seeded_article(
        &context,
        &owner,
        "Keep Two Title",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        &first_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete first");

    let page = crate::logic::search::search_articles(&context.state, &owner, &params(Some("keep")))
        .await
        .expect("search keep");
    assert_eq!(
        page.items.len(),
        1,
        "hard delete must remove only its own docs"
    );
}

#[tokio::test]
async fn search_version_number_range_finds_versions() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Numbered Article",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "note",
        context.upload(&unique_pdf("v2")),
    )
    .await
    .expect("create v2");

    let mut version_only = params(Some("2.0.0"));
    version_only.ranges = Some("version_number".to_string());
    let page = crate::logic::search::search_articles(&context.state, &actor, &version_only)
        .await
        .expect("version number search");
    assert_eq!(page.items.len(), 1, "version number hit");
    assert!(
        page.items[0]
            .versions
            .iter()
            .any(|version| strip_marks(&version.version_number) == "2.0.0"),
        "the matching version card must be listed"
    );
}

#[tokio::test]
async fn search_after_clear_flag_and_resync_revives_the_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Revived Article",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    crate::repository::delete::clear_soft_deleted_flag(&context.state.database, &article_id)
        .expect("clear flag");
    crate::logic::search::sync_article_best_effort(&context.state, &article_id).await;

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("revived")))
            .await
            .expect("search revived");
    assert_eq!(
        page.items.len(),
        1,
        "cleared flag + resync must bring the article back"
    );
}

#[tokio::test]
async fn search_a_comment_only_phrase_lists_it_under_its_article_and_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (_, version_id) = create_seeded_article(
        &context,
        &actor,
        "Ordinary Title",
        "ordinary summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::comment::create_comment(
        &context.state,
        &actor,
        &version_id,
        "the zephyr keyword lives only here",
    )
    .await
    .expect("comment");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("zephyr")))
            .await
            .expect("search zephyr");
    assert_eq!(page.items.len(), 1, "comment match lists its article");
    let article = &page.items[0];
    assert_eq!(
        strip_marks(&article.title),
        "Ordinary Title",
        "the parent article is surfaced for a comment-only match"
    );
    assert_eq!(article.versions.len(), 1);
    let version = &article.versions[0];
    assert_eq!(strip_marks(&version.version_number), "1.0.0");
    assert_eq!(version.comments.len(), 1, "the matching comment is listed");
    assert_eq!(
        strip_marks(&version.comments[0].content),
        "the zephyr keyword lives only here"
    );
}

#[tokio::test]
async fn search_reports_an_article_level_hit_once_across_versions() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com");
    let (article_id, _) = create_seeded_article(
        &context,
        &actor,
        "Multi Version Marker",
        "shared marker phrase",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "note",
        context.upload(&unique_pdf("v2")),
    )
    .await
    .expect("create version");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("marker")))
            .await
            .expect("search marker");
    let item = page
        .items
        .iter()
        .find(|item| strip_marks(&item.title) == "Multi Version Marker")
        .expect("article present");
    let summary_hits = item
        .article_hits
        .iter()
        .filter(|hit| hit.label == "summary")
        .count();
    assert_eq!(
        summary_hits, 1,
        "an article-level summary hit must appear once even with 2 versions, got {:?}",
        item.article_hits
    );
}

#[tokio::test]
async fn search_comment_only_article_keeps_its_author_id() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "bob@example.com");
    let (_, version_id) = create_seeded_article(
        &context,
        &actor,
        "Commented Only",
        "summary",
        "rust",
        "1.0.0",
        "note",
    )
    .await;
    crate::logic::comment::create_comment(
        &context.state,
        &actor,
        &version_id,
        "the zephyr marker lives only in a comment",
    )
    .await
    .expect("comment");

    let page =
        crate::logic::search::search_articles(&context.state, &actor, &params(Some("zephyr")))
            .await
            .expect("search zephyr");
    assert_eq!(page.items.len(), 1, "comment match lists its article");
    let article = &page.items[0];
    assert_eq!(
        article.author_id, actor,
        "an article surfaced via a comment must keep its author id so the author link is valid"
    );
}
