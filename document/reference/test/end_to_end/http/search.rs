
use std::time::Duration;

use common::pow::Pow;
use reqwest::StatusCode;
use serde_json::{Value, json};

use super::super::extract_token;
use super::context::EndToEndHttpContext;

async fn login(ctx: &EndToEndHttpContext, email: &str) -> String {
    ctx.submit_email_authentication(email).await;
    let mail_body = ctx.wait_for_mail(email, 10).await;
    let token = extract_token(&mail_body);
    let pow = ctx.server_proof_of_work(&token).await;
    let resp = ctx
        .client
        .post(format!("{}/authenticate/token", ctx.base_url))
        .json(&json!({ "pow": pow }))
        .send()
        .await
        .expect("POST token");
    assert_eq!(resp.status(), StatusCode::OK);
    resp.json::<Value>().await.expect("token response json")["session_token"]
        .as_str()
        .expect("session_token present")
        .to_string()
}

async fn create_article(
    ctx: &EndToEndHttpContext,
    session: &str,
    title: &str,
    summary: &str,
    tags: &str,
    version: &str,
    note: &str,
) -> (String, String) {
    let form = reqwest::multipart::Form::new()
        .text("title", title.to_string())
        .text("summary", summary.to_string())
        .text("tags", tags.to_string())
        .text("version", version.to_string())
        .text("note", note.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_variant(title)).file_name("seed.pdf"),
        );
    let resp = ctx
        .client
        .post(format!("{}/article", ctx.base_url))
        .header("nail-token", session)
        .multipart(form)
        .send()
        .await
        .expect("POST article");
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create article: {}",
        resp.text().await.expect("create body")
    );
    let body: Value = resp.json().await.expect("create json");
    (
        body["article_id"].as_str().expect("article_id").to_string(),
        body["version_id"].as_str().expect("version_id").to_string(),
    )
}

async fn update_name(ctx: &EndToEndHttpContext, session: &str, name: &str) {
    let pow: Pow = ctx.server_proof_of_work(name).await;
    let resp = ctx
        .client
        .post(format!("{}/user/name", ctx.base_url))
        .header("nail-token", session)
        .json(&json!({ "pow": pow }))
        .send()
        .await
        .expect("POST user/name");
    assert_eq!(resp.status(), StatusCode::OK, "set name failed");
}

async fn add_comment(ctx: &EndToEndHttpContext, session: &str, version_id: &str, content: &str) {
    let resp = ctx
        .client
        .post(format!("{}/version/{version_id}/comments", ctx.base_url))
        .header("nail-token", session)
        .json(&json!({ "content": content }))
        .send()
        .await
        .expect("POST comment");
    assert_eq!(resp.status(), StatusCode::CREATED, "create comment failed");
}

async fn search(ctx: &EndToEndHttpContext, session: &str, query: &str) -> Value {
    let resp = ctx
        .client
        .get(format!("{}/article/search{query}", ctx.base_url))
        .header("nail-token", session)
        .send()
        .await
        .expect("GET article/search");
    assert_eq!(resp.status(), StatusCode::OK, "search {query} not 200");
    resp.json::<Value>().await.expect("search json")
}

