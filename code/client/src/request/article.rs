use common::request::{DeleteMode, UpdateArticleRequest};
use common::response::ListPage;
use common::response::article::{ArticleIdView, ArticleView, CreateArticleView};
use common::response::search::SearchArticleItem;

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn search_articles(query: &[(&str, &str)]) -> RequestResult<ListPage<SearchArticleItem>> {
    let pow = prove_pow().await?;
    let path = url::build_path_with_query(&["articles"], query);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_article(article_id: &str) -> RequestResult<ArticleView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id], &[]);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn create_article(form: web_sys::FormData) -> RequestResult<CreateArticleView> {
    let pow = prove_pow().await?;
    http::post_form("/articles", form, true, Some(&pow)).await
}

pub async fn update_article(
    article_id: &str,
    title: &str,
    summary: &str,
    tags: &str,
) -> RequestResult<ArticleIdView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id], &[]);
    let body = UpdateArticleRequest {
        title: title.to_string(),
        summary: summary.to_string(),
        tags: tags.to_string(),
    };
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn delete_article(article_id: &str, mode: DeleteMode) -> RequestResult<ArticleIdView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(
        &["articles", &article_id],
        &[("mode", &serde_json::to_string(&mode).unwrap_or_default())],
    );
    http::delete_json(&path, true, Some(&pow)).await
}

pub async fn undelete_soft_article(article_id: &str) -> RequestResult<ArticleIdView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id, "restore"], &[]);
    http::post_json(&path, &(), true, Some(&pow)).await
}
