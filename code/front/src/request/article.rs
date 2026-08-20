use nail_common::request::{DeleteMode, UpdateArticleRequest};
use nail_common::response::ListPage;
use nail_common::response::article::{ArticleIdView, ArticleView, CreateArticleView};
use nail_common::response::search::SearchArticleItem;

use crate::request::error::RequestResult;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn search_articles(query: &[(&str, &str)]) -> RequestResult<ListPage<SearchArticleItem>> {
    let path = url::build_path_with_query(&["articles"], query);
    http::get_json(&path, true).await
}

pub async fn read_article(article_id: &str) -> RequestResult<ArticleView> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id], &[]);
    http::get_json(&path, true).await
}

pub async fn create_article(form: web_sys::FormData) -> RequestResult<CreateArticleView> {
    http::post_form("/articles", form, true).await
}

pub async fn update_article(
    article_id: &str,
    title: &str,
    summary: &str,
    tags: &str,
) -> RequestResult<ArticleIdView> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id], &[]);
    let body = UpdateArticleRequest {
        title: title.to_string(),
        summary: summary.to_string(),
        tags: tags.to_string(),
    };
    http::patch_json(&path, &body, true).await
}

pub async fn delete_article(article_id: &str, mode: DeleteMode) -> RequestResult<ArticleIdView> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(
        &["articles", &article_id],
        &[("mode", &serde_json::to_string(&mode).unwrap_or_default())],
    );
    http::delete_json(&path, true).await
}

pub async fn undelete_soft_article(article_id: &str) -> RequestResult<ArticleIdView> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id, "restore"], &[]);
    http::post_json(&path, &(), true).await
}
