use nail_common::response::version::{VersionIdView, VersionListPage, VersionView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_versions(article_id: &str, page: u64, limit: u64) -> RequestResult<VersionListPage> {
    let path = url::build_path_with_query(
        &["article", article_id, "version", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn read_version(version_id: &str, article_id: &str) -> RequestResult<VersionView> {
    let path = url::build_path_with_query(
        &["version", version_id, "read"],
        &[("article_id", article_id)],
    );
    http::get_json(&path, true).await
}

pub async fn create_version(article_id: &str, form: web_sys::FormData) -> RequestResult<VersionIdView> {
    let path = url::build_path_with_query(&["article", article_id, "version", "create"], &[]);
    http::post_form(&path, form, true).await
}
