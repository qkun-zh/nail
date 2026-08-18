use nail_common::request::{CreateTagRequest, TagUpdateRequest};
pub use nail_common::response::tag::{TagListPage, TagNameView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_tags(page: Option<u64>, limit: Option<u64>) -> RequestResult<TagListPage> {
    let mut query: Vec<(&str, &str)> = Vec::new();
    let page_str;
    let limit_str;
    if let Some(p) = page {
        page_str = p.to_string();
        query.push(("page", &page_str));
    }
    if let Some(l) = limit {
        limit_str = l.to_string();
        query.push(("limit", &limit_str));
    }
    let path = url::build_path_with_query(&["tag", "list"], &query);
    http::get_json(&path, true).await
}

pub async fn read_tag(tag_id: &str) -> RequestResult<TagNameView> {
    let path = url::build_path_with_query(&["tag", tag_id, "read"], &[]);
    http::get_json(&path, true).await
}

pub async fn create_tag(name: &str) -> RequestResult<TagNameView> {
    let path = url::build_path_with_query(&["tag", "create"], &[]);
    let body = CreateTagRequest {
        name: name.to_string(),
    };
    http::post_json(&path, &body, true).await
}

pub async fn update_tag(tag_id: &str, name: &str) -> RequestResult<TagNameView> {
    let path = url::build_path_with_query(&["tag", tag_id, "update"], &[]);
    let body = TagUpdateRequest {
        name: Some(name.to_string()),
    };
    http::post_json(&path, &body, true).await
}

pub async fn delete_tag(tag_id: &str) -> RequestResult<()> {
    let path = url::build_path_with_query(&["tag", tag_id, "delete"], &[]);
    http::post_json(&path, &(), true).await
}
