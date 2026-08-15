use nail_common::request::ArticleSearchParams;

use super::context::{TestCtx, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

const ALL_RANGES: &str = "title,summary,author_name,comment,note,tag,version_number";

fn params(q: Option<&str>) -> ArticleSearchParams {
    ArticleSearchParams {
        q: q.map(str::to_string),
        ranges: Some(ALL_RANGES.to_string()),
        sort: None,
        from: None,
        to: None,
        limit: None,
        page: None,
    }
}

#[tokio::test]
async fn search_articles_rejects_an_unknown_range() {
    let context = TestCtx::new().await.expect("test context");
    let mut request = params(Some("rust"));
    request.ranges = Some("bogus".to_string());
    let error = crate::logic::search::search_articles(&context.state, &request)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("unknown search range: bogus")
    );
}

#[tokio::test]
async fn search_articles_rejects_from_greater_than_to() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        from: Some("2024-01-15T10:30:00Z".to_string()),
        to: Some("2024-01-15T10:00:00Z".to_string()),
        ..params(None)
    };
    let error = crate::logic::search::search_articles(&context.state, &request)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("from must not be greater than to")
    );
}

#[tokio::test]
async fn search_articles_rejects_an_overlong_query() {
    let context = TestCtx::new().await.expect("test context");
    let long = "a".repeat(513);
    let error = crate::logic::search::search_articles(&context.state, &params(Some(&long)))
        .await
        .unwrap_err();
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn search_articles_accepts_multibyte_query_within_char_limit() {
    let context = TestCtx::new().await.expect("test context");
    // 512 CJK characters = 1536 bytes, but 512 chars is within the limit.
    let multibyte_query = "中".repeat(512);
    let result =
        crate::logic::search::search_articles(&context.state, &params(Some(&multibyte_query)))
            .await;
    assert!(result.is_ok(), "512 CJK chars (512 chars) should pass");
}

#[tokio::test]
async fn search_articles_rejects_multibyte_query_over_char_limit() {
    let context = TestCtx::new().await.expect("test context");
    // 513 CJK characters = 1539 bytes, and 513 chars exceeds the limit.
    let multibyte_query = "中".repeat(513);
    let error =
        crate::logic::search::search_articles(&context.state, &params(Some(&multibyte_query)))
            .await
            .unwrap_err();
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn search_articles_returns_nothing_for_an_empty_query() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Searchable Title",
            summary: "A summary for search.",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let page = crate::logic::search::search_articles(&context.state, &params(None))
        .await
        .expect("search");
    assert_eq!(page.total, 0, "empty query must return no articles");
    assert!(page.article_list.is_empty());
}

#[tokio::test]
async fn search_filters_by_iso8601_time_range_and_renders_utc_times() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let now = nail_common::time::now_ms().expect("now");
    // Three articles with distinct timestamps: -2h, -3h, -4h from now.
    let titles = ["Recent One", "Middle One", "Old One"];
    let offsets_ms = [2, 3, 4];
    for (title, offset_hours) in titles.iter().zip(offsets_ms.iter()) {
        let article_id = nail_common::time::uuidv7_min_for_ms(now - offset_hours * 3_600_000);
        let version_id = nail_common::time::uuidv7_max_for_ms(now - offset_hours * 3_600_000);
        crate::repository::article::create_article(
            &context.state.graph,
            &crate::repository::article::ArticleDraft {
                article_id: article_id.clone(),
                author_id: actor.clone(),
                title: title.to_string(),
                summary: "summary".to_string(),
                tags: vec!["rust".to_string()],
                first_version: crate::repository::version::VersionDraft {
                    version_id,
                    version_number: "1.0.0".to_string(),
                    content_hash: format!("{offset_hours:032x}"),
                    note: "note".to_string(),
                },
            },
        )
        .await
        .expect("create");
        crate::logic::search::sync_article_best_effort(&context.state, &article_id).await;
    }

    // Filter to only the article from ~2 hours ago: from = now-2h30m, to = now-1h30m.
    let from = nail_common::time::format_rfc3339_utc(now - 150 * 60_000).expect("from");
    let to = nail_common::time::format_rfc3339_utc(now - 90 * 60_000).expect("to");
    let request = ArticleSearchParams {
        q: Some("summary".to_string()),
        from: Some(from),
        to: Some(to),
        ..params(None)
    };
    let page = crate::logic::search::search_articles(&context.state, &request)
        .await
        .expect("search");
    assert_eq!(page.total, 1, "only the recent article falls in the range");
    assert_eq!(page.article_list[0].title, "Recent One");
    // Rendering is UTC ISO8601 with a trailing Z.
    assert!(
        page.article_list[0].time.ends_with('Z'),
        "time should be UTC ISO8601: {}",
        page.article_list[0].time
    );
}

