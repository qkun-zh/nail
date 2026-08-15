
use anyhow::{Context, Result};
use common::pow::{Challenge, Pow};
use common::request::{
    EmailUpdateConfirmRequest, EmailUpdateSendRequest, LogoutRequest, NameSetRequest, TokenRequest,
};
use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};

use super::{
    SESSION_TOKEN_KEY, api_base_url, post_json, post_json_with_token, timeout_signal,
    unwrap_envelope,
};

pub fn read_session_token() -> Result<String> {
    LocalStorage::get::<String>(SESSION_TOKEN_KEY)
        .map_err(|e| anyhow::anyhow!("failed to read session token from localStorage: {e}"))
}

pub async fn get_challenge() -> Result<Challenge> {
    let url = format!("{}/api/challenge/read", api_base_url());
    let (signal, timer) = timeout_signal()?;
    let result = Request::get(&url)
        .header("Accept", "application/json")
        .abort_signal(Some(&signal))
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, false).await
}

pub async fn post_email_read(pow: &Pow) -> Result<serde_json::Value> {
    let body = TokenRequest { pow: pow.clone() };
    post_json("/email/read", &body).await
}

pub async fn post_user_create(pow: &Pow) -> Result<serde_json::Value> {
    let body = TokenRequest { pow: pow.clone() };
    post_json("/user/create", &body).await
}

pub async fn get_session(
    session_token: &str,
    want_id: bool,
    want_name: bool,
) -> Result<serde_json::Value> {
    let mut query = Vec::new();
    if want_id {
        query.push("id=true".to_string());
    }
    if want_name {
        query.push("name=true".to_string());
    }
    let query_string = if query.is_empty() {
        String::new()
    } else {
        format!("?{}", query.join("&"))
    };
    let url = format!("{}/api/session/read{query_string}", api_base_url());
    let (signal, timer) = timeout_signal()?;
    let result = Request::get(&url)
        .header("Accept", "application/json")
        .header("session-token", session_token)
        .abort_signal(Some(&signal))
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, true).await
}

async fn current_user_id(session_token: &str) -> Result<String> {
    let data = get_session(session_token, true, false).await?;
    data.get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("session response missing user id"))
}

pub async fn post_logout(pow: &Pow, session_token: &str) -> Result<serde_json::Value> {
    let body = LogoutRequest { pow: pow.clone() };
    post_json_with_token("/session/delete", session_token, &body).await
}

pub async fn update_user_name(pow: &Pow) -> Result<serde_json::Value> {
    let token = read_session_token()?;
    if token.is_empty() {
        return Err(anyhow::anyhow!("not logged in: authenticate first"));
    }
    let user_id = current_user_id(&token).await?;
    let body = NameSetRequest { pow: pow.clone() };
    post_json_with_token(&format!("/user/{}/update", super::url_encode(&user_id)), &token, &body)
        .await
}

pub async fn post_email_update_send(
    old_email_pow: &Pow,
    new_email_pow: &Pow,
    session_token: &str,
) -> Result<serde_json::Value> {
    let body = EmailUpdateSendRequest {
        old_email_pow: old_email_pow.clone(),
        new_email_pow: new_email_pow.clone(),
    };
    post_json_with_token("/email/read", session_token, &body).await
}

pub async fn post_email_update_confirm(
    pow: &Pow,
    old_email_token: &str,
    new_email_token: &str,
    session_token: &str,
) -> Result<serde_json::Value> {
    let user_id = current_user_id(session_token).await?;
    let body = EmailUpdateConfirmRequest {
        pow: pow.clone(),
        old_email_token: old_email_token.to_string(),
        new_email_token: new_email_token.to_string(),
    };
    post_json_with_token(
        &format!("/user/{}/update", super::url_encode(&user_id)),
        session_token,
        &body,
    )
    .await
}

pub async fn post_deregister_user(
    email_pow: &Pow,
    session_token: &str,
) -> Result<serde_json::Value> {
    let body = TokenRequest { pow: email_pow.clone() };
    post_json_with_token("/email/read", session_token, &body).await
}

pub async fn post_deregister_user_confirm(
    pow: &Pow,
    session_token: &str,
) -> Result<serde_json::Value> {
    let user_id = current_user_id(session_token).await?;
    let body = serde_json::json!({ "mode": "transfer", "pow": pow });
    post_json_with_token(
        &format!("/user/{}/delete", super::url_encode(&user_id)),
        session_token,
        &body,
    )
    .await
}

pub async fn check_is_author(
    session_token: &str,
    article_id: Option<&str>,
    version_id: Option<&str>,
    comment_id: Option<&str>,
) -> Result<bool> {
    if let Some(article_id) = article_id.filter(|v| !v.trim().is_empty()) {
        let data = super::article::read_article_detail(article_id).await?;
        return Ok(data
            .get("is_author")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }
    if let Some(version_id) = version_id.filter(|v| !v.trim().is_empty()) {
        let data = super::version::read_version_detail(version_id, "").await?;
        return Ok(data
            .get("is_author")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }
    if let Some(comment_id) = comment_id.filter(|v| !v.trim().is_empty()) {
        let vid = version_id.unwrap_or_default();
        if vid.trim().is_empty() {
            return Ok(false);
        }
        return super::comment::check_comment_is_author(session_token, vid, comment_id).await;
    }
    Ok(false)
}
