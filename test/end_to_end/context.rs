use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use nail_common::pow::{Challenge, Pow, ProveInput, prove};
use uuid::Uuid;

use super::smtp_sink::{self, MailBox};

pub const TEST_POW_DIFFICULTY: u64 = 16;
const INTERACT_TIMEOUT_SECS: u64 = 15;

async fn reserve_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve port");
    listener.local_addr().expect("reserved addr").port()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|dir| dir.parent())
        .expect("repo root")
        .to_path_buf()
}

fn write_config(
    conf_dir: &std::path::Path,
    backend_port: u16,
    smtp_port: u16,
) -> (PathBuf, PathBuf, PathBuf) {
    let pdf_dir = std::env::temp_dir().join(format!("nail_e2e_pdf_{}", Uuid::now_v7()));
    let search_dir = std::env::temp_dir().join(format!("nail_e2e_search_{}", Uuid::now_v7()));
    let log_dir = std::env::temp_dir().join(format!("nail_e2e_log_{}", Uuid::now_v7()));

    let server = format!(
        r#"listen_addr = "127.0.0.1:{backend_port}"
db_path = "memory"
search_index_path = "{search}"
pdf_storage_path = "{pdf}"
pow_difficulty_iterations = {difficulty}
token_ttl_seconds = 8000
session_ttl_seconds = 8000
challenge_ttl_seconds = 300
download_token_ttl_seconds = 60
token_cache_capacity = 100000
email_cooldown_seconds = 1
timezone_offset_seconds = 28800
user_zero_email = "admin@example.com"
max_pdf_size_bytes = 33554432
max_tags_per_article = 8
max_title_chars = 200
max_summary_chars = 2000
max_comment_body_chars = 1024
max_version_note_chars = 1024
max_text_field_bytes = 1048576
max_search_query_chars = 512
search_page_size = 8
max_search_pages = 1024
log_dir = "{log}"
log_retention_days = 7
log_max_file_count = 10080
log_prune_interval_secs = 1800
log_filter = "warn"
"#,
        backend_port = backend_port,
        search = search_dir.display(),
        pdf = pdf_dir.display(),
        difficulty = TEST_POW_DIFFICULTY,
        log = log_dir.display(),
    );
    std::fs::write(conf_dir.join("server.toml"), server).expect("write server.toml");

    let smtp = format!(
        r#"host = "127.0.0.1"
port = {smtp_port}
username = ""
password = ""
from_email = "nail-test@localhost"
from_name = "nail"
timeout_secs = 10
wall_clock_timeout_secs = 30
starttls = false
"#
    );
    std::fs::write(conf_dir.join("smtp.toml"), smtp).expect("write smtp.toml");

    let email = r#"allowed_domains = ["example.com"]
"#;
    std::fs::write(conf_dir.join("email.toml"), email).expect("write email.toml");

    (pdf_dir, search_dir, log_dir)
}

