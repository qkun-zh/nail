
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use common::pow::{Challenge, Pow, ProveInput, prove};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

pub const SMTP_REFUSED_PORT: u16 = 1;

use seekstorm::index::Close;

pub struct TestCtx {
    pub state: crate::other::AppState,
    pub app: Router,
    pdf_dir: std::path::PathBuf,
    search_dir: std::path::PathBuf,
}

impl TestCtx {
    pub async fn new() -> TestCtx {
        let search_dir = std::env::temp_dir().join(format!("nail_search_index_{}", Uuid::now_v7()));
        let mut state = crate::unit_tests::context::state_in_dir(&search_dir).await;
        let config = Arc::make_mut(&mut state.config);
        config.smtp.host = "127.0.0.1".to_string();
        config.smtp.port = SMTP_REFUSED_PORT;
        config.smtp.password.clear();
        state.email = crate::other::email::EmailService::new(
            config.smtp.clone(),
            config.server.email_cooldown_seconds,
        );
        let pdf_dir = std::env::temp_dir().join(format!("nail_new_pdf_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&pdf_dir).expect("create pdf temp dir");
        config.server.pdf_storage_path = pdf_dir.to_string_lossy().to_string();
        let app = crate::api::router(state.clone()).expect("build router");
        TestCtx {
            state,
            app,
            pdf_dir,
            search_dir,
        }
    }

    pub fn difficulty(&self) -> u64 {
        self.state.config.server.pow_difficulty_iterations
    }


    pub fn client_proof_of_work(&self, payload: &str) -> Pow {
        let challenge = Challenge {
            id: Uuid::now_v7(),
            difficulty: self.difficulty(),
        };
        prove(ProveInput {
            challenge,
            payload: payload.to_string(),
        })
        .expect("proof generation must succeed")
    }

    pub fn issued_proof_of_work(&self, payload: &str) -> Pow {
        let pow = self.client_proof_of_work(payload);
        crate::repo::token::challenge::create_challenge(
            &self.state.cache,
            &pow.challenge.id.to_string(),
        );
        pow
    }

    pub fn tampered(&self, pow: &Pow) -> Pow {
        let mut tampered = pow.clone();
        let first = tampered
            .solution
            .chars()
            .next()
            .expect("solution non-empty");
        let flip = if first == '0' { '1' } else { '0' };
        tampered.solution = format!("{flip}{}", &tampered.solution[1..]);
        tampered
    }


    pub async fn json(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let body = body.map(|b| Body::from(serde_json::to_vec(&b).expect("serialize json")));
        self.raw_with(method, uri, body, Some("application/json"), token)
            .await
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.json("GET", uri, None, token).await
    }

    pub async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        self.json("POST", uri, Some(body), token).await
    }

    pub async fn multipart(
        &self,
        method: &str,
        uri: &str,
        fields: &[(&str, Vec<u8>)],
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let boundary = "----nail_new_boundary";
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            );
            body.extend_from_slice(value);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        let content_type = format!("multipart/form-data; boundary={boundary}");
        self.raw_with(
            method,
            uri,
            Some(Body::from(body)),
            Some(&content_type),
            token,
        )
        .await
    }

    pub async fn download(
        &self,
        uri: &str,
        token: Option<&str>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut builder = Request::builder().uri(uri);
        if let Some(t) = token {
            builder = builder.header("nail-token", t);
        }
        let mut req = builder.body(Body::empty()).expect("build request");
        inject_connect_info(&mut req);
        let resp = self.app.clone().oneshot(req).await.expect("oneshot");
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body")
            .to_vec();
        (status, headers, bytes)
    }

