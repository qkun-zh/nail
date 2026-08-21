use common::request::{CreateCommentRequest, DeleteMode};
use common::response::ListPage;
use common::response::comment::{CommentIdView, CommentView};

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_comments(
    version_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<ListPage<CommentView>> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(
        &["versions", &version_id, "comments"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_comment(comment_id: &str) -> RequestResult<CommentView> {
    let pow = prove_pow().await?;
    let comment_id = validate_id(comment_id, "comment_id")?;
    let path = url::build_path_with_query(&["comments", &comment_id], &[]);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_comment_children(
    parent_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<ListPage<CommentView>> {
    let pow = prove_pow().await?;
    let parent_id = validate_id(parent_id, "parent_id")?;
    let path = url::build_path_with_query(
        &["comments", &parent_id, "replies"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn create_comment(version_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let pow = prove_pow().await?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = url::build_path_with_query(&["versions", &version_id, "comments"], &[]);
    http::post_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
        Some(&pow),
    )
    .await
}

pub async fn create_reply(parent_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let pow = prove_pow().await?;
    let parent_id = validate_id(parent_id, "parent_id")?;
    let path = url::build_path_with_query(&["comments", &parent_id, "replies"], &[]);
    http::post_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
        Some(&pow),
    )
    .await
}

pub async fn delete_comment(comment_id: &str, mode: DeleteMode) -> RequestResult<CommentIdView> {
    let pow = prove_pow().await?;
    let comment_id = validate_id(comment_id, "comment_id")?;
    let path = url::build_path_with_query(
        &["comments", &comment_id],
        &[("mode", &serde_json::to_string(&mode).unwrap_or_default())],
    );
    http::delete_json(&path, true, Some(&pow)).await
}

pub async fn update_comment(comment_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let pow = prove_pow().await?;
    let comment_id = validate_id(comment_id, "comment_id")?;
    let path = url::build_path_with_query(&["comments", &comment_id], &[]);
    http::patch_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
        Some(&pow),
    )
    .await
}

pub async fn undelete_soft_comment(comment_id: &str) -> RequestResult<CommentIdView> {
    let pow = prove_pow().await?;
    let comment_id = validate_id(comment_id, "comment_id")?;
    let path = url::build_path_with_query(&["comments", &comment_id, "restore"], &[]);
    http::post_json(&path, &(), true, Some(&pow)).await
}
