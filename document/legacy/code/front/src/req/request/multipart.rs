
use anyhow::{Context, Result};
use gloo_net::http::Request;

use super::{api_base_url, timeout_signal, unwrap_envelope, url_encode};

pub async fn create_article(
    session_token: &str,
    title: &str,
    summary: &str,
    tags_raw: &str,
    version: &str,
    note: &str,
    file: web_sys::File,
) -> Result<serde_json::Value> {
    let form = web_sys::FormData::new()
        .map_err(|e| anyhow::anyhow!("failed to create FormData: {e:?}"))?;
    form.append_with_str("title", title)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `title`: {e:?}"))?;
    form.append_with_str("summary", summary)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `summary`: {e:?}"))?;
    form.append_with_str("tags", tags_raw)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `tags`: {e:?}"))?;
    form.append_with_str("version", version)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `version`: {e:?}"))?;
    form.append_with_str("note", note)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `note`: {e:?}"))?;
    form.append_with_blob("file", &file)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `file`: {e:?}"))?;

    let url = format!("{}/api/article/create", api_base_url());
    let (signal, timer) = timeout_signal()?;
    let result = Request::post(&url)
        .header("session-token", session_token)
        .abort_signal(Some(&signal))
        .body(form)
        .context("failed to build multipart request")?
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, true).await
}

pub async fn create_article_version(
    session_token: &str,
    article_id: &str,
    version: &str,
    note: &str,
    file: web_sys::File,
) -> Result<serde_json::Value> {
    let form = web_sys::FormData::new()
        .map_err(|e| anyhow::anyhow!("failed to create FormData: {e:?}"))?;
    form.append_with_str("version", version)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `version`: {e:?}"))?;
    form.append_with_str("note", note)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `note`: {e:?}"))?;
    form.append_with_blob("file", &file)
        .map_err(|e| anyhow::anyhow!("failed to append FormData field `file`: {e:?}"))?;

    let url = format!(
        "{}/api/article/{}/version/create",
        api_base_url(),
        url_encode(article_id)
    );
    let (signal, timer) = timeout_signal()?;
    let result = Request::post(&url)
        .header("session-token", session_token)
        .abort_signal(Some(&signal))
        .body(form)
        .context("failed to build multipart request")?
        .send()
        .await;
    timer.cancel();
    let resp = result.context("failed to send request")?;
    unwrap_envelope(resp, true).await
}
