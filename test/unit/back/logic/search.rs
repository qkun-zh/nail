use nail_common::search::ArticleSearchParams;

use super::context::{TestCtx, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

fn params(q: Option<&str>) -> ArticleSearchParams {
    ArticleSearchParams {
        q: q.map(str::to_string),
        ranges: None,
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
    assert_eq!(error, LogicError::bad_request("unknown search range: bogus"));
}

#[tokio::test]
async fn search_articles_rejects_from_greater_than_to() {
    let context = TestCtx::new().await.expect("test context");
    let request = ArticleSearchParams {
        from: Some(200),
        to: Some(100),
        ..params(None)
    };
    let error = crate::logic::search::search_articles(&context.state, &request)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("from must not be greater than to"));
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
async fn search_articles_returns_articles_for_an_empty_query() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        "Searchable Title",
        "A summary for search.",
        "#rust",
        "1.0.0",
        "note",
        context.upload(&valid_pdf()),
    )
    .await
    .expect("create");

    let page = crate::logic::search::search_articles(&context.state, &params(None))
        .await
        .expect("search");
    assert!(page.total >= 1);
    assert!(page.article_list.iter().any(|item| item.title == "Searchable Title"));
    assert_eq!(page.page, 1);
    assert!(!page.has_prev);
}