fn ids(body: &Value) -> Vec<String> {
    body["article_list"]
        .as_array()
        .expect("article_list array")
        .iter()
        .filter_map(|a| a["id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn search_multiword_and_time_window_over_real_tcp() {
    let ctx = EndToEndHttpContext::start().await;
    let email = format!("e2e_search_and_{}@qq.com", uuid::Uuid::now_v7());
    let session = login(&ctx, &email).await;

    let (a, version_a) = create_article(
        &ctx,
        &session,
        "memory safety guide",
        "s",
        "#t",
        "1.0.0",
        "n",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let (b, version_b) =
        create_article(&ctx, &session, "memory alone", "s", "#t", "1.0.0", "n").await;

    let body = search(&ctx, &session, "?q=memory%20safety&ranges=title").await;
    assert_eq!(ids(&body), vec![a.clone()], "AND 必须只命中两词齐备的文章");

    let t_a = common::time::uuidv7_timestamp_secs(&version_a).expect("version_a uuidv7");
    let t_b = common::time::uuidv7_timestamp_secs(&version_b).expect("version_b uuidv7");
    assert!(t_a < t_b, "seed 间隔必须让两篇落在不同秒");

    let body = search(&ctx, &session, &format!("?from={t_a}&to={t_a}")).await;
    assert_eq!(ids(&body), vec![a.clone()], "from==to 闭区间必须只命中 a");
    let body = search(&ctx, &session, &format!("?from={t_b}")).await;
    assert_eq!(ids(&body), vec![b.clone()], "from=t_b 必须只命中 b");
    let body = search(&ctx, &session, &format!("?to={t_a}")).await;
    assert_eq!(ids(&body), vec![a.clone()], "to=t_a 必须只命中 a");

    let resp = ctx
        .client
        .get(format!(
            "{}/article/search?from={t_b}&to={t_a}",
            ctx.base_url
        ))
        .header("nail-token", &session)
        .send()
        .await
        .expect("GET from>to");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "from>to 必须 400");
    let resp = ctx
        .client
        .get(format!("{}/article/search?q=x&ranges=bogus", ctx.base_url))
        .header("nail-token", &session)
        .send()
        .await
        .expect("GET bad range");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "非法 range 必须 400"
    );
    let resp = ctx
        .client
        .get(format!(
            "{}/article/search?sort=time:sideways",
            ctx.base_url
        ))
        .header("nail-token", &session)
        .send()
        .await
        .expect("GET bad sort");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "非法 sort 必须 400");
    let resp = ctx
        .client
        .get(format!(
            "{}/article/search?q={}",
            ctx.base_url,
            "a".repeat(513)
        ))
        .header("nail-token", &session)
        .send()
        .await
        .expect("GET long q");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "超长 q 必须 400");
}

#[tokio::test]
async fn search_cross_table_ranges_and_hits_over_real_tcp() {
    let ctx = EndToEndHttpContext::start().await;
    let email = format!("e2e_search_ranges_{}@qq.com", uuid::Uuid::now_v7());
    let session = login(&ctx, &email).await;
    update_name(&ctx, &session, "rustacean").await;

    let (x, version_x) = create_article(
        &ctx,
        &session,
        "plain title",
        "plain summary",
        "#rust",
        "1.0.0",
        "note memory leak",
    )
    .await;
    add_comment(&ctx, &session, &version_x, "safety explained in depth").await;

    let body = search(&ctx, &session, "?q=safety&ranges=comment").await;
    assert_eq!(
        ids(&body),
        vec![x.clone()],
        "comment 范围必须命中评论所属文章"
    );
    let hit = &body["article_list"][0]["hits"][0];
    assert_eq!(hit["field"], "comment", "hits 字段名必须是 comment");
    assert_eq!(hit["label"], "评论", "hits 标签必须是中文 评论");
    assert!(
        hit["snippet"]
            .as_str()
            .unwrap_or_default()
            .contains("safety"),
        "snippet 必须含命中词"
    );
    assert!(
        hit["snippet"]
            .as_str()
            .unwrap_or_default()
            .contains("<mark>"),
        "snippet 必须带后端 <mark> 高亮"
    );

    let body = search(&ctx, &session, "?q=rust&ranges=tag").await;
    assert_eq!(
        ids(&body),
        vec![x.clone()],
        "tag 范围必须命中带该标签的文章"
    );
    assert_eq!(body["article_list"][0]["hits"][0]["field"], "tag");
    assert_eq!(body["article_list"][0]["hits"][0]["label"], "标签");

    let body = search(&ctx, &session, "?q=memory&ranges=note").await;
    assert_eq!(ids(&body), vec![x.clone()], "note 范围必须命中最新版本说明");
    assert_eq!(body["article_list"][0]["hits"][0]["field"], "note");
    assert_eq!(body["article_list"][0]["hits"][0]["label"], "版本说明");

    let body = search(&ctx, &session, "?q=rustacean&ranges=author").await;
    assert_eq!(
        ids(&body),
        vec![x.clone()],
        "author 范围必须命中该作者的文章"
    );
    assert_eq!(body["article_list"][0]["hits"][0]["field"], "author");
    assert_eq!(body["article_list"][0]["hits"][0]["label"], "作者");

    let body = search(&ctx, &session, "?q=rust&ranges=title").await;
    assert!(ids(&body).is_empty(), "title 范围不得命中 tag 命中");
}

