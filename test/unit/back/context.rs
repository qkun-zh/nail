use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use nail_common::pow::{Challenge, Pow, ProveInput, prove};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::config::logging::LoggingConfig;
use crate::infrastructure::config::server::ServerConfig;
use crate::infrastructure::pdf::{PdfUpload, TempPdf};
use crate::infrastructure::state::AppState;
use crate::interface;
use crate::repository;
use crate::repository::cache::ChallengeEntry;

#[derive(Clone, Default)]
pub struct RecordingSender {
    pub sent: Arc<std::sync::Mutex<Vec<(String, String, String)>>>,
}

impl emailer::EmailSender for RecordingSender {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> emailer::BoxFuture<'a, Result<(), emailer::SendEmailError>> {
        let sent = self.sent.clone();
        let to = to.to_string();
        let subject = subject.to_string();
        let body = body.to_string();
        Box::pin(async move {
            let mut messages = sent.lock().map_err(|_| {
                emailer::SendEmailError::Transport("recording sender poisoned".to_string())
            })?;
            messages.push((to, subject, body));
            Ok(())
        })
    }

    fn clone_box(&self) -> Box<dyn emailer::EmailSender> {
        Box::new(Self {
            sent: Arc::clone(&self.sent),
        })
    }
}

pub struct TestCtx {
    pub state: AppState,
    pub app: Router,
    pub recorder: RecordingSender,
}

impl TestCtx {
    pub async fn new() -> anyhow::Result<Self> {
        Self::with_cooldown_seconds(0).await
    }

    pub async fn with_cooldown_seconds(cooldown_seconds: u64) -> anyhow::Result<Self> {
        let config = test_config();
        let (state, recorder) = build_state(&config, cooldown_seconds).await?;
        let app = interface::router::build_router(state.clone());
        Ok(Self {
            state,
            app,
            recorder,
        })
    }

    pub async fn with_config(config: AppConfig) -> anyhow::Result<Self> {
        let (state, recorder) = build_state(&config, 0).await?;
        let app = interface::router::build_router(state.clone());
        Ok(Self {
            state,
            app,
            recorder,
        })
    }

    pub fn difficulty(&self) -> u64 {
        self.state.config.server.pow_difficulty_iterations
    }

    pub fn client_pow(&self, payload: &str) -> Pow {
        prove(ProveInput {
            challenge: Challenge {
                id: Uuid::now_v7(),
                difficulty: self.difficulty(),
            },
            payload: payload.to_string(),
        })
        .expect("proof generation must succeed")
    }

    pub fn issued_pow(&self, payload: &str) -> Pow {
        let challenge = Challenge {
            id: Uuid::now_v7(),
            difficulty: self.difficulty(),
        };
        self.state
            .caches
            .challenge
            .insert(&challenge.id.to_string(), ChallengeEntry);
        prove(ProveInput {
            challenge,
            payload: payload.to_string(),
        })
        .expect("proof generation must succeed")
    }

    pub fn upload(&self, bytes: &[u8]) -> PdfUpload {
        let hash = nail_common::hash::pdf(bytes);
        let temp_path = std::path::Path::new(&self.state.config.server.pdf_storage_path)
            .join(".tmp")
            .join(format!("{}.pdf", uuid::Uuid::now_v7()));
        std::fs::write(&temp_path, bytes).expect("write temp pdf");
        PdfUpload::received(hash, TempPdf::new(temp_path))
    }

    pub fn emails(&self) -> Vec<(String, String, String)> {
        self.recorder
            .sent
            .lock()
            .expect("recording sender lock")
            .clone()
    }

    pub async fn create_tag(&self, name: &str) -> String {
        crate::repository::tag::create_tag(&self.state.graph, name)
            .await
            .expect("create tag")
    }

    pub async fn seed_tags(&self, names: &[&str]) {
        for name in names {
            self.create_tag(name).await;
        }
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.json("GET", uri, None, token).await
    }

