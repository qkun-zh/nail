
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::unit_tests::context::TestCtx;

enum Req {
    None,
    Json(Value),
    Multipart(&'static [(&'static str, &'static str)]),
}

fn pow_body(payload: &str) -> Value {
    json!({
        "challenge": {
            "id": "019f0000-0000-7000-8000-000000000001",
            "difficulty": 0,
        },
        "solution": "00",
        "payload": payload,
    })
}

struct RouteRow {
    method: &'static str,
    path: &'static str,
    req: Req,
    requires_session: bool,
    note: &'static str,
}

const ID: &str = "019f0000-0000-7000-8000-000000000002";
const VERSION_ID: &str = "019f0000-0000-7000-8000-000000000003";

fn routes() -> Vec<RouteRow> {
    vec![
        RouteRow {
            method: "GET",
            path: "/authenticate/challenge",
            req: Req::None,
            requires_session: false,
            note: "公开：签发 PoW challenge（服务端记账）",
        },
        RouteRow {
            method: "POST",
            path: "/authenticate/pow",
            req: Req::Json(pow_body("")),
            requires_session: false,
            note: "公开：邮箱 PoW → 铸造认证 token（空邮箱 → 400 域名不允许）",
        },
        RouteRow {
            method: "POST",
            path: "/authenticate/token",
            req: Req::Json(json!({ "pow": pow_body("") })),
            requires_session: false,
            note: "公开：认证 token 兑换 session（空 payload → 400）",
        },
        RouteRow {
            method: "GET",
            path: "/meta/limits",
            req: Req::None,
            requires_session: false,
            note: "公开：客户端限额镜像",
        },
        RouteRow {
            method: "POST",
            path: "/authenticate/verify",
            req: Req::Json(json!({})),
            requires_session: true,
            note: "session 校验：body = `{}`（VerifySessionRequest 空结构体）；畸形 token 400 / 无效 401 / 有效 200",
        },
        RouteRow {
            method: "POST",
            path: "/user/logout",
            req: Req::Json(json!({ "pow": pow_body("rand") })),
            requires_session: true,
            note: "带 PoW 的登出；payload 是前端短随机串",
        },
        RouteRow {
            method: "GET",
            path: "/user/name",
            req: Req::None,
            requires_session: true,
            note: "显示名读取",
        },
        RouteRow {
            method: "POST",
            path: "/user/name",
            req: Req::Json(json!({ "pow": pow_body("new-name") })),
            requires_session: true,
            note: "显示名设置（PoW payload = 名字）",
        },
        RouteRow {
            method: "POST",
            path: "/user/deregister",
            req: Req::Json(json!({ "pow": pow_body("someone@qq.com") })),
            requires_session: true,
            note: "注销请求（PoW payload = 账号邮箱）",
        },
        RouteRow {
            method: "POST",
            path: "/user/deregister/confirm",
            req: Req::Json(json!({ "pow": pow_body(ID) })),
            requires_session: true,
            note: "注销确认（PoW payload = 邮箱确认 token）",
        },
        RouteRow {
            method: "POST",
            path: "/email/check",
            req: Req::Json(json!({ "pow": pow_body("someone@qq.com") })),
            requires_session: true,
            note: "PoW 邮箱 == 绑定邮箱？",
        },
        RouteRow {
            method: "POST",
            path: "/email/update/send",
            req: Req::Json(json!({
                "old_email_pow": pow_body("old@qq.com"),
                "new_email_pow": pow_body("new@qq.com"),
            })),
            requires_session: true,
            note: "铸造 token 对并双发确认信",
        },
        RouteRow {
            method: "POST",
            path: "/email/update/confirm",
            req: Req::Json(json!({
                "pow": pow_body("email-update-confirm-payload"),
                "old_email_token": ID,
                "new_email_token": VERSION_ID,
            })),
            requires_session: true,
            note: "校验 token 对 → 换 hash → 轮换 session",
        },
        RouteRow {
            method: "POST",
            path: "/author/check",
            req: Req::Json(json!({ "article_id": ID })),
            requires_session: true,
            note: "显示层门面；写操作权威校验在各写 handler",
        },
        RouteRow {
            method: "GET",
            path: "/article",
            req: Req::None,
            requires_session: true,
            note: "文章列表（分页）",
        },
        RouteRow {
            method: "POST",
            path: "/article",
            req: Req::Multipart(&[
                ("title", "t"),
                ("summary", "s"),
                ("tags", "#x"),
                ("version", "1.0.0"),
                ("note", "n"),
                ("file", "pdf"),
            ]),
            requires_session: true,
            note: "建文（multipart；即带首版本 version/note/file）",
        },
        RouteRow {
            method: "GET",
            path: "/article/search",
            req: Req::None,
            requires_session: true,
            note: "搜索（filter/sort/pagination 全在 query）",
        },
        RouteRow {
            method: "GET",
            path: "/article/{id}",
            req: Req::None,
            requires_session: true,
            note: "文章详情（不含版本列表）",
        },
        RouteRow {
            method: "POST",
            path: "/article/{id}",
            req: Req::Json(json!({ "title": "t", "summary": "s" })),
            requires_session: true,
            note: "改文（UpdateArticleRequest；作者 403 门禁在 logic）",
        },
        RouteRow {
            method: "POST",
            path: "/article/{id}/delete",
            req: Req::Json(json!({})),
            requires_session: true,
            note: "删文（DeleteArticleRequest 空结构体；转移语义）",
        },
        RouteRow {
            method: "GET",
            path: "/article/{id}/version/{version_id}/pdf",
            req: Req::None,
            requires_session: true,
            note: "PDF 直读（归属 gate → 流式响应）",
        },
        RouteRow {
            method: "GET",
            path: "/article/{id}/version",
            req: Req::None,
            requires_session: true,
            note: "版本列表（分页；不随详情返回）",
        },
        RouteRow {
            method: "POST",
            path: "/article/{id}/version",
            req: Req::Multipart(&[("version", "1.0.0"), ("note", "n"), ("file", "pdf")]),
            requires_session: true,
            note: "加版本（multipart；作者 403 门禁）",
        },
        RouteRow {
            method: "GET",
            path: "/article/{id}/version/{version_id}/download",
            req: Req::None,
            requires_session: true,
            note: "mint 单次下载 token（绑定 session 用户）",
        },
        RouteRow {
            method: "GET",
            path: "/article/download?token={version_id}",
            req: Req::None,
            requires_session: true,
            note: "consume 下载 token（query: token）",
        },
        RouteRow {
            method: "GET",
            path: "/version/{id}",
            req: Req::None,
            requires_session: true,
            note: "版本详情（含 note；article_id 归属参数可选）",
        },
        RouteRow {
            method: "GET",
            path: "/version/{id}/comments",
            req: Req::None,
            requires_session: true,
            note: "版本评论树（扁平列表）",
        },
        RouteRow {
            method: "POST",
            path: "/version/{id}/comments",
            req: Req::Json(json!({ "content": "c" })),
            requires_session: true,
            note: "顶层评论（CreateCommentRequest）",
        },
        RouteRow {
            method: "POST",
            path: "/comments/{id}/replies",
            req: Req::Json(json!({ "content": "c" })),
            requires_session: true,
            note: "回复评论（CreateCommentRequest）",
        },
        RouteRow {
            method: "POST",
            path: "/comments/{id}/delete",
            req: Req::Json(json!({})),
            requires_session: true,
            note: "删评论（DeleteCommentRequest 空结构体；转移语义）",
        },
    ]
}

fn path_with_ids(path: &str) -> String {
    path.replace("{id}", ID).replace("{version_id}", VERSION_ID)
}

async fn assert_gate(
    ctx: &TestCtx,
    row: &RouteRow,
    token: Option<&str>,
    expected: StatusCode,
    label: &str,
) {
    let uri = path_with_ids(row.path);
    let (status, _) = match &row.req {
        Req::None => ctx.json(row.method, &uri, None, token).await,
        Req::Json(body) => ctx.json(row.method, &uri, Some(body.clone()), token).await,
        Req::Multipart(fields) => {
            let bytes: Vec<(&str, Vec<u8>)> = fields
                .iter()
                .map(|(n, v)| (*n, v.as_bytes().to_vec()))
                .collect();
            ctx.multipart(row.method, &uri, &bytes, token).await
        }
    };
    assert_eq!(
        status,
        expected,
        "[{}] {} {} (token={}): expected {expected}, got {status} — {}",
        label,
        row.method,
        row.path,
        token.unwrap_or("<none>"),
        row.note,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_protected_routes_reject_anonymous_requests_with_401() {
    let ctx = TestCtx::new().await;
    for row in routes() {
        if row.requires_session {
            assert_gate(&ctx, &row, None, StatusCode::UNAUTHORIZED, "anonymous").await;
        } else {
            let expected = match row.path {
                "/authenticate/challenge" | "/meta/limits" => StatusCode::OK,
                _ => StatusCode::BAD_REQUEST,
            };
            assert_gate(&ctx, &row, None, expected, "public").await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_protected_routes_reject_malformed_session_token_with_400() {
    let ctx = TestCtx::new().await;
    for row in routes() {
        if row.requires_session {
            assert_gate(
                &ctx,
                &row,
                Some(&ctx.malformed_session()),
                StatusCode::BAD_REQUEST,
                "malformed-token",
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_protected_routes_reject_unknown_session_token_with_401() {
    let ctx = TestCtx::new().await;
    let ghost = ctx.ghost_session();
    for row in routes() {
        if row.requires_session {
            assert_gate(
                &ctx,
                &row,
                Some(&ghost),
                StatusCode::UNAUTHORIZED,
                "unknown-token",
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn challenge_returns_difficulty_from_config() {
    let ctx = TestCtx::new().await;
    let (status, body) = ctx.get("/authenticate/challenge", None).await;
    ctx.ok(status);
    assert_eq!(
        body["difficulty"],
        json!(ctx.difficulty()),
        "challenge 必须带配置难度（客户端自报难度不参与）"
    );
    assert!(body["id"].as_str().is_some(), "challenge 必须带 uuidv7 id");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn limits_exposes_only_client_limits_subset() {
    let ctx = TestCtx::new().await;
    let (status, body) = ctx.get("/meta/limits", None).await;
    ctx.ok(status);
    let s = &ctx.state.config.server;
    assert_eq!(body["max_tags_per_article"], json!(s.max_tags_per_article));
    assert_eq!(
        body["max_comment_body_chars"],
        json!(s.max_comment_body_chars)
    );
    assert_eq!(
        body["max_version_note_chars"],
        json!(s.max_version_note_chars)
    );
    assert_eq!(body["max_title_chars"], json!(s.max_title_chars));
    assert_eq!(body["max_summary_chars"], json!(s.max_summary_chars));
    assert_eq!(body["max_pdf_size_bytes"], json!(s.max_pdf_size_bytes));
    assert_eq!(body["search_page_size"], json!(s.search_page_size));
    assert_eq!(body["max_search_pages"], json!(s.max_search_pages));
    assert_eq!(body["max_page"], json!(s.max_page));
    for leaked in [
        "password",
        "listen_addr",
        "db_path",
        "pdf_storage_path",
        "smtp",
    ] {
        assert!(body.get(leaked).is_none(), "limits 不得泄露 {leaked}");
    }
}