#[tokio::test]
async fn search_sort_and_pagination_over_real_tcp() {
    let ctx = EndToEndHttpContext::start().await;
    let email = format!("e2e_search_sort_{}@qq.com", uuid::Uuid::now_v7());
    let session = login(&ctx, &email).await;

    let mut created = Vec::new();
    for i in 0..5 {
        created.push(
            create_article(&ctx, &session, &format!("Sort{i}"), "s", "#t", "1.0.0", "n")
                .await
                .0,
        );
        std::thread::sleep(Duration::from_millis(2));
    }

    let body = search(&ctx, &session, "?sort=title:asc").await;
    assert_eq!(ids(&body), created.clone(), "title asc 必须按字母序");

    let body = search(&ctx, &session, "?sort=time:desc").await;
    let mut time_desc = created.clone();
    time_desc.reverse();
    assert_eq!(ids(&body), time_desc, "time desc 必须最新在前");

    let body = search(&ctx, &session, "?sort=title:asc,time:desc").await;
    assert_eq!(ids(&body), created.clone(), "多键排序必须按主键 title asc");

    let body = search(&ctx, &session, "?limit=2&page=1").await;
    assert_eq!(ids(&body), vec![created[4].clone(), created[3].clone()]);
    assert_eq!(body["total"], 5);
    assert_eq!(body["total_pages"], 3);
    assert_eq!(body["has_more"], true);
    assert_eq!(body["has_prev"], false);
    let body = search(&ctx, &session, "?limit=2&page=2").await;
    assert_eq!(ids(&body), vec![created[2].clone(), created[1].clone()]);
    assert_eq!(body["has_more"], true);
    assert_eq!(body["has_prev"], true);
    let body = search(&ctx, &session, "?limit=2&page=3").await;
    assert_eq!(ids(&body), vec![created[0].clone()]);
    assert_eq!(body["has_more"], false);
    assert_eq!(body["has_prev"], true);

    let body = search(&ctx, &session, "?limit=2&page=999").await;
    assert!(ids(&body).is_empty(), "越界页必须空页");
    assert_eq!(body["total"], 5);
    assert_eq!(body["truncated"], false);
}

fn pdf_variant(variant: &str) -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1 = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n";
    let obj2 = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n";
    let obj3 = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\n";
    let variant_comment = format!("% {variant}\n");
    let off1 = header.len();
    let off2 = off1 + obj1.len();
    let off3 = off2 + obj2.len();
    let xref_start = off3 + obj3.len() + variant_comment.len();
    let mut pdf = Vec::from(header);
    pdf.extend_from_slice(obj1);
    pdf.extend_from_slice(obj2);
    pdf.extend_from_slice(obj3);
    pdf.extend_from_slice(variant_comment.as_bytes());
    pdf.extend_from_slice(
        format!(
            "xref\n0 4\n0000000000 65535 f\n{off1:010} 00000 n\n{off2:010} 00000 n\n{off3:010} 00000 n\n\
             trailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n{xref_start}\n%%EOF"
        )
        .as_bytes(),
    );
    pdf
}
