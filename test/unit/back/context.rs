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
use crate::infrastructure::config::email::EmailConfig;
use crate::infrastructure::config::server::ServerConfig;
use crate::infrastructure::config::smtp::SmtpConfig;
use crate::infrastructure::email::{BoxFuture, EmailSender, RateLimitedSender, SendEmailError};
use crate::infrastructure::state::AppState;
use crate::interface;
use crate::repository;
use crate::repository::cache::ChallengeEntry;

#[derive(Clone, Default)]
pub struct RecordingSender {
    pub sent: Arc<std::sync::Mutex<Vec<(String, String, String)>>>,
}

impl EmailSender for RecordingSender {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, Result<(), SendEmailError>> {
        let sent = self.sent.clone();
        let to = to.to_string();
        let subject = subject.to_string();
        let body = body.to_string();
        Box::pin(async move {
            let mut messages = sent.lock().map_err(|_| {
                SendEmailError::Transport(anyhow::anyhow!("recording sender poisoned"))
            })?;
            messages.push((to, subject, body));
            Ok(())
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

    pub fn emails(&self) -> Vec<(String, String, String)> {
        self.recorder
            .sent
            .lock()
            .expect("recording sender lock")
            .clone()
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.json("GET", uri, None, token).await
    }

    pub async fn post(&self, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
        self.json("POST", uri, Some(body), token).await
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
        let body = body.map(|value| Body::from(serde_json::to_vec(&value).expect("serialize json")));
        let request = builder
            .header("content-type", "application/json")
            .body(body.unwrap_or_else(Body::empty))
            .expect("build request");
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
    cooldown_seconds: u64,
) -> anyhow::Result<(AppState, RecordingSender)> {
    let graph = repository::graph::open("memory").await?;
    repository::seed::init_graph(&graph, &config.server.user_zero_email).await?;
    let caches = repository::cache::TokenCaches::new(
        Duration::from_secs(config.server.token_ttl_seconds),
        Duration::from_secs(config.server.session_ttl_seconds),
        Duration::from_secs(config.server.challenge_ttl_seconds),
        config.server.token_cache_capacity,
    );
    let recorder = RecordingSender::default();
    let email = RateLimitedSender::new(Arc::new(recorder.clone()), cooldown_seconds);
    let state = AppState {
        graph,
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
            pow_difficulty_iterations: 1,
            token_ttl_seconds: 8000,
            session_ttl_seconds: 8000,
            challenge_ttl_seconds: 300,
            token_cache_capacity: 100,
            email_cooldown_seconds: 60,
            timezone_offset_seconds: 0,
            user_zero_email: "user-zero@example.com".to_string(),
            log_dir: "log/back".to_string(),
            log_retention_days: 7,
            log_max_file_count: 100,
            log_prune_interval_secs: 1800,
            log_filter: "warn".to_string(),
        },
        smtp: SmtpConfig {
            host: "localhost".to_string(),
            port: 1,
            username: String::new(),
            password: String::new(),
            from_email: "noreply@example.com".to_string(),
            from_name: "nail".to_string(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: false,
        },
        email: EmailConfig {
            allowed_domains: vec!["example.com".to_string()],
        },
    }
}
