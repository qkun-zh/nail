
use std::sync::Arc;

use common::pow::{Challenge, ProveInput, prove};
use seekstorm::index::Close;
use uuid::Uuid;

use super::smtp_sink::{self, MailBox};

pub const TEST_POW_DIFFICULTY: u64 = 16;

pub struct EndToEndHttpContext {
    pub inbox: MailBox,
    pub base_url: String,
    pub client: reqwest::Client,
    search: crate::repo::search::SearchIndexHandle,
    search_dir: std::path::PathBuf,
    pdf_dir: std::path::PathBuf,
    _handle: tokio::task::JoinHandle<()>,
}

impl Drop for EndToEndHttpContext {
    fn drop(&mut self) {
        let index = self.search.clone();
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
        let _ = std::fs::remove_dir_all(&self.search_dir);
        let _ = std::fs::remove_dir_all(&self.pdf_dir);
    }
}

impl EndToEndHttpContext {
    pub async fn start() -> EndToEndHttpContext {
        let (smtp_port, inbox) = smtp_sink::start_sink().await;

        let mut config = crate::other::conf::AppConfig::load().expect("embedded config must load");
        config.server.pow_difficulty_iterations = TEST_POW_DIFFICULTY;
        config.smtp.host = "127.0.0.1".to_string();
        config.smtp.port = smtp_port;
        config.smtp.username.clear();
        config.smtp.password.clear();
        config.smtp.starttls = false;
        config.server.db_path = "memory".to_string();
        config.server.db_namespace = "e2e_ns".to_string();
        config.server.db_database = "e2e_db".to_string();
        let pdf_dir = std::env::temp_dir().join(format!("nail_e2e_pdf_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&pdf_dir).expect("create e2e pdf dir");
        config.server.pdf_storage_path = pdf_dir.to_string_lossy().to_string();
        let search_dir = std::env::temp_dir().join(format!("nail_e2e_search_{}", Uuid::now_v7()));
        config.server.search_index_path = search_dir.to_string_lossy().to_string();

        let db = crate::repo::new(&config.server.db_path)
            .await
            .expect("e2e graph db");
        crate::repo::schema::init_graph(&db)
            .await
            .expect("e2e schema");
        let search = crate::repo::search::open_or_create_index(&config.server.search_index_path)
            .await
            .expect("e2e search index");
        crate::repo::search::rebuild_index(&search, &db)
            .await
            .expect("e2e search index rebuild");

        let cache = crate::repo::TokenCaches::new(
            std::time::Duration::from_secs(config.server.token_ttl_seconds),
            std::time::Duration::from_secs(config.server.session_ttl_seconds),
            std::time::Duration::from_secs(config.server.download_token_ttl_seconds),
            std::time::Duration::from_secs(config.server.challenge_ttl_seconds),
            config.server.token_cache_capacity,
        );
        let shared_config = Arc::new(config);
        let state = crate::other::AppState {
            db: db.clone(),
            search: search.clone(),
            cache: cache.clone(),
            email: crate::other::email::EmailService::new(
                shared_config.smtp.clone(),
                shared_config.server.email_cooldown_seconds,
            ),
            config: shared_config.clone(),
        };

        let app = crate::api::router(state.clone()).expect("e2e router");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("e2e bind");
        let addr = listener.local_addr().expect("e2e addr");
        let pdf_dir_in_task = pdf_dir.clone();
        let handle = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await;
            let _ = std::fs::remove_dir_all(&pdf_dir_in_task);
        });

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build http client");

        EndToEndHttpContext {
            inbox,
            base_url: format!("http://{addr}"),
            client,
            search: search.clone(),
            search_dir,
            pdf_dir,
            _handle: handle,
        }
    }

    pub async fn server_proof_of_work(&self, payload: &str) -> common::pow::Pow {
        let challenge: Challenge = self
            .client
            .get(format!("{}/authenticate/challenge", self.base_url))
            .send()
            .await
            .expect("GET challenge")
            .json()
            .await
            .expect("challenge json");
        prove(ProveInput {
            challenge,
            payload: payload.to_string(),
        })
        .expect("proof generation must succeed")
    }

    pub async fn submit_email_authentication(
        &self,
        email: &str,
    ) -> (Option<String>, serde_json::Value) {
        let challenge: Challenge = self
            .client
            .get(format!("{}/authenticate/challenge", self.base_url))
            .send()
            .await
            .expect("GET challenge")
            .json()
            .await
            .expect("challenge json");
        let pow = prove(ProveInput {
            challenge,
            payload: email.to_string(),
        })
        .expect("proof generation must succeed");
        let resp = self
            .client
            .post(format!("{}/authenticate/pow", self.base_url))
            .json(&pow)
            .send()
            .await
            .expect("POST pow");
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.expect("pow response json");
        assert_eq!(
            status,
            reqwest::StatusCode::OK,
            "submit_email_authentication failed: {body}"
        );
        let subject = body["email_subject"].as_str().map(|s| s.to_string());
        (subject, body)
    }

    pub async fn wait_for_mail(&self, to: &str, timeout_secs: u64) -> String {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            let mail = {
                let inbox = self.inbox.lock().expect("inbox lock");
                inbox
                    .iter()
                    .find(|m| m.to.eq_ignore_ascii_case(to))
                    .cloned()
            };
            if let Some(m) = mail {
                return m.body;
            }
            if std::time::Instant::now() > deadline {
                panic!("timed out waiting for SMTP mail to {to}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