    async fn raw_with(
        &self,
        method: &str,
        uri: &str,
        body: Option<Body>,
        content_type: Option<&str>,
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(ct) = content_type {
            builder = builder.header("content-type", ct);
        }
        if let Some(t) = token {
            builder = builder.header("nail-token", t);
        }
        let mut req = builder
            .body(body.unwrap_or_else(Body::empty))
            .expect("build request");
        inject_connect_info(&mut req);
        let resp = self.app.clone().oneshot(req).await.expect("oneshot");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("response must be json")
        };
        (status, value)
    }


    pub async fn register(&self, email: &str) -> (String, String) {
        let email_hash = common::hash::email(email);
        let user_id = crate::repo::user::find_or_create_user(&self.state.db, &email_hash)
            .await
            .expect("find_or_create_user");
        let session = self.session_for(&user_id);
        (user_id, session)
    }

    pub fn session_for(&self, user_id: &str) -> String {
        let session = Uuid::now_v7().to_string();
        crate::repo::token::session::create_session_token(&self.state.cache, &session, user_id);
        session
    }

    pub fn ghost_session(&self) -> String {
        Uuid::now_v7().to_string()
    }

    pub fn malformed_session(&self) -> String {
        "not-a-uuid".to_string()
    }


    pub async fn create_article(
        &self,
        session: &str,
        title: &str,
        summary: &str,
        tags: &str,
        version: &str,
        note: &str,
    ) -> (String, String) {
        let pdf = crate::unit_tests::context::test_pdf_variant(&format!("{title}|{version}"));
        let (status, body) = self
            .multipart(
                "POST",
                "/article",
                &[
                    ("title", title.as_bytes().to_vec()),
                    ("summary", summary.as_bytes().to_vec()),
                    ("tags", tags.as_bytes().to_vec()),
                    ("version", version.as_bytes().to_vec()),
                    ("note", note.as_bytes().to_vec()),
                    ("file", pdf),
                ],
                Some(session),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "create_article failed: {}",
            body
        );
        let article_id = body["article_id"]
            .as_str()
            .expect("article_id in response")
            .to_string();
        let version_id = body["version_id"]
            .as_str()
            .expect("version_id in response")
            .to_string();
        (article_id, version_id)
    }

    pub async fn add_version(
        &self,
        session: &str,
        article_id: &str,
        version: &str,
        note: &str,
        pdf: Option<&[u8]>,
    ) -> String {
        let pdf = match pdf {
            Some(p) => p.to_vec(),
            None => crate::unit_tests::context::test_pdf(),
        };
        let (status, body) = self
            .multipart(
                "POST",
                &format!("/article/{article_id}/version"),
                &[
                    ("version", version.as_bytes().to_vec()),
                    ("note", note.as_bytes().to_vec()),
                    ("file", pdf),
                ],
                Some(session),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "add_version failed: {}", body);
        let version_id = body["version_id"]
            .as_str()
            .expect("version_id in response")
            .to_string();
        version_id
    }

    pub async fn seed_article(&self, session: &str) -> (String, String) {
        self.create_article(
            session,
            "seed title",
            "seed summary",
            "#seed",
            "1.0.0",
            "initial",
        )
        .await
    }

    pub fn test_pdf(&self) -> Vec<u8> {
        crate::unit_tests::context::test_pdf()
    }

    pub fn pdf_storage_path(&self) -> &std::path::Path {
        &self.pdf_dir
    }


    pub fn expect(&self, actual: StatusCode, expected: StatusCode, ctx: &str) {
        assert_eq!(actual, expected, "{ctx}: expected {expected}, got {actual}");
    }

    pub fn ok(&self, s: StatusCode) {
        self.expect(s, StatusCode::OK, "expected 200 OK");
    }

    pub fn created(&self, s: StatusCode) {
        self.expect(s, StatusCode::CREATED, "expected 201 CREATED");
    }

    pub fn bad(&self, s: StatusCode) {
        self.expect(s, StatusCode::BAD_REQUEST, "expected 400 BAD_REQUEST");
    }

    pub fn unauth(&self, s: StatusCode) {
        self.expect(s, StatusCode::UNAUTHORIZED, "expected 401 UNAUTHORIZED");
    }

    pub fn forbidden(&self, s: StatusCode) {
        self.expect(s, StatusCode::FORBIDDEN, "expected 403 FORBIDDEN");
    }

    pub fn not_found(&self, s: StatusCode) {
        self.expect(s, StatusCode::NOT_FOUND, "expected 404 NOT_FOUND");
    }

    pub fn reason(&self, body: &Value) -> String {
        body.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }


    pub async fn incoming_edge_from_id(
        &self,
        target_kind: &str,
        edge_type: &str,
        target_id: &str,
    ) -> Option<String> {
        let db = self.state.db.read().await;
        let target = crate::repo::db::resolve_node_id_sync(&db, target_kind, target_id)
            .expect("resolve target")
            .expect("target must exist");
        let edges = db
            .exec(
                agdb::QueryBuilder::search()
                    .to(target)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(crate::repo::types::KEY_TYPE)
                    .value(edge_type)
                    .query(),
            )
            .expect("edge query");
        edges
            .elements
            .first()
            .map(|el| {
                crate::repo::db::read_node_sync::<crate::repo::types::IdRow>(&db, el.from)
                    .map(|r| r.map(|row| row.id))
            })
            .transpose()
            .expect("read from id")
            .flatten()
    }

    pub async fn article_tag_names(&self, article_id: &str) -> Vec<String> {
        let db = self.state.db.read().await;
        let edges = db
            .exec(
                agdb::QueryBuilder::search()
                    .from(crate::repo::types::alias_of(
                        crate::repo::types::ENTITY_TYPE_ARTICLE,
                        article_id,
                    ))
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(crate::repo::types::KEY_TYPE)
                    .value(crate::repo::types::EDGE_ARTICLE_TO_TAG)
                    .query(),
            )
            .expect("tag edge query");
        let mut names: Vec<String> = edges
            .elements
            .iter()
            .filter_map(|el| {
                crate::repo::db::read_node_sync::<crate::repo::types::TagRow>(&db, el.to)
                    .map(|row| row.map(|r| r.tag_name))
                    .expect("read tag name")
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for TestCtx {
    fn drop(&mut self) {
        let index = self.state.search.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("seekstorm-close".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(index.close());
                }
                let _ = tx.send(());
            })
            .is_ok();
        if spawned {
            let _ = rx.recv();
        }
        let _ = std::fs::remove_dir_all(&self.pdf_dir);
        let _ = std::fs::remove_dir_all(&self.search_dir);
    }
}

fn inject_connect_info<B>(req: &mut Request<B>) {
    req.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:3000"
            .parse::<SocketAddr>()
            .expect("static socket addr"),
    ));
}

#[path = "context_fixtures.rs"]
mod fixtures;
pub use fixtures::*;
