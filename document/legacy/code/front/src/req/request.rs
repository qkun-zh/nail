
use crate::conf::AppConfig;
use anyhow::{Context, Result};
use common::response::ResponseEnvelope;
use gloo_net::http::{Request, Response};
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Timeout;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub mod article;
pub mod auth;
pub mod comment;
pub mod multipart;
pub mod version;

pub use article::{delete_article, read_article_detail, search_articles, update_article};
pub use auth::{
    check_is_author, get_challenge, get_session, post_deregister_user,
    post_deregister_user_confirm, post_email_read, post_email_update_confirm,
    post_email_update_send, post_logout, post_user_create, read_session_token, update_user_name,
};
pub use comment::{
    create_comment_reply, create_version_comment, delete_comment, read_version_comments,
};
pub use multipart::{create_article, create_article_version};
pub use version::{download_pdf, mint_download_url, read_article_versions, read_version_detail};

pub const SESSION_TOKEN_KEY: &str = "session_token";

const REQUEST_TIMEOUT_MS: u32 = 30_000;

fn api_base_url() -> String {
    AppConfig::load().api_base_url
}

fn timeout_signal() -> Result<(web_sys::AbortSignal, Timeout)> {
    let controller = web_sys::AbortController::new()
        .map_err(|e| anyhow::anyhow!("failed to create AbortController: {e:?}"))?;
    let signal = controller.signal();
    let timer = Timeout::new(REQUEST_TIMEOUT_MS, {
        let controller = controller.clone();
        move || controller.abort()
    });
    Ok((signal, timer))
}

async fn check_response(resp: Response, authenticated: bool) -> Result<Response> {
    let status = resp.status();
    if (200..300).contains(&status) {
        return Ok(resp);
    }
    if authenticated && status == 401 {
        LocalStorage::delete(SESSION_TOKEN_KEY);
        crate::page::auth_gate::mark_session_invalid();
        return Err(anyhow::anyhow!(
            "session expired, please authenticate again"
        ));
    }
    let message = error_message(&resp).await;
    Err(anyhow::anyhow!("{message}"))
}

async fn error_message(resp: &Response) -> String {
    resp.text()
        .await
        .ok()
        .and_then(|text| {
            serde_json::from_str::<ResponseEnvelope<serde_json::Value>>(&text).ok()
        })
        .filter(|env| !env.message.trim().is_empty())
        .map(|env| env.message)
        .unwrap_or_else(|| format!("request failed with HTTP {}", resp.status()))
}

async fn unwrap_envelope<T: DeserializeOwned>(resp: Response, authenticated: bool) -> Result<T> {
    let resp = check_response(resp, authenticated).await?;
    let envelope: ResponseEnvelope<T> =
        resp.json().await.context("failed to parse response body")?;
    if (200..300).contains(&envelope.code) {
        envelope
            .data
            .ok_or_else(|| anyhow::anyhow!("empty response data"))
    } else {
        Err(anyhow::anyhow!("{}", envelope.message))
    }
}

async fn post_json<B: Serialize, R: DeserializeOwned>(path: &str, body: &B) -> Result<R> {
    let url = format!("{}/api{}", api_base_url(), path);
    let (signal, timer) = timeout_signal()?;
    let result = Request::post(&url)
        .header("Content-Type", "application/json")
        .abort_signal(Some(&signal))
        .json(body)
        .context("failed to serialize request body")?
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, false).await
}

async fn get_with_session(path: &str) -> Result<serde_json::Value> {
    let url = format!("{}/api{}", api_base_url(), path);
    let token = read_session_token()?;
    if token.is_empty() {
        return Err(anyhow::anyhow!("not logged in: authenticate first"));
    }
    let (signal, timer) = timeout_signal()?;
    let result = Request::get(&url)
        .header("Accept", "application/json")
        .header("session-token", &token)
        .abort_signal(Some(&signal))
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, true).await
}

pub fn url_encode(value: &str) -> String {
    js_sys::encode_uri_component(value)
        .as_string()
        .unwrap_or_else(|| value.to_string())
}

async fn post_json_with_token<B: Serialize, R: DeserializeOwned>(
    path: &str,
    session_token: &str,
    body: &B,
) -> Result<R> {
    let url = format!("{}/api{}", api_base_url(), path);
    let (signal, timer) = timeout_signal()?;
    let result = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("session-token", session_token)
        .abort_signal(Some(&signal))
        .json(body)
        .context("failed to serialize request body")?
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, true).await
}
