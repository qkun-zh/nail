use wasm_bindgen::JsCast;

use crate::request::error::RequestResult;
use crate::request::http;
use crate::request::pow::prove_pow;
use crate::request::session::read_session_token;
use crate::request::validate::validate_id;

const FALLBACK_FILENAME: &str = "article.pdf";

pub fn origin_of(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let scheme = &url[..scheme_end];
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let rest = &url[scheme_end + 3..];
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

pub fn resolve_download_url(minted: &str, window_origin: &str) -> Option<String> {
    if minted.starts_with('/') {
        if minted.starts_with("//") {
            return None;
        }
        return Some(format!("{window_origin}{minted}"));
    }
    let origin = origin_of(minted)?;
    if origin == window_origin {
        Some(minted.to_string())
    } else {
        None
    }
}

pub fn filename_from_content_disposition(header: Option<&str>) -> String {
    let Some(header) = header else {
        return FALLBACK_FILENAME.to_string();
    };
    let Some(filename) = header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix("filename=")
            .map(|name| name.trim_matches('"').to_string())
    }) else {
        return FALLBACK_FILENAME.to_string();
    };
    if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
        return FALLBACK_FILENAME.to_string();
    }
    filename
}

pub async fn mint_download_url(article_id: &str, version_id: &str) -> RequestResult<String> {
    let pow = prove_pow().await?;
    let article_id = validate_id(article_id, "article_id")?;
    let version_id = validate_id(version_id, "version_id")?;
    let path = crate::request::url::build_path_with_query(
        &["articles", &article_id, "versions", &version_id, "content"],
        &[("mode", "download")],
    );
    let mint: common::response::content::MintUrl = http::get_json(&path, true, Some(&pow)).await?;
    Ok(mint.url)
}

pub async fn download_pdf(minted_url: &str) -> Result<(), String> {
    let pow = crate::request::pow::prove_pow()
        .await
        .map_err(|error| error.to_string())?;
    let window_origin = web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .ok_or_else(|| "no window origin available".to_string())?;
    let absolute_url = resolve_download_url(minted_url, &window_origin)
        .ok_or_else(|| "refusing to send the session token to a foreign origin".to_string())?;
    let token = read_session_token().ok_or_else(|| "authenticate to download".to_string())?;

    let controller = web_sys::AbortController::new()
        .map_err(|error| format!("failed to create AbortController: {error:?}"))?;
    let signal = controller.signal();
    let timer =
        gloo_timers::callback::Timeout::new(http::REQUEST_TIMEOUT_MS, move || controller.abort());

    let pow_json =
        serde_json::to_string(&pow).map_err(|error| format!("failed to serialize pow: {error}"))?;
    let response = gloo_net::http::Request::get(&absolute_url)
        .header("session-token", &token)
        .header("x-pow", &pow_json)
        .abort_signal(Some(&signal))
        .send()
        .await
        .map_err(|error| format!("download failed: {error}"));
    timer.cancel();
    let response = response?;

    let status = response.status();
    if !crate::request::envelope::is_success(status) {
        http::handle_unauthorized_status(status, true);
        let message = http::read_error_message(&response).await;
        return Err(message);
    }

    let bytes = response
        .binary()
        .await
        .map_err(|error| format!("failed to read download bytes: {error}"))?;
    let filename =
        filename_from_content_disposition(response.headers().get("content-disposition").as_deref());
    save_blob(&bytes, &filename)
}

fn save_blob(bytes: &[u8], filename: &str) -> Result<(), String> {
    let array = js_sys::Uint8Array::from(bytes);
    let blob = web_sys::Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&array))
        .map_err(|error| format!("blob creation failed: {error:?}"))?;
    let object_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|error| format!("object url failed: {error:?}"))?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| "no document available".to_string())?;
    let anchor = document
        .create_element("a")
        .map_err(|error| format!("create anchor failed: {error:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "anchor cast failed".to_string())?;
    anchor.set_href(&object_url);
    anchor.set_download(filename);
    anchor.click();
    let object_url_for_revoke = object_url.clone();
    gloo_timers::callback::Timeout::new(0, move || {
        let _ = web_sys::Url::revoke_object_url(&object_url_for_revoke);
    })
    .forget();
    Ok(())
}

#[cfg(test)]
#[path = "../../../../test/unit/client/request/download/tests.rs"]
mod tests;
