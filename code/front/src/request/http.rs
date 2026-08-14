use gloo_net::http::{Request, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::infrastructure::config::api_base_url;
use crate::request::envelope::{is_success, parse_envelope, unwrap_envelope};
use crate::request::error::{RequestError, RequestResult};
use crate::request::session::{
    clear_session_token, notify_session_invalid, read_session_token, should_invalidate_session,
};

pub const REQUEST_TIMEOUT_MS: u32 = 30_000;

pub fn build_absolute_url(path_and_query: &str) -> String {
    format!("{}/api{}", api_base_url(), path_and_query)
}

pub fn handle_unauthorized_status(status: u16, authenticated: bool) {
    if should_invalidate_session(status, authenticated) {
        clear_session_token();
        notify_session_invalid();
    }
}

pub async fn read_error_message(response: &Response) -> String {
    let status = response.status();
    if let Ok(text) = response.text().await
        && let Ok(envelope) = serde_json::from_str::<
            nail_common::response::ResponseEnvelope<serde_json::Value>,
        >(&text)
        && !envelope.message.trim().is_empty()
    {
        return envelope.message;
    }
    format!("request failed with HTTP {status}")
}

fn timeout_signal() -> RequestResult<(web_sys::AbortSignal, gloo_timers::callback::Timeout)> {
    let controller = web_sys::AbortController::new().map_err(|error| {
        RequestError::network(format!("failed to create AbortController: {error:?}"))
    })?;
    let signal = controller.signal();
    let timer = gloo_timers::callback::Timeout::new(REQUEST_TIMEOUT_MS, move || controller.abort());
    Ok((signal, timer))
}

fn session_header() -> RequestResult<Option<String>> {
    match read_session_token() {
        Some(token) if !token.is_empty() => Ok(Some(token)),
        _ => Err(RequestError::status(401, "authenticate to continue")),
    }
}

async fn unwrap_json<T: DeserializeOwned>(
    response: Response,
    authenticated: bool,
) -> RequestResult<T> {
    let status = response.status();
    if !is_success(status) {
        handle_unauthorized_status(status, authenticated);
        let message = read_error_message(&response).await;
        return Err(RequestError::status(status, message));
    }
    let text = response
        .text()
        .await
        .map_err(|error| RequestError::network(format!("failed to read response body: {error}")))?;
    let envelope = parse_envelope(&text)?;
    if !is_success(envelope.code) {
        handle_unauthorized_status(envelope.code, authenticated);
    }
    unwrap_envelope(envelope)
}

pub async fn get_json<T: DeserializeOwned>(
    path_and_query: &str,
    authenticated: bool,
) -> RequestResult<T> {
    let (signal, timer) = timeout_signal()?;
    let mut request = Request::get(&build_absolute_url(path_and_query))
        .header("Accept", "application/json")
        .abort_signal(Some(&signal));
    if authenticated && let Some(token) = session_header()? {
        request = request.header("session-token", &token);
    }
    let result = request.send().await;
    timer.cancel();
    let response =
        result.map_err(|error| RequestError::network(format!("request failed: {error}")))?;
    unwrap_json(response, authenticated).await
}

pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path_and_query: &str,
    body: &B,
    authenticated: bool,
) -> RequestResult<T> {
    let (signal, timer) = timeout_signal()?;
    let mut request = Request::post(&build_absolute_url(path_and_query))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .abort_signal(Some(&signal));
    if authenticated && let Some(token) = session_header()? {
        request = request.header("session-token", &token);
    }
    let request = request.json(body).map_err(|error| {
        RequestError::network(format!("failed to serialize request body: {error}"))
    })?;
    let result = request.send().await;
    timer.cancel();
    let response =
        result.map_err(|error| RequestError::network(format!("request failed: {error}")))?;
    unwrap_json(response, authenticated).await
}

pub async fn post_form<T: DeserializeOwned>(
    path_and_query: &str,
    form: web_sys::FormData,
    authenticated: bool,
) -> RequestResult<T> {
    let (signal, timer) = timeout_signal()?;
    let mut request = Request::post(&build_absolute_url(path_and_query))
        .header("Accept", "application/json")
        .abort_signal(Some(&signal));
    if authenticated && let Some(token) = session_header()? {
        request = request.header("session-token", &token);
    }
    let request = request.body(form).map_err(|error| {
        RequestError::network(format!("failed to build multipart request: {error}"))
    })?;
    let result = request.send().await;
    timer.cancel();
    let response =
        result.map_err(|error| RequestError::network(format!("request failed: {error}")))?;
    unwrap_json(response, authenticated).await
}