async fn seed_article(
    context: &TestCtx,
    author_id: &str,
    title: &str,
    summary: &str,
    tags: Vec<&str>,
    note: &str,
    version_ms: u64,
) {
    let article_id = nail_common::time::uuidv7_min_for_ms(version_ms);
    let version_id = nail_common::time::uuidv7_max_for_ms(version_ms);
    crate::repository::article::create_article(
        &context.state.graph,
        &crate::repository::article::ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: summary.to_string(),
            tags: tags.into_iter().map(str::to_string).collect(),
            first_version: crate::repository::version::VersionDraft {
                version_id,
                version_number: "1.0.0".to_string(),
                content_hash: format!("{version_ms:032x}"),
                note: note.to_string(),
            },
        },
    )
    .await
    .expect("create article");
    crate::logic::search::sync_article_best_effort(&context.state, &article_id).await;
}

#[tokio::test]
async fn search_combines_keyword_range_time_author_tag_and_sort() {
    let context = TestCtx::new().await.expect("test context");
    let alice = member(&context, "alice@example.com").await;
    let bob = member(&context, "bob@example.com").await;
    crate::repository::user::update_user_name(&context.state.graph, &alice, "alice-smith")
        .await
        .expect("alice name");
    crate::repository::user::update_user_name(&context.state.graph, &bob, "bob-jones")
        .await
        .expect("bob name");
    let now = nail_common::time::now_ms().expect("now");
    // A: newest, alice, tags rust+database, note mentions pipeline
    seed_article(
        &context,
        &alice,
        "Quantum Index",
        "Solar store notes",
        vec!["rust", "database", "shared"],
        "neural pipeline design",
        now - 2 * 3_600_000,
    )
    .await;
    // B: middle, bob, tags web+api, summary mentions pipeline
    seed_article(
        &context,
        &bob,
        "Lunar Cache",
        "Quantum pipeline analysis",
        vec!["web", "api", "shared"],
        "bob note",
        now - 5 * 3_600_000,
    )
    .await;
    // C: oldest, alice, tags rust+search, title mentions store
    seed_article(
        &context,
        &alice,
        "Neural Store",
        "Atomic journal",
        vec!["rust", "search", "shared"],
        "lunar filter",
        now - 10 * 3_600_000,
    )
    .await;

    // 1. keyword matches title field across authors
    let page = crate::logic::search::search_articles(&context.state, &params(Some("quantum")))
        .await
        .expect("quantum");
    assert_eq!(page.total, 2, "quantum hits A(title) and B(summary)");

    // 2. ranges=title narrows to title-only hits
    let mut title_only = params(Some("quantum"));
    title_only.ranges = Some("title".to_string());
    let page = crate::logic::search::search_articles(&context.state, &title_only)
        .await
        .expect("quantum title");
    assert_eq!(page.total, 1, "title-only search finds only A");
    assert_eq!(page.article_list[0].title, "<mark>Quantum</mark> Index");

    // 3. tag search: rust tag matches A and C
    let page = crate::logic::search::search_articles(&context.state, &params(Some("rust")))
        .await
        .expect("rust tag");
    assert_eq!(page.total, 2, "rust tag hits A and C");

    // 4. author search by user name
    let page = crate::logic::search::search_articles(&context.state, &params(Some("alice-smith")))
        .await
        .expect("alice author");
    assert_eq!(page.total, 2, "alice authored A and C");

    // 5. note field: pipeline appears in A(note) and B(summary), not C
    let page = crate::logic::search::search_articles(&context.state, &params(Some("pipeline")))
        .await
        .expect("pipeline");
    assert_eq!(page.total, 2, "pipeline hits A(note) and B(summary)");

    // 6. time range with only from (to empty): keep A and B, drop C
    let from = nail_common::time::format_rfc3339_utc(now - 7 * 3_600_000).expect("from");
    let page = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            q: Some("shared".to_string()),
            from: Some(from),
            ..params(None)
        },
    )
    .await
    .expect("from only");
    assert_eq!(page.total, 2, "from-only keeps A and B");

    // 7. time range with only to (from empty): keep A, B, C (all before now)
    let to = nail_common::time::format_rfc3339_utc(now - 60_000).expect("to");
    let page = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            q: Some("shared".to_string()),
            to: Some(to),
            ..params(None)
        },
    )
    .await
    .expect("to only");
    assert_eq!(page.total, 3, "to-only keeps everything before now");

    // 8. keyword + time range combined: quantum within last 6h = A and B (C is older)
    let from6 = nail_common::time::format_rfc3339_utc(now - 6 * 3_600_000).expect("from6");
    let page = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            q: Some("quantum".to_string()),
            from: Some(from6.clone()),
            ..params(None)
        },
    )
    .await
    .expect("quantum+from");
    assert_eq!(page.total, 2, "quantum within 6h is A and B");

    // 9. keyword + closed time window [now-6h, now-3h] keeps only B
    let to3 = nail_common::time::format_rfc3339_utc(now - 3 * 3_600_000).expect("to3");
    let page = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            q: Some("quantum".to_string()),
            from: Some(from6),
            to: Some(to3),
            ..params(None)
        },
    )
    .await
    .expect("quantum+window");
    assert_eq!(page.total, 1, "quantum in [now-6h, now-3h] is B only");
    assert_eq!(page.article_list[0].title, "Lunar Cache");
}