    pub async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        self.json("POST", uri, Some(body), token).await
    }

    pub async fn get_bytes(
        &self,
        uri: &str,
        token: Option<&str>,
    ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let mut builder = Request::builder().method("GET").uri(uri);
        if let Some(token) = token {
            builder = builder.header("session-token", token);
        }
        let request = builder.body(Body::empty()).expect("build request");
        let response = self.app.clone().oneshot(request).await.expect("oneshot");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        (status, headers, bytes.to_vec())
    }

    pub async fn json(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header("session-token", token);
        }
        let body =
            body.map(|value| Body::from(serde_json::to_vec(&value).expect("serialize json")));
        let request = builder
            .header("content-type", "application/json")
            .body(body.unwrap_or_else(Body::empty))
            .expect("build request");
        self.run(request).await
    }

    pub async fn post_multipart(
        &self,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str)],
        file_field: &str,
        file_name: &str,
        file_bytes: &[u8],
    ) -> (StatusCode, Value) {
        let boundary = format!("nail-test-boundary-{}", Uuid::now_v7());
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                )
                .as_bytes(),
            );
        }
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{file_field}\"; filename=\"{file_name}\"\r\nContent-Type: application/pdf\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(file_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let mut builder = Request::builder().method("POST").uri(uri);
        if let Some(token) = token {
            builder = builder.header("session-token", token);
        }
        let request = builder
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .expect("build multipart request");
        self.run(request).await
    }

    async fn run(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.app.clone().oneshot(request).await.expect("oneshot");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("response must be json")
        };
        (status, value)
    }
}

pub async fn build_state(
    config: &AppConfig,
    _cooldown_seconds: u64,
) -> anyhow::Result<(AppState, RecordingSender)> {
    let mut config = config.clone();
    config.server.pdf_storage_path = std::env::temp_dir()
        .join(format!("nail_test_pdf_{}", uuid::Uuid::now_v7()))
        .to_string_lossy()
        .to_string();
    let graph = repository::graph::open("memory")?;
    repository::seed::init_graph(&graph, &config.server.user_zero_email).await?;
    let search_dir =
        std::env::temp_dir().join(format!("nail_state_search_{}", uuid::Uuid::now_v7()));
    let search = repository::search::SearchIndex::open_or_create_with_segments(
        search_dir.to_str().expect("temp path"),
        2,
    )
    .await?;
    crate::infrastructure::pdf::prepare_pdf_storage(&config.server.pdf_storage_path).await?;
    let caches = repository::cache::TokenCaches::new(
        Duration::from_secs(config.server.token_ttl_seconds),
        Duration::from_secs(config.server.session_ttl_seconds),
        Duration::from_secs(config.server.challenge_ttl_seconds),
        Duration::from_secs(config.server.download_token_ttl_seconds),
        config.server.token_cache_capacity,
    );
    let recorder = RecordingSender::default();
    let email = emailer::Emailer::with_sender(Arc::new(recorder.clone()), &config.emailer);
    let state = AppState {
        graph,
        search,
        caches,
        email,
        config: Arc::new(config.clone()),
    };
    Ok((state, recorder))
}

pub fn test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            listen_addr: "127.0.0.1:0".to_string(),
            db_path: "memory".to_string(),
            search_index_path: "memory-search".to_string(),
            pdf_storage_path: "/tmp/nail_test_pdf".to_string(),
            pow_difficulty_iterations: 1,
            token_ttl_seconds: 8000,
            session_ttl_seconds: 8000,
            challenge_ttl_seconds: 300,
            download_token_ttl_seconds: 60,
            token_cache_capacity: 100,
            email_cooldown_seconds: 60,
            user_zero_email: "user-zero@example.com".to_string(),
            max_pdf_size_bytes: 32 * 1024 * 1024,
            max_tags_per_article: 8,
            max_title_chars: 200,
            max_summary_chars: 2000,
            max_comment_body_chars: 1024,
            max_version_note_chars: 1024,
            max_text_field_bytes: 1024 * 1024,
            max_search_query_chars: 512,
            search_page_size: 8,
            max_search_pages: 1024,
        },
        logging: LoggingConfig {
            dir: "log/back".to_string(),
            retention_days: 7,
            filter: "warn".to_string(),
        },
        emailer: emailer::EmailerConfig {
            host: "localhost".to_string(),
            port: 1,
            username: String::new(),
            password: String::new(),
            from_email: "noreply@example.com".to_string(),
            from_name: "nail".to_string(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: false,
            per_recipient_cooldown_secs: 0,
            global_max_per_minute: 30,
        },
        email_allowed_domains: vec!["example.com".to_string()],
    }
}

pub fn valid_pdf() -> Vec<u8> {
    b"%PDF-1.4\n%%EOF\n".to_vec()
}

pub fn unique_pdf(seed: &str) -> Vec<u8> {
    format!("%PDF-1.4\n{seed}\n%%EOF\n").into_bytes()
}
