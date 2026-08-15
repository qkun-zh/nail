
use anyhow::{Context, Result};
use gloo_net::http::Request;
use wasm_bindgen::JsCast;

use super::auth::read_session_token;
use super::{
    api_base_url, check_response, get_with_session, timeout_signal, unwrap_envelope, url_encode,
};

pub async fn read_article_versions(article_id: &str, page: u64) -> Result<serde_json::Value> {
    get_with_session(&format!(
        "/article/{}/version/read?page={}",
        url_encode(article_id),
        page
    ))
    .await
}

pub async fn read_version_detail(version_id: &str, article_id: &str) -> Result<serde_json::Value> {
    get_with_session(&format!(
        "/version/{}/read?article_id={}",
        url_encode(version_id),
        url_encode(article_id)
    ))
    .await
}

pub async fn mint_download_url(
    session_token: &str,
    article_id: &str,
    version_id: &str,
) -> Result<String> {
    let url = format!(
        "{}/api/article/{}/version/{}/content/read?download=1",
        api_base_url(),
        url_encode(article_id),
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
    data.get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("download mint response missing url"))
}

fn same_origin(absolute_url: &str, base: &str) -> bool {
    if base.is_empty() {
        return absolute_url.starts_with('/') && !absolute_url.starts_with("//");
    }
    let Ok(parsed) = web_sys::Url::new_with_base(absolute_url, base) else {
        return false;
    };
    let Ok(base) = web_sys::Url::new(base) else {
        return false;
    };
    parsed.origin() == base.origin()
}

pub async fn download_pdf(absolute_url: &str) -> Result<(), String> {
    if !same_origin(absolute_url, &api_base_url()) {
        return Err("refusing to send session token to a foreign origin".to_string());
    }
    let token = read_session_token().map_err(|e| e.to_string())?;
    if token.is_empty() {
        return Err("authenticate to download".to_string());
    }
    let (signal, timer) = timeout_signal().map_err(|e| format!("{e}"))?;
    let result = gloo_net::http::Request::get(absolute_url)
        .header("session-token", &token)
        .abort_signal(Some(&signal))
        .send()
        .await;
    timer.cancel();
    let resp = result.map_err(|e| e.to_string())?;
    let resp = check_response(resp, true)
        .await
        .map_err(|e| format!("{e}"))?;
    let bytes = resp.binary().await.map_err(|e| e.to_string())?;
    let filename = resp
        .headers()
        .get("content-disposition")
        .and_then(|value| {
            value
                .split(';')
                .find_map(|part| {
                    let part = part.trim();
                    let name = part.strip_prefix("filename=")?;
                    Some(name.trim_matches('"').to_string())
                })
                .filter(|name| !name.is_empty() && !name.contains('/'))
        })
        .unwrap_or_else(|| "article.pdf".to_string());
    let array = js_sys::Uint8Array::from(&bytes[..]);
    let blob = web_sys::Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&array))
        .map_err(|e| format!("blob creation failed: {e:?}"))?;
    let object_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("object url failed: {e:?}"))?;
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let anchor = document
        .create_element("a")
        .map_err(|e| format!("create anchor failed: {e:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "anchor cast failed".to_string())?;
    anchor.set_href(&object_url);
    anchor.set_download(&filename);
    anchor.click();
    gloo_timers::callback::Timeout::new(0, move || {
        web_sys::Url::revoke_object_url(&object_url).ok();
    })
    .forget();
    Ok(())
}