#[tokio::test]
async fn search_sorts_by_time_title_and_author_and_paginates() {
    let context = TestCtx::new().await.expect("test context");
    let alice = member(&context, "alice@example.com").await;
    let bob = member(&context, "bob@example.com").await;
    crate::repository::user::update_user_name(&context.state.graph, &alice, "alice-smith")
        .await
        .expect("alice name");
    crate::repository::user::update_user_name(&context.state.graph, &bob, "bob-jones")
        .await
        .expect("bob name");
    let now = nail_common::time::now_ms().expect("now");
    seed_article(
        &context,
        &alice,
        "Banana",
        "zzz",
        vec!["rust"],
        "n1",
        now - 2 * 3_600_000,
    )
    .await;
    seed_article(
        &context,
        &bob,
        "Apple",
        "yyy",
        vec!["rust"],
        "n2",
        now - 5 * 3_600_000,
    )
    .await;
    seed_article(
        &context,
        &alice,
        "Cherry",
        "xxx",
        vec!["rust"],
        "n3",
        now - 10 * 3_600_000,
    )
    .await;

    // time desc: newest first
    let mut sort_time = params(Some("rust"));
    sort_time.sort = Some("time:desc".to_string());
    let page = crate::logic::search::search_articles(&context.state, &sort_time)
        .await
        .expect("time desc");
    let titles: Vec<&str> = page.article_list.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, vec!["Banana", "Apple", "Cherry"], "time desc");

    // time asc: oldest first
    let mut sort_time_asc = params(Some("rust"));
    sort_time_asc.sort = Some("time:asc".to_string());
    let page = crate::logic::search::search_articles(&context.state, &sort_time_asc)
        .await
        .expect("time asc");
    let titles: Vec<&str> = page.article_list.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, vec!["Cherry", "Apple", "Banana"], "time asc");

    // title asc
    let mut sort_title = params(Some("rust"));
    sort_title.sort = Some("title:asc".to_string());
    let page = crate::logic::search::search_articles(&context.state, &sort_title)
        .await
        .expect("title asc");
    let titles: Vec<&str> = page.article_list.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, vec!["Apple", "Banana", "Cherry"], "title asc");

    // author asc: alice articles then bob
    let mut sort_author = params(Some("rust"));
    sort_author.sort = Some("author:asc".to_string());
    let page = crate::logic::search::search_articles(&context.state, &sort_author)
        .await
        .expect("author asc");
    let authors: Vec<&str> = page
        .article_list
        .iter()
        .map(|a| a.author_name.as_str())
        .collect();
    assert_eq!(
        authors,
        vec!["alice-smith", "alice-smith", "bob-jones"],
        "author asc"
    );

    // pagination: limit 2 page 1 gives 2 rows and has_next
    let page = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            limit: Some(2),
            page: Some(1),
            ..params(Some("rust"))
        },
    )
    .await
    .expect("page 1");
    assert_eq!(page.article_list.len(), 2);
    assert!(page.has_next);
    assert_eq!(page.total, 3);
    let page2 = crate::logic::search::search_articles(
        &context.state,
        &ArticleSearchParams {
            limit: Some(2),
            page: Some(2),
            ..params(Some("rust"))
        },
    )
    .await
    .expect("page 2");
    assert_eq!(page2.article_list.len(), 1);
    assert!(!page2.has_next);
}

