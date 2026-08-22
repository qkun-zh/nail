use common::request::DeleteMode;
use common::response::ListPage;
use common::response::version::{VersionIdView, VersionListItem, VersionView};

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_versions(
    article_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<ListPage<VersionListItem>> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(
        &["articles", &article_id, "versions"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_version(version_id: &str, article_id: &str) -> RequestResult<VersionView> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let article_id = validate_id(article_id, "article_id")?;
    let path =
        url::build_path_with_query(&["versions", &version_id], &[("article_id", &article_id)]);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn create_version(
    article_id: &str,
    form: web_sys::FormData,
) -> RequestResult<VersionIdView> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let path = url::build_path_with_query(&["articles", &article_id, "versions"], &[]);
    http::post_form(&path, form, true, Some(&pow)).await
}

pub async fn update_version(version_id: &str, note: &str) -> RequestResult<VersionIdView> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["versions", &version_id], &[]);
    let body = serde_json::json!({ "note": note });
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn delete_version(version_id: &str, mode: DeleteMode) -> RequestResult<VersionIdView> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["versions", &version_id], &[("mode", mode.as_str())]);
    http::delete_json(&path, true, Some(&pow)).await
}

pub async fn undelete_soft_version(version_id: &str) -> RequestResult<VersionIdView> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["versions", &version_id, "restore"], &[]);
    http::post_json(&path, &(), true, Some(&pow)).await
}
