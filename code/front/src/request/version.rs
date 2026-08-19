use nail_common::request::{DeleteBody, DeleteMode};
use nail_common::response::ListPage;
use nail_common::response::version::{VersionIdView, VersionListItem, VersionView};

use crate::request::error::RequestResult;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_versions(
    article_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<ListPage<VersionListItem>> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(
        &["article", &article_id, "version", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn read_version(version_id: &str, article_id: &str) -> RequestResult<VersionView> {
    let version_id = validate_id(version_id, "version_id")?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(
        &["version", &version_id, "read"],
        &[("article_id", &article_id)],
    );
    http::get_json(&path, true).await
}

pub async fn create_version(
    article_id: &str,
    form: web_sys::FormData,
) -> RequestResult<VersionIdView> {
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["article", &article_id, "version", "create"], &[]);
    http::post_form(&path, form, true).await
}

pub async fn update_version(version_id: &str, note: &str) -> RequestResult<VersionIdView> {
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["version", &version_id, "update"], &[]);
    let body = serde_json::json!({ "note": note });
    http::post_json(&path, &body, true).await
}

pub async fn delete_version(version_id: &str, mode: DeleteMode) -> RequestResult<VersionIdView> {
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["version", &version_id, "delete"], &[]);
    http::post_json(&path, &DeleteBody { mode: Some(mode) }, true).await
}

pub async fn undelete_soft_version(version_id: &str) -> RequestResult<VersionIdView> {
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["version", &version_id, "undelete-soft"], &[]);
    http::post_json(&path, &(), true).await
}
