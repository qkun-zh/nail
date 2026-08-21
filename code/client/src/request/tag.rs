use common::request::{CreateTagRequest, DeleteMode, TagUpdateRequest};
use common::response::EmptyView;
use common::response::ListPage;
pub use common::response::NamedRef;
pub use common::response::tag::TagListItem;

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_tags(
    page: Option<u64>,
    limit: Option<u64>,
) -> RequestResult<ListPage<TagListItem>> {
    let pow = prove_pow().await?;
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
    let path = url::build_path_with_query(&["tags"], &query);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_tag(tag_id: &str) -> RequestResult<NamedRef> {
    let pow = prove_pow().await?;
    let tag_id = validate_id(tag_id, "tag_id")?;
    let path = url::build_path_with_query(&["tags", &tag_id], &[]);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn create_tag(name: &str) -> RequestResult<NamedRef> {
    let pow = prove_pow().await?;
    let path = url::build_path_with_query(&["tags"], &[]);
    let body = CreateTagRequest {
        name: name.to_string(),
    };
    http::post_json(&path, &body, true, Some(&pow)).await
}

pub async fn update_tag(tag_id: &str, name: &str) -> RequestResult<NamedRef> {
    let pow = prove_pow().await?;
    let tag_id = validate_id(tag_id, "tag_id")?;
    let path = url::build_path_with_query(&["tags", &tag_id], &[]);
    let body = TagUpdateRequest {
        name: Some(name.to_string()),
    };
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn delete_tag(tag_id: &str) -> RequestResult<()> {
    let pow = prove_pow().await?;
    let tag_id = validate_id(tag_id, "tag_id")?;
    let path = url::build_path_with_query(
        &["tags", &tag_id],
        &[(
            "mode",
            &serde_json::to_string(&DeleteMode::Hard).unwrap_or_default(),
        )],
    );
    http::delete_json(&path, true, Some(&pow)).await
}

pub async fn apply_tag(article_id: &str, tag_id: &str) -> RequestResult<EmptyView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let tag_id = validate_id(tag_id, "tag_id")?;
    let path = url::build_path_with_query(&["articles", &article_id, "tags", &tag_id], &[]);
    http::put_json(&path, &(), true, Some(&pow)).await
}

pub async fn unapply_tag(article_id: &str, tag_id: &str) -> RequestResult<EmptyView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let tag_id = validate_id(tag_id, "tag_id")?;
    let path = url::build_path_with_query(&["articles", &article_id, "tags", &tag_id], &[]);
    http::delete_json(&path, true, Some(&pow)).await
}