async fn wait_for_http_ok(url: &str, timeout: Duration) {
    let client = reqwest::Client::new();
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(response) = client.get(url).send().await
            && response.status().is_success()
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "server never became healthy at {url}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

pub struct TestBackend {
    pub base_url: String,
    pub inbox: MailBox,
    pub client: reqwest::Client,
    backend: Child,
    _conf_dir: PathBuf,
    _pdf_dir: PathBuf,
    _search_dir: PathBuf,
    _log_dir: PathBuf,
}

impl TestBackend {
    pub async fn start() -> Self {
        let (smtp_port, inbox) = smtp_sink::start_sink().await;
        let backend_port = reserve_port().await;
        let conf_dir = std::env::temp_dir().join(format!("nail_e2e_conf_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&conf_dir).expect("create conf dir");
        let (pdf_dir, search_dir, log_dir) = write_config(&conf_dir, backend_port, smtp_port);

        let backend = Command::new(env!("CARGO_BIN_EXE_nail_back"))
            .env("CONF_DIR", &conf_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn nail_back");
        let base_url = format!("http://127.0.0.1:{backend_port}");
        wait_for_http_ok(&format!("{base_url}/config/read"), Duration::from_secs(30)).await;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build http client");

        Self {
            base_url,
            inbox,
            client,
            backend,
            _conf_dir: conf_dir,
            _pdf_dir: pdf_dir,
            _search_dir: search_dir,
            _log_dir: log_dir,
        }
    }

    pub async fn wait_for_mail(&self, to: &str, timeout_secs: u64) -> String {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            let mail = {
                let mut inbox = self.inbox.lock().expect("inbox lock");
                let position = inbox.iter().position(|m| m.to.eq_ignore_ascii_case(to));
                position.map(|index| inbox.remove(index))
            };
            if let Some(mail) = mail {
                return mail.body;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for mail to {to}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn server_pow(&self, payload: &str) -> Pow {
        let body: serde_json::Value = self
            .client
            .get(format!("{}/challenge/read", self.base_url))
            .send()
            .await
            .expect("GET challenge")
            .json()
            .await
            .expect("challenge json");
        let challenge = Challenge {
            id: Uuid::parse_str(body["data"]["id"].as_str().expect("challenge id"))
                .expect("challenge uuid"),
            difficulty: body["data"]["difficulty"].as_u64().expect("difficulty"),
        };
        prove(ProveInput {
            challenge,
            payload: payload.to_string(),
        })
        .expect("proof generation must succeed")
    }

    pub async fn authenticate(&self, email: &str) -> String {
        let pow = self.server_pow(email).await;
        let response = self
            .client
            .post(format!("{}/email/read?intent=authenticate", self.base_url))
            .json(&serde_json::json!({ "pow": pow }))
            .send()
            .await
            .expect("POST email/read");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let mail_body = self.wait_for_mail(email, 10).await;
        let token = smtp_sink::extract_token(&mail_body);
        let token_pow = self.server_pow(&token).await;
        let response = self
            .client
            .post(format!("{}/user/create", self.base_url))
            .json(&serde_json::json!({ "pow": token_pow }))
            .send()
            .await
            .expect("POST user/create");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        response
            .json::<serde_json::Value>()
            .await
            .expect("session json")["data"]["session_token"]
            .as_str()
            .expect("session_token")
            .to_string()
    }
}

impl Drop for TestBackend {
    fn drop(&mut self) {
        let _ = self.backend.kill();
        let _ = self.backend.wait();
        let _ = std::fs::remove_dir_all(&self._conf_dir);
        let _ = std::fs::remove_dir_all(&self._pdf_dir);
        let _ = std::fs::remove_dir_all(&self._search_dir);
        let _ = std::fs::remove_dir_all(&self._log_dir);
    }
}

pub struct BrowserContext {
    pub frontend_url: String,
    pub backend: TestBackend,
    pub page: Page,
    browser: Browser,
    _handler_task: tokio::task::JoinHandle<()>,
    trunk: Child,
}

impl BrowserContext {
    pub async fn start() -> Self {
        let backend = TestBackend::start().await;

        let frontend_port = reserve_port().await;
        let frontend_url = format!("http://127.0.0.1:{frontend_port}");
        let cache_dir = std::env::temp_dir().join("trunk-e2e-cache");
        std::fs::create_dir_all(&cache_dir).expect("create trunk cache dir");

        let trunk = Command::new("trunk")
            .arg("serve")
            .arg("--address")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(frontend_port.to_string())
            .arg("--proxy-backend")
            .arg(format!("{}/", backend.base_url))
            .arg("--proxy-rewrite")
            .arg("/api/")
            .arg("--no-autoreload")
            .env("XDG_CACHE_HOME", &cache_dir)
            .env_remove("NO_COLOR")
            .current_dir(repo_root().join("code/front"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn trunk serve");
        wait_for_http_ok(&frontend_url, Duration::from_secs(120)).await;

        let chrome = std::env::var("NAIL_E2E_CHROME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/usr/bin/chromium"));
        let config = BrowserConfig::builder()
            .chrome_executable(chrome)
            .no_sandbox()
            .new_headless_mode()
            .user_data_dir(std::env::temp_dir().join(format!("nail_chrome_{}", Uuid::now_v7())))
            .build()
            .expect("browser config");
        let (browser, mut handler) = Browser::launch(config).await.expect("launch chromium");
        let handler_task = tokio::spawn(async move {
            while let Some(message) = handler.next().await {
                if message.is_err() {
                    break;
                }
            }
        });
        let page = browser
            .new_page(frontend_url.clone())
            .await
            .expect("open page");

        Self {
            frontend_url,
            backend,
            page,
            browser,
            _handler_task: handler_task,
            trunk,
        }
    }

    pub async fn body_text(&self) -> String {
        self.page
            .evaluate("document.body.innerText")
            .await
            .expect("evaluate innerText")
            .into_value()
            .expect("innerText value")
    }

    pub async fn wait_for_text(&self, needle: &str, timeout_secs: u64) {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            let text = self.body_text().await;
            if text.contains(needle) {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "page never contained {needle:?}; body so far: {:?}",
                text.chars().take(400).collect::<String>()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn local_storage(&self) -> serde_json::Value {
        let raw: String = self
            .page
            .evaluate("JSON.stringify(localStorage)")
            .await
            .expect("evaluate localStorage")
            .into_value()
            .expect("localStorage value");
        serde_json::from_str(&raw).expect("parse localStorage json")
    }

    pub async fn click_retry(&self, selector: &str, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(element) = self.page.find_element(selector).await
                && element.click().await.is_ok()
            {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to click {what} (selector {selector:?})"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn type_retry(&self, selector: &str, text: &str, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(element) = self.page.find_element(selector).await
                && element.click().await.is_ok()
                && element.type_str(text).await.is_ok()
            {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to type into {what} (selector {selector:?})"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn login_via_ui(&self, email: &str) -> String {
        self.page
            .goto(format!("{}/private/authenticate", self.frontend_url))
            .await
            .expect("goto authenticate");
        self.wait_for_text("authenticate", 15).await;

        self.type_retry("form:nth-of-type(1) input", email, "email input")
            .await;
        self.click_retry("form:nth-of-type(1) button", "send button")
            .await;
        let mail_body = self.backend.wait_for_mail(email, 20).await;
        let token = smtp_sink::extract_token(&mail_body);

        self.type_retry("form:nth-of-type(2) input", &token, "token input")
            .await;
        self.click_retry("form:nth-of-type(2) button", "sign in button")
            .await;

        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(session) = session_token_from_storage(&self.local_storage().await) {
                return session;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "session_token never landed in localStorage"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Drop for BrowserContext {
    fn drop(&mut self) {
        let _ = self.trunk.kill();
        let _ = self.trunk.wait();
        std::mem::drop(self.browser.kill());
    }
}

pub fn session_token_from_storage(storage: &serde_json::Value) -> Option<String> {
    storage
        .get("session_token")
        .and_then(|value| value.as_str())
        .and_then(|raw| {
            serde_json::from_str::<String>(raw)
                .ok()
                .or_else(|| Some(raw.to_string()))
        })
        .filter(|token| !token.is_empty())
}
