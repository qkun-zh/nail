use nail_common::request::ArticleSearchParams;

use super::context::{TestCtx, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops"];

async fn member(context: &TestCtx, email: &str) -> String {
    context.seed_tags(TEST_TAGS).await;
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

async fn admin(context: &TestCtx) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &context.state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn plain(context: &TestCtx, email: &str) -> String {
    crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
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

#[tokio::test]
async fn search_articles_denies_a_user_without_the_grant() {
    let context = TestCtx::new().await.expect("test context");
    let outsider = plain(&context, "stranger@example.com").await;

    let error = crate::logic::search::search_articles(&context.state, &outsider, &params(None))
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn search_articles_rejects_an_unknown_range() {
    let context = TestCtx::new().await.expect("test context");
    let mut request = params(Some("rust"));
    request.ranges = Some("bogus".to_string());
    let error =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
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
    let error =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("from must not be greater than to")
    );
}

#[tokio::test]
async fn search_articles_rejects_an_invalid_from_bound() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        from: Some("not-a-datetime".to_string()),
        ..params(None)
    };
    let error =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request(
            "from must be an ISO8601 datetime (year to second precision, no timezone means UTC)"
        )
    );
}

#[tokio::test]
async fn search_articles_accepts_an_empty_from_bound() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        from: Some("   ".to_string()),
        to: Some("   ".to_string()),
        ..params(None)
    };
    let result =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn search_articles_rejects_an_overlong_query() {
    let context = TestCtx::new().await.expect("test context");
    let long = "a".repeat(513);
    let error = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some(&long)),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn search_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        page: Some(1025),
        ..params(Some("rust"))
    };
    let error =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("page exceeds max search pages")
    );
}

#[tokio::test]
async fn search_allows_a_page_at_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        page: Some(1024),
        ..params(Some("rust"))
    };
    let page =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .expect("page at the limit is allowed");
    assert!(page.article_list.is_empty());
}

#[tokio::test]
async fn search_articles_accepts_multibyte_query_within_char_limit() {
    let context = TestCtx::new().await.expect("test context");
    let multibyte_query = "中".repeat(512);
    let result = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some(&multibyte_query)),
    )
    .await;
    assert!(result.is_ok(), "512 CJK chars (512 chars) should pass");
}

#[tokio::test]
async fn search_articles_rejects_multibyte_query_over_char_limit() {
    let context = TestCtx::new().await.expect("test context");
    let multibyte_query = "中".repeat(513);
    let error = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some(&multibyte_query)),
    )
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(None),
    )
    .await
    .expect("search");
    assert_eq!(
        page.article_list.len() as u64,
        0,
        "empty query must return no articles"
    );
    assert!(page.article_list.is_empty());
}

#[tokio::test]
async fn search_filters_by_iso8601_time_range_and_renders_utc_times() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let now = nail_common::time::now_ms().expect("now");
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

    let from = nail_common::time::format_rfc3339_utc(now - 150 * 60_000).expect("from");
    let to = nail_common::time::format_rfc3339_utc(now - 90 * 60_000).expect("to");
    let request = ArticleSearchParams {
        q: Some("summary".to_string()),
        from: Some(from),
        to: Some(to),
        ..params(None)
    };
    let page =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .expect("search");
    assert_eq!(
        page.article_list.len() as u64,
        1,
        "only the recent article falls in the range"
    );
    assert_eq!(page.article_list[0].title, "Recent One");
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
async fn search_combines_keyword_range_time_author_tag() {
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
        "Quantum Index",
        "Solar store notes",
        vec!["rust", "database", "shared"],
        "neural pipeline design",
        now - 2 * 3_600_000,
    )
    .await;
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("quantum")),
    )
    .await
    .expect("quantum");
    assert_eq!(
        page.article_list.len() as u64,
        2,
        "quantum hits A(title) and B(summary)"
    );

    let mut title_only = params(Some("quantum"));
    title_only.ranges = Some("title".to_string());
    let page =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &title_only)
            .await
            .expect("quantum title");
    assert_eq!(
        page.article_list.len() as u64,
        1,
        "title-only search finds only A"
    );
    assert_eq!(page.article_list[0].title, "<mark>Quantum</mark> Index");

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("rust")),
    )
    .await
    .expect("rust tag");
    assert_eq!(page.article_list.len() as u64, 2, "rust tag hits A and C");

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("alice-smith")),
    )
    .await
    .expect("alice author");
    assert_eq!(page.article_list.len() as u64, 2, "alice authored A and C");

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("pipeline")),
    )
    .await
    .expect("pipeline");
    assert_eq!(
        page.article_list.len() as u64,
        2,
        "pipeline hits A(note) and B(summary)"
    );

    let from = nail_common::time::format_rfc3339_utc(now - 7 * 3_600_000).expect("from");
    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &ArticleSearchParams {
            q: Some("shared".to_string()),
            from: Some(from),
            ..params(None)
        },
    )
    .await
    .expect("from only");
    assert_eq!(page.article_list.len() as u64, 2, "from-only keeps A and B");

    let to = nail_common::time::format_rfc3339_utc(now - 60_000).expect("to");
    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &ArticleSearchParams {
            q: Some("shared".to_string()),
            to: Some(to),
            ..params(None)
        },
    )
    .await
    .expect("to only");
    assert_eq!(
        page.article_list.len() as u64,
        3,
        "to-only keeps everything before now"
    );

    let from6 = nail_common::time::format_rfc3339_utc(now - 6 * 3_600_000).expect("from6");
    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &ArticleSearchParams {
            q: Some("quantum".to_string()),
            from: Some(from6.clone()),
            ..params(None)
        },
    )
    .await
    .expect("quantum+from");
    assert_eq!(
        page.article_list.len() as u64,
        2,
        "quantum within 6h is A and B"
    );

    let to3 = nail_common::time::format_rfc3339_utc(now - 3 * 3_600_000).expect("to3");
    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &ArticleSearchParams {
            q: Some("quantum".to_string()),
            from: Some(from6),
            to: Some(to3),
            ..params(None)
        },
    )
    .await
    .expect("quantum+window");
    assert_eq!(
        page.article_list.len() as u64,
        1,
        "quantum in [now-6h, now-3h] is B only"
    );
    assert_eq!(page.article_list[0].title, "Lunar Cache");
}

