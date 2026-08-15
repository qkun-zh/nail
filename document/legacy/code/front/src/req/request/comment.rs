
use anyhow::{Context, Result};
use common::request::{CreateCommentRequest, DeleteCommentRequest};
use gloo_net::http::Request;

use super::{
    api_base_url, get_with_session, post_json_with_token, timeout_signal, unwrap_envelope,
    url_encode,
};

pub async fn read_version_comments(
    version_id: &str,
    page: u64,
    limit: u64,
) -> Result<serde_json::Value> {
    get_with_session(&format!(
        "/version/{}/comments/read?page={}&limit={}",
        url_encode(version_id),
        page,
        limit
    ))
    .await
}

pub async fn check_comment_is_author(
    session_token: &str,
    version_id: &str,
    comment_id: &str,
) -> Result<bool> {
    let url = format!(
        "{}/api/version/{}/comments/read?check_if_is_author=true",
        api_base_url(),
        url_encode(version_id)
    );
    let (signal, timer) = timeout_signal()?;
    let result = Request::get(&url)
        .header("Accept", "application/json")
        .header("session-token", session_token)
        .abort_signal(Some(&signal))
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    let data = unwrap_envelope::<serde_json::Value>(resp, true).await?;
    let found = data.get("comments").and_then(|v| v.as_array()).and_then(
        |list| {
            list.iter()
                .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(comment_id))
        },
    );
    Ok(found
        .and_then(|c| c.get("is_author").and_then(|v| v.as_bool()))
        .unwrap_or(false))
}

pub async fn create_version_comment(
    session_token: &str,
    version_id: &str,
    content: &str,
) -> Result<serde_json::Value> {
    let body = CreateCommentRequest {
        content: content.to_string(),
    };
    post_json_with_token(
        &format!("/version/{}/comments/create", url_encode(version_id)),
        session_token,
        &body,
    )
    .await
}

pub async fn delete_comment(session_token: &str, comment_id: &str) -> Result<serde_json::Value> {
    let body = DeleteCommentRequest {};
    post_json_with_token(
        &format!("/comment/{}/delete", url_encode(comment_id)),
        session_token,
        &body,
    )
    .await
}

pub async fn create_comment_reply(
    session_token: &str,
    comment_id: &str,
    content: &str,
) -> Result<serde_json::Value> {
    let body = CreateCommentRequest {
        content: content.to_string(),
    };
    post_json_with_token(
        &format!("/comments/{}/replies/create", url_encode(comment_id)),
        session_token,
        &body,
    )
    .await
}
