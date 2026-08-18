use nail_common::request::{CreateCommentRequest, DeleteBody, DeleteMode};
use nail_common::response::comment::{CommentIdView, CommentListPage, CommentView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_comments(
    version_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<CommentListPage> {
    let path = url::build_path_with_query(
        &["version", version_id, "comments", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn read_comment(comment_id: &str) -> RequestResult<CommentView> {
    let path = url::build_path_with_query(&["comment", comment_id, "read"], &[]);
    http::get_json(&path, true).await
}

pub async fn read_comment_children(
    parent_id: &str,
    page: u64,
    limit: u64,
) -> RequestResult<CommentListPage> {
    let path = url::build_path_with_query(
        &["comment", parent_id, "replies", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn create_comment(version_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let path = url::build_path_with_query(&["version", version_id, "comments", "create"], &[]);
    http::post_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
    )
    .await
}

pub async fn create_reply(parent_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let path = url::build_path_with_query(&["comments", parent_id, "replies", "create"], &[]);
    http::post_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
    )
    .await
}

pub async fn delete_comment(comment_id: &str, mode: DeleteMode) -> RequestResult<CommentIdView> {
    let path = url::build_path_with_query(&["comment", comment_id, "delete"], &[]);
    http::post_json(&path, &DeleteBody { mode: Some(mode) }, true).await
}

pub async fn update_comment(comment_id: &str, content: &str) -> RequestResult<CommentIdView> {
    let path = url::build_path_with_query(&["comment", comment_id, "update"], &[]);
    http::post_json(
        &path,
        &CreateCommentRequest {
            content: content.to_string(),
        },
        true,
    )
    .await
}

pub async fn undelete_soft_comment(comment_id: &str) -> RequestResult<CommentIdView> {
    let path = url::build_path_with_query(&["comment", comment_id, "undelete-soft"], &[]);
    http::post_json(&path, &(), true).await
}