#[tokio::test]
async fn search_paginates_with_limit_and_page() {
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
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
    let page2 = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("rust")),
    )
    .await
    .expect("search rust");
    assert!(
        page.article_list.len() as u64 >= 1,
        "rust should find the article via its tag"
    );
    let tag_hit = page.article_list[0]
        .article_hits
        .iter()
        .find(|hit| hit.label == "tag")
        .expect("tag hit present");
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("9")),
    )
    .await
    .expect("search 9");
    assert!(
        page.article_list.len() as u64 >= 1,
        "single-char query must find the article"
    );
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
    let page =
        crate::logic::search::search_articles(&context.state, &admin(&context).await, &request)
            .await
            .expect("search");
    assert_eq!(
        page.article_list.len() as u64,
        0,
        "empty ranges must return no articles"
    );
    assert!(page.article_list.is_empty());
}

#[tokio::test]
async fn space_separated_keywords_match_any_field_or() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let now = nail_common::time::now_ms().expect("now");
    seed_article(
        &context,
        &actor,
        "Alpha",
        "zzz",
        vec!["rust"],
        "note",
        now - 3_600_000,
    )
    .await;
    seed_article(
        &context,
        &actor,
        "Beta",
        "zzz",
        vec!["rust"],
        "note",
        now - 2 * 3_600_000,
    )
    .await;
    seed_article(
        &context,
        &actor,
        "Alpha Beta",
        "zzz",
        vec!["rust"],
        "note",
        now - 5 * 3_600_000,
    )
    .await;

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("alpha beta")),
    )
    .await
    .expect("alpha beta or");
    assert_eq!(page.article_list.len() as u64, 3, "space must OR the terms");
    let mut titles: Vec<String> = page
        .article_list
        .iter()
        .map(|a| a.title.replace("<mark>", "").replace("</mark>", ""))
        .collect();
    titles.sort_unstable();
    assert_eq!(
        titles,
        vec![
            "Alpha".to_string(),
            "Alpha Beta".to_string(),
            "Beta".to_string()
        ],
        "space OR matches every article holding either term"
    );

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("+alpha +beta")),
    )
    .await
    .expect("+alpha +beta and");
    assert_eq!(
        page.article_list.len() as u64,
        1,
        "leading '+' on both terms must AND"
    );
    assert_eq!(
        page.article_list[0]
            .title
            .replace("<mark>", "")
            .replace("</mark>", ""),
        "Alpha Beta"
    );

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("alpha +beta")),
    )
    .await
    .expect("alpha +beta");
    assert_eq!(
        page.article_list.len() as u64,
        2,
        "required beta matches its two articles"
    );
    let mut titles: Vec<String> = page
        .article_list
        .iter()
        .map(|a| a.title.replace("<mark>", "").replace("</mark>", ""))
        .collect();
    titles.sort_unstable();
    assert_eq!(titles, vec!["Alpha Beta".to_string(), "Beta".to_string()]);

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("alpha -beta")),
    )
    .await
    .expect("alpha -beta");
    assert_eq!(
        page.article_list.len() as u64,
        1,
        "alpha -beta must exclude Beta and Alpha Beta"
    );
    assert_eq!(
        page.article_list[0]
            .title
            .replace("<mark>", "")
            .replace("</mark>", ""),
        "Alpha"
    );
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

    let page = crate::logic::search::search_articles(
        &context.state,
        &admin(&context).await,
        &params(Some("unique")),
    )
    .await
    .expect("search unique");
    assert!(
        page.article_list.len() as u64 >= 1,
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
