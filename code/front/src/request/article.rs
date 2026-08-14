use nail_common::request::{DeleteBody, DeleteMode, UpdateArticleRequest};
use nail_common::response::article::{
    ArticleIdView, ArticleListPage, ArticleView, CreateArticleView,
};
use nail_common::response::search::SearchPage;

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_articles(page: u64, limit: u64) -> RequestResult<ArticleListPage> {
    let path = url::build_path_with_query(
        &["article", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn search_articles(query: &[(&str, &str)]) -> RequestResult<SearchPage> {
    let path = url::build_path_with_query(&["article", "read"], query);
    http::get_json(&path, true).await
}

pub async fn read_article(article_id: &str, check_author: bool) -> RequestResult<ArticleView> {
    let mut query = Vec::new();
    if check_author {
        query.push(("check_if_is_author", "true"));
    }
    let path = url::build_path_with_query(&["article", article_id, "read"], &query);
    http::get_json(&path, true).await
}

pub async fn create_article(form: web_sys::FormData) -> RequestResult<CreateArticleView> {
    http::post_form("/article/create", form, true).await
}

pub async fn update_article(
    article_id: &str,
    title: &str,
    summary: &str,
    tags: &str,
) -> RequestResult<ArticleIdView> {
    let path = url::build_path_with_query(&["article", article_id, "update"], &[]);
    let body = UpdateArticleRequest {
        title: title.to_string(),
        summary: summary.to_string(),
        tags: tags.to_string(),
    };
    http::post_json(&path, &body, true).await
}

pub async fn delete_article(article_id: &str, mode: DeleteMode) -> RequestResult<ArticleIdView> {
    let path = url::build_path_with_query(&["article", article_id, "delete"], &[]);
    http::post_json(&path, &DeleteBody { mode: Some(mode) }, true).await
}