#[tokio::test]
async fn bare_tag_search_matches_tag_field() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let now = nail_common::time::now_ms().expect("now");
    // Tags are stored without any '#' prefix now.
    seed_article(
        &context,
        &actor,
        "Hash Tag Probe",
        "probe summary",
        vec!["rust"],
        "probe note",
        now - 3_600_000,
    )
    .await;

    let page = crate::logic::search::search_articles(&context.state, &params(Some("rust")))
        .await
        .expect("search rust");
    assert!(page.total >= 1, "rust should find the article via its tag");
    let tag_hit = page.article_list[0]
        .article_hits
        .iter()
        .find(|hit| hit.label == "tag")
        .expect("tag hit present");
    // No '#' anywhere in the rendered tag snippet.
    assert_eq!(
        tag_hit.snippet, "[\"<mark>rust</mark>\"]",
        "bare tag snippet"
    );
}

#[tokio::test]
async fn single_char_query_reports_field_hits() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Sample 9 for scheduler",
            summary: "Releases for version 9.",
            tags: "rust",
            version: "9.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let page = crate::logic::search::search_articles(&context.state, &params(Some("9")))
        .await
        .expect("search 9");
    assert!(page.total >= 1, "single-char query must find the article");
    let item = page
        .article_list
        .iter()
        .find(|item| item.title == "Sample 9 for scheduler")
        .expect("article present");
    assert!(
        item.article_hits.iter().any(|hit| hit.label == "summary"),
        "single-char query must report a summary hit, got: {:?}",
        item.article_hits
    );
    assert!(
        item.title.contains('9'),
        "title must render the raw value for a single-char query: {:?}",
        item.title
    );
}

#[tokio::test]
async fn search_articles_returns_nothing_for_empty_ranges() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Searchable Title",
            summary: "A summary for search.",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let request = ArticleSearchParams {
        ranges: Some(String::new()),
        ..params(Some("searchable"))
    };
    let page = crate::logic::search::search_articles(&context.state, &request)
        .await
        .expect("search");
    assert_eq!(page.total, 0, "empty ranges must return no articles");
    assert!(page.article_list.is_empty());
}

#[tokio::test]
async fn keyword_that_misses_tags_does_not_report_a_tag_hit() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let now = nail_common::time::now_ms().expect("now");
    seed_article(
        &context,
        &actor,
        "Unique Title",
        "probe summary",
        vec!["rust"],
        "probe note",
        now - 3_600_000,
    )
    .await;

    let page = crate::logic::search::search_articles(&context.state, &params(Some("unique")))
        .await
        .expect("search unique");
    assert!(
        page.total >= 1,
        "unique should find the article via its title"
    );
    let item = &page.article_list[0];
    assert_eq!(item.title, "<mark>Unique</mark> Title");
    assert!(
        !item.article_hits.iter().any(|hit| hit.label == "tag"),
        "keyword that misses the tag must not report a tag hit, got: {:?}",
        item.article_hits
    );
}
