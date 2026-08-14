use super::context::{BrowserContext, TestBackend};
use super::smtp_sink;

async fn request_json(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (reqwest::StatusCode, serde_json::Value) {
    let mut request = client.request(method, url);
    if let Some(session) = session {
        request = request.header("session-token", session);
    }
    let response = if let Some(body) = body {
        request.json(&body).send().await.expect("send json request")
    } else {
        request.send().await.expect("send request")
    };
    let status = response.status();
    let json = response
        .json::<serde_json::Value>()
        .await
        .expect("response json");
    (status, json)
}

async fn create_article(
    backend: &TestBackend,
    session: &str,
    title: &str,
    version: &str,
    pdf: Vec<u8>,
) -> (String, String) {
    let form = reqwest::multipart::Form::new()
        .text("title", title.to_string())
        .text("summary", "e2e summary".to_string())
        .text("tags", "#e2e".to_string())
        .text("version", version.to_string())
        .text("note", "initial".to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf).file_name("seed.pdf"),
        );
    let response = backend
        .client
        .post(format!("{}/article/create", backend.base_url))
        .header("session-token", session)
        .multipart(form)
        .send()
        .await
        .expect("POST article/create");
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let json = response
        .json::<serde_json::Value>()
        .await
        .expect("article json");
    (
        json["data"]["article_id"]
            .as_str()
            .expect("article_id")
            .to_string(),
        json["data"]["version_id"]
            .as_str()
            .expect("version_id")
            .to_string(),
    )
}

async fn read_user_id(backend: &TestBackend, session: &str) -> String {
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/session/read?id=true", backend.base_url),
        Some(session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "session read: {json}");
    json["data"]["id"].as_str().expect("user id").to_string()
}

