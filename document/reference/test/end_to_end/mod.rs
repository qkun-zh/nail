
pub mod browser;
pub mod http;

#[cfg(test)]
mod tests {
    use super::extract_token;
    use super::http::context::EndToEndHttpContext;
    use super::unit_pdf;

    #[tokio::test]
    async fn authentication_mail_chain_over_real_tcp() {
        let ctx = EndToEndHttpContext::start().await;

        let email = "end_to_end@qq.com";
        let (_subject, _body) = ctx.submit_email_authentication(email).await;

        let mail_body = ctx.wait_for_mail(email, 5).await;
        assert!(
            !mail_body.is_empty(),
            "captured mail body must contain the token"
        );
        {
            let inbox = ctx.inbox.lock().expect("inbox lock");
            let mail = inbox
                .iter()
                .find(|m| m.to.eq_ignore_ascii_case(email))
                .expect("captured mail present");
            assert!(
                uuid::Uuid::parse_str(&mail.subject).is_ok(),
                "subject must be a UUID v7: {}",
                mail.subject
            );
        }

        let token = extract_token(&mail_body);
        let pow = ctx.server_proof_of_work(&token).await;
        let resp = ctx
            .client
            .post(format!("{}/authenticate/token", ctx.base_url))
            .json(&serde_json::json!({ "pow": pow }))
            .send()
            .await
            .expect("POST token");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let session = resp
            .json::<serde_json::Value>()
            .await
            .expect("token response json")["session_token"]
            .as_str()
            .expect("session_token present")
            .to_string();

        let verify = ctx
            .client
            .post(format!("{}/authenticate/verify", ctx.base_url))
            .header("nail-token", &session)
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST verify");
        assert_eq!(verify.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn article_create_with_initial_version_over_real_tcp() {
        let ctx = EndToEndHttpContext::start().await;

        let email = "e2e_article@qq.com";
        ctx.submit_email_authentication(email).await;
        let mail_body = ctx.wait_for_mail(email, 5).await;
        let token = extract_token(&mail_body);
        let pow = ctx.server_proof_of_work(&token).await;
        let session = ctx
            .client
            .post(format!("{}/authenticate/token", ctx.base_url))
            .json(&serde_json::json!({ "pow": pow }))
            .send()
            .await
            .expect("POST token")
            .json::<serde_json::Value>()
            .await
            .expect("token json")["session_token"]
            .as_str()
            .expect("session_token")
            .to_string();

        let pdf = unit_pdf();
        let multipart = reqwest::multipart::Form::new()
            .text("title", "e2e title")
            .text("summary", "e2e summary")
            .text("tags", "#e2e")
            .text("version", "1.0.0")
            .text("note", "initial")
            .part(
                "file",
                reqwest::multipart::Part::bytes(pdf).file_name("v1.pdf"),
            );
        let create = ctx
            .client
            .post(format!("{}/article", ctx.base_url))
            .header("nail-token", &session)
            .multipart(multipart)
            .send()
            .await
            .expect("POST article");
        assert_eq!(
            create.status(),
            reqwest::StatusCode::CREATED,
            "create article: {}",
            create.text().await.expect("create body")
        );

        let list = ctx
            .client
            .get(format!("{}/article", ctx.base_url))
            .header("nail-token", &session)
            .send()
            .await
            .expect("GET article");
        assert_eq!(list.status(), reqwest::StatusCode::OK);
    }
}

pub(crate) fn unit_pdf() -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1 = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n";
    let obj2 = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n";
    let obj3 = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\n";
    let off1 = header.len();
    let off2 = off1 + obj1.len();
    let off3 = off2 + obj2.len();
    let xref_start = off3 + obj3.len();
    let mut pdf = Vec::from(header);
    pdf.extend_from_slice(obj1);
    pdf.extend_from_slice(obj2);
    pdf.extend_from_slice(obj3);
    pdf.extend_from_slice(
        format!(
            "xref\n0 4\n0000000000 65535 f\n{off1:010} 00000 n\n{off2:010} 00000 n\n{off3:010} 00000 n\n\
             trailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n{xref_start}\n%%EOF"
        )
        .as_bytes(),
    );
    pdf
}

pub(crate) fn extract_token(mail_body: &str) -> String {
    let candidates: Vec<String> = mail_body
        .split_whitespace()
        .filter(|w| uuid::Uuid::parse_str(w).is_ok() && w.len() == 36 && w.as_bytes()[14] == b'7')
        .map(|s| s.to_string())
        .collect();
    candidates
        .last()
        .cloned()
        .expect("authentication mail must contain a UUID v7 token")
}
