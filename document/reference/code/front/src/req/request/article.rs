
use anyhow::Result;
use common::request::{DeleteArticleRequest, UpdateArticleRequest};
use common::search::ArticleSearchParams;

use super::{get_with_session, post_json_with_token, url_encode};

pub async fn read_article_detail(article_id: &str) -> Result<serde_json::Value> {
    get_with_session(&format!(
        "/article/{}/read?check_if_is_author=true",
        url_encode(article_id)
    ))
    .await
}

pub async fn search_articles(query: &ArticleSearchParams) -> Result<serde_json::Value> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(v) = &query.q {
        pairs.push(("key_word".to_string(), url_encode(v)));
    }
    if let Some(v) = &query.ranges {
        pairs.push(("ranges".to_string(), url_encode(v)));
    }
    if let Some(v) = &query.sort {
        pairs.push(("sort".to_string(), url_encode(v)));
    }
    if let Some(v) = query.from {
        pairs.push(("from".to_string(), v.to_string()));
    }
    if let Some(v) = query.to {
        pairs.push(("to".to_string(), v.to_string()));
    }
    if let Some(v) = query.limit {
        pairs.push(("limit".to_string(), v.to_string()));
    }
    if let Some(v) = query.page {
        pairs.push(("page".to_string(), v.to_string()));
    }
    let query_string = pairs
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join("&");
    let path = if query_string.is_empty() {
        "/article/read".to_string()
    } else {
        format!("/article/read?{query_string}")
    };
    get_with_session(&path).await
}

pub async fn update_article(
    session_token: &str,
    article_id: &str,
    title: &str,
    summary: &str,
    tags_raw: &str,
) -> Result<serde_json::Value> {
    let body = UpdateArticleRequest {
        title: title.to_string(),
        summary: summary.to_string(),
        tags: tags_raw.to_string(),
    };
    post_json_with_token(
        &format!("/article/{}/update", url_encode(article_id)),
        session_token,
        &body,
    )
    .await
}

pub async fn delete_article(session_token: &str, article_id: &str) -> Result<serde_json::Value> {
    let body = DeleteArticleRequest {};
    post_json_with_token(
        &format!("/article/{}/delete", url_encode(article_id)),
        session_token,
        &body,
    )
    .await
}