#[tokio::test]
async fn browser_authentication_flow_lands_a_session_in_local_storage() {
    let context = BrowserContext::start().await;
    let session = context.login_via_ui("alice@example.com").await;
    assert!(!session.is_empty());

    let (status, json) = request_json(
        &context.backend.client,
        reqwest::Method::GET,
        &format!("{}/session/read?id=true", context.backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "session read: {json}");
    assert!(!json["data"]["id"].as_str().unwrap_or("").is_empty());
}

#[tokio::test]
async fn account_and_content_flows_over_real_tcp() {
    let backend = TestBackend::start().await;
    let session = backend.authenticate("alice@example.com").await;
    let user_id = read_user_id(&backend, &session).await;

    let pdf = smtp_sink::unique_pdf("seed-a");
    let (article_id, version_id) =
        create_article(&backend, &session, "e2e title", "1.0.0", pdf.clone()).await;

    let response = backend
        .client
        .get(format!(
            "{}/article/{article_id}/version/{version_id}/content/read",
            backend.base_url
        ))
        .header("session-token", &session)
        .send()
        .await
        .expect("GET content/read");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("application/pdf")
    );
    let downloaded = response.bytes().await.expect("pdf bytes");
    assert_eq!(downloaded.as_ref(), pdf.as_slice());

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/article/read", backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "list: {json}");
    assert_eq!(
        json["data"]["article_list"].as_array().map(Vec::len),
        Some(1)
    );

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/article/{article_id}/read", backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "read: {json}");
    assert_eq!(json["data"]["title"].as_str(), Some("e2e title"));

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/article/{article_id}/update", backend.base_url),
        Some(&session),
        Some(serde_json::json!({
            "title": "e2e title updated",
            "summary": "updated summary",
            "tags": "#e2e #updated"
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "update: {json}");

    let form = reqwest::multipart::Form::new()
        .text("version", "2.0.0".to_string())
        .text("note", "second".to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(smtp_sink::unique_pdf("seed-b"))
                .file_name("seed-b.pdf"),
        );
    let response = backend
        .client
        .post(format!(
            "{}/article/{article_id}/version/create",
            backend.base_url
        ))
        .header("session-token", &session)
        .multipart(form)
        .send()
        .await
        .expect("POST version/create");
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let json = response
        .json::<serde_json::Value>()
        .await
        .expect("version json");
    let version_id_2 = json["data"]["version_id"]
        .as_str()
        .expect("version_id")
        .to_string();

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/article/{article_id}/version/read", backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "versions: {json}");
    assert_eq!(
        json["data"]["version_list"].as_array().map(Vec::len),
        Some(2)
    );

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!(
            "{}/version/{version_id_2}/comments/create",
            backend.base_url
        ),
        Some(&session),
        Some(serde_json::json!({ "content": "first comment" })),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::CREATED,
        "comment create: {json}"
    );
    let comment_id = json["data"]["comment_id"]
        .as_str()
        .expect("comment_id")
        .to_string();

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/version/{version_id_2}/comments/read", backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "comments: {json}");
    assert_eq!(json["data"]["comments"].as_array().map(Vec::len), Some(1));

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/comment/{comment_id}/update", backend.base_url),
        Some(&session),
        Some(serde_json::json!({ "content": "edited comment" })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "comment update: {json}");

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/comment/{comment_id}/delete", backend.base_url),
        Some(&session),
        Some(serde_json::json!({ "mode": "hard" })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "comment delete: {json}");

    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/article/{article_id}/delete", backend.base_url),
        Some(&session),
        Some(serde_json::json!({ "mode": "hard" })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "article delete: {json}");
    assert_eq!(json["message"].as_str(), Some("deleted"));

    let rename_pow = backend.server_pow("e2e-alice").await;
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/user/{user_id}/update", backend.base_url),
        Some(&session),
        Some(serde_json::json!({ "pow": rename_pow })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "rename: {json}");
    assert_eq!(json["data"]["name"].as_str(), Some("e2e-alice"));

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/email/read?intent=change_email", backend.base_url),
        Some(&session),
        Some(serde_json::json!({
            "old_email_pow": backend.server_pow("alice@example.com").await,
            "new_email_pow": backend.server_pow("alice-new@example.com").await,
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "email step1: {json}");

    let old_mail = backend.wait_for_mail("alice@example.com", 10).await;
    let new_mail = backend.wait_for_mail("alice-new@example.com", 10).await;
    let old_token = smtp_sink::extract_token(&old_mail);
    let new_token = smtp_sink::extract_token(&new_mail);
    let payload = format!(
        "{old_token}
{new_token}"
    );
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/user/{user_id}/update", backend.base_url),
        Some(&session),
        Some(serde_json::json!({
            "pow": backend.server_pow(&payload).await,
            "old_email_token": old_token,
            "new_email_token": new_token,
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "email step2: {json}");
    let new_session = json["data"]["session_token"]
        .as_str()
        .expect("session")
        .to_string();

    let (status, _) = request_json(
        &backend.client,
        reqwest::Method::GET,
        &format!("{}/session/read?id=true", backend.base_url),
        Some(&session),
        None,
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "old session must die"
    );

    let logout_pow = backend.server_pow("logout-nonce").await;
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/session/delete", backend.base_url),
        Some(&new_session),
        Some(serde_json::json!({ "pow": logout_pow })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "logout: {json}");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let session = backend.authenticate("alice-new@example.com").await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/email/read?intent=deregister", backend.base_url),
        Some(&session),
        Some(serde_json::json!({ "pow": backend.server_pow("alice-new@example.com").await })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "deregister step1: {json}");
    let mail = backend.wait_for_mail("alice-new@example.com", 10).await;
    let token = smtp_sink::extract_token(&mail);
    let (status, json) = request_json(
        &backend.client,
        reqwest::Method::POST,
        &format!("{}/user/{user_id}/delete", backend.base_url),
        Some(&session),
        Some(serde_json::json!({
            "mode": "transfer",
            "pow": backend.server_pow(&token).await,
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "deregister: {json}");
    assert_eq!(json["message"].as_str(), Some("deleted"));
}
