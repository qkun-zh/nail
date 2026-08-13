
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;

use super::super::http::smtp_sink::{self, MailBox};

pub const TEST_POW_DIFFICULTY: u64 = 16;

const INTERACT_TIMEOUT_SECS: u64 = 10;

pub static BROWSER_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct EndToEndBrowserContext {
    pub base_url: String,
    pub inbox: MailBox,
    pub client: reqwest::Client,
    _browser: Browser,
    pub page: Page,
    backend: Child,
    proxy: Child,
    _conf_dir: PathBuf,
    _proxy_conf_dir: PathBuf,
    _handler_task: tokio::task::JoinHandle<()>,
}

impl EndToEndBrowserContext {
    pub async fn new() -> Self {
        let base_url = "http://localhost:8080".to_string();

        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../front/dist");
        assert!(
            dist.join("index.html").is_file(),
            "missing code/front/dist: run `cd code/front && trunk build --release` first"
        );
        let backend_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/nail_back");
        assert!(
            backend_bin.is_file(),
            "missing {}: run `cargo build` first",
            backend_bin.display()
        );

        let (smtp_port, inbox) = smtp_sink::start_sink().await;

        let conf_dir =
            std::env::temp_dir().join(format!("nail_browser_conf_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&conf_dir).expect("create temp conf dir");
        let prod_conf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conf/back");
        write_patched_server_toml(&conf_dir, &prod_conf.join("server.toml"));
        std::fs::copy(prod_conf.join("email.toml"), conf_dir.join("email.toml"))
            .expect("copy email.toml");
        write_sink_smtp_toml(&conf_dir, smtp_port);

        let backend = Command::new(&backend_bin)
            .env("CONF_DIR", &conf_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn nail_back");
        wait_for_http_ok("http://127.0.0.1:3000/meta/limits", Duration::from_secs(30)).await;

        let pingap_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../code/proxy/pingap-linux-gnu-x86-full");
        let proxy_conf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conf/proxy");
        let proxy_conf_dir = write_patched_proxy_conf(proxy_conf);
        let proxy = Command::new(&pingap_bin)
            .arg("-c")
            .arg(&proxy_conf_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn pingap");
        wait_for_non_503(
            format!("{base_url}/api/meta/limits"),
            Duration::from_secs(60),
        )
        .await;

        let chrome = std::env::var("NAIL_E2E_CHROME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/usr/bin/chromium"));
        let config = BrowserConfig::builder()
            .chrome_executable(chrome)
            .no_sandbox()
            .new_headless_mode()
            .user_data_dir(
                std::env::temp_dir().join(format!("nail_chrome_{}", uuid::Uuid::now_v7())),
            )
            .build()
            .expect("browser config");
        let (browser, mut handler) = Browser::launch(config).await.expect("launch chromium");
        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        let page = browser.new_page(base_url.clone()).await.expect("open page");

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build http client");

        EndToEndBrowserContext {
            base_url,
            inbox,
            client,
            _browser: browser,
            page,
            backend,
            proxy,
            _conf_dir: conf_dir,
            _proxy_conf_dir: proxy_conf_dir,
            _handler_task: handler_task,
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
            if let Some(m) = mail {
                return m.body;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for SMTP mail to {to}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn body_text(&self) -> String {
        let value: String = self
            .page
            .evaluate("document.body.innerText")
            .await
            .expect("evaluate body.innerText")
            .into_value()
            .expect("innerText value");
        value
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

    pub(crate) async fn click_retry(&self, selector: &str, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(el) = self.page.find_element(selector).await {
                if el.click().await.is_ok() {
                    return;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to click {what} (selector {selector:?})"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) async fn type_retry(&self, selector: &str, text: &str, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(el) = self.page.find_element(selector).await {
                if el.click().await.is_ok() && el.type_str(text).await.is_ok() {
                    return;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to type into {what} (selector {selector:?})"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) async fn press_enter_retry(&self, selector: &str, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(el) = self.page.find_element(selector).await {
                if el.press_key("Enter").await.is_ok() {
                    return;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to submit {what} (selector {selector:?})"
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

    pub fn session_token_from_storage(storage: &serde_json::Value) -> Option<String> {
        storage
            .get("session_token")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                serde_json::from_str::<String>(s)
                    .ok()
                    .or_else(|| Some(s.to_string()))
            })
            .filter(|s| !s.is_empty())
    }

    pub async fn login_via_ui(&self, email: &str) -> String {
        let page = &self.page;
        page.goto(format!("{}/private/authenticate", self.base_url))
            .await
            .expect("goto authenticate");

        self.wait_for_text("send", 10).await;

        self.type_retry("input[type=\"email\"]", email, "email input")
            .await;
        self.click_retry("form button[type=\"submit\"]", "send button")
            .await;

        let mail_body = self.wait_for_mail(email, 10).await;
        let token = super::super::extract_token(&mail_body);

        self.type_retry("input[placeholder=\"token\"]", &token, "token input")
            .await;
        self.click_retry(
            "form:nth-of-type(2) button[type=\"submit\"]",
            "authenticate button",
        )
        .await;

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let storage = self.local_storage().await;
            if let Some(session) = Self::session_token_from_storage(&storage) {
                return session;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "session_token never landed in localStorage (storage so far: {storage})"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
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
        let form = reqwest::multipart::Form::new()
            .text("title", title.to_string())
            .text("summary", summary.to_string())
            .text("tags", tags.to_string())
            .text("version", version.to_string())
            .text("note", note.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(seed_pdf_variant(title)).file_name("seed.pdf"),
            );
        let resp = self
            .client
            .post(format!("{}/api/article", self.base_url))
            .header("nail-token", session)
            .multipart(form)
            .send()
            .await
            .expect("POST article");
        assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
        let json: serde_json::Value = resp.json().await.expect("article json");
        let article_id = json["article_id"].as_str().expect("article_id").to_string();
        let version_id = json["version_id"].as_str().expect("version_id").to_string();
        (article_id, version_id)
    }

    pub async fn set_file_input(&self, selector: &str, file_path: &Path) {
        let deadline = std::time::Instant::now() + Duration::from_secs(INTERACT_TIMEOUT_SECS);
        loop {
            if let Ok(el) = self.page.find_element(selector).await {
                let params =
                    chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams::builder()
                        .file(file_path.to_string_lossy().to_string())
                        .node_id(el.node_id)
                        .build()
                        .expect("build set-file params");
                if self.page.execute(params).await.is_ok() {
                    return;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never able to set file on {selector:?}"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn write_pdf_temp(&self, title: &str) -> PathBuf {
        let bytes = seed_pdf_variant(title);
        let path = std::env::temp_dir().join(format!(
            "nail_e2e_upload_{}_{}.pdf",
            uuid::Uuid::now_v7(),
            title
        ));
        std::fs::write(&path, bytes).expect("write temp pdf");
        path
    }

    pub async fn login_via_api(&self, email: &str) -> String {
        let challenge: common::pow::Challenge = self
            .client
            .get(format!("{}/api/authenticate/challenge", self.base_url))
            .send()
            .await
            .expect("GET challenge")
            .json()
            .await
            .expect("challenge json");
        let pow = common::pow::prove(common::pow::ProveInput {
            challenge,
            payload: email.to_string(),
        })
        .expect("proof");
        let resp = self
            .client
            .post(format!("{}/api/authenticate/pow", self.base_url))
            .json(&pow)
            .send()
            .await
            .expect("POST pow");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let body = self.wait_for_mail(email, 10).await;
        let token = super::super::extract_token(&body);
        let challenge2: common::pow::Challenge = self
            .client
            .get(format!("{}/api/authenticate/challenge", self.base_url))
            .send()
            .await
            .expect("GET challenge2")
            .json()
            .await
            .expect("challenge2 json");
        let pow2 = common::pow::prove(common::pow::ProveInput {
            challenge: challenge2,
            payload: token.clone(),
        })
        .expect("proof2");
        let resp2 = self
            .client
            .post(format!("{}/api/authenticate/token", self.base_url))
            .json(&common::request::TokenRequest { pow: pow2 })
            .send()
            .await
            .expect("POST token");
        assert_eq!(resp2.status(), reqwest::StatusCode::OK);
        let json: serde_json::Value = resp2.json().await.expect("token json");
        json["session_token"]
            .as_str()
            .expect("session_token")
            .to_string()
    }

    pub async fn set_session_token(&self, token: &str) {
        let js = format!("localStorage.setItem('session_token', JSON.stringify({token:?})); true");
        let _: bool = self
            .page
            .evaluate(js)
            .await
            .expect("set session token")
            .into_value()
            .expect("set session token value");
    }

    pub async fn search_article_id_by_title(&self, session: &str, title: &str) -> String {
        let resp = self
            .client
            .get(format!("{}/api/article/search?q={}", self.base_url, title))
            .header("nail-token", session)
            .send()
            .await
            .expect("search by title");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let json: serde_json::Value = resp.json().await.expect("search json");
        json["article_list"]
            .as_array()
            .and_then(|l| l.first())
            .and_then(|a| a.get("id").and_then(|v| v.as_str()))
            .expect("article present in search result")
            .to_string()
    }

    pub async fn first_comment_id(&self, session: &str, version_id: &str, content: &str) -> String {
        let resp = self
            .client
            .get(format!(
                "{}/api/version/{}/comments",
                self.base_url, version_id
            ))
            .header("nail-token", session)
            .send()
            .await
            .expect("get comments");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let json: serde_json::Value = resp.json().await.expect("comments json");
        json["comments"]
            .as_array()
            .and_then(|l| {
                l.iter()
                    .find(|c| c.get("content").and_then(|v| v.as_str()) == Some(content))
            })
            .and_then(|c| c.get("id").and_then(|v| v.as_str()))
            .unwrap_or_else(|| panic!("comment with content {content:?} not found"))
            .to_string()
    }

    pub async fn first_version_id(&self, session: &str, article_id: &str) -> String {
        let resp = self
            .client
            .get(format!(
                "{}/api/article/{}/version?page=1",
                self.base_url, article_id
            ))
            .header("nail-token", session)
            .send()
            .await
            .expect("get version list");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let json: serde_json::Value = resp.json().await.expect("versions json");
        json["version_list"]
            .as_array()
            .and_then(|l| l.first())
            .and_then(|v| v.get("id").and_then(|x| x.as_str()))
            .expect("version present in list")
            .to_string()
    }
}

impl Drop for EndToEndBrowserContext {
    fn drop(&mut self) {
        let _ = self.proxy.kill();
        let _ = self.backend.kill();
        let _ = std::fs::remove_dir_all(&self._conf_dir);
        let _ = std::fs::remove_dir_all(&self._proxy_conf_dir);
    }
}

fn seed_pdf_variant(variant: &str) -> Vec<u8> {
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
fn write_patched_server_toml(conf_dir: &Path, prod_server_toml: &Path) {
    let original = std::fs::read_to_string(prod_server_toml).expect("read server.toml");
    let patches: [(&str, &str); 4] = [
        (
            "db_path = \"/home/qkun/nail/data/surrealkv\"",
            "db_path = \"memory\"",
        ),
        (
            "db_namespace = \"prod_ns\"",
            "db_namespace = \"e2e_browser_ns\"",
        ),
        (
            "db_database = \"prod_db\"",
            "db_database = \"e2e_browser_db\"",
        ),
        (
            "pow_difficulty_iterations = 8192",
            &format!("pow_difficulty_iterations = {TEST_POW_DIFFICULTY}"),
        ),
    ];
    let mut patched = original.clone();
    for (from, to) in patches {
        assert!(
            patched.contains(from),
            "server.toml patch anchor missing: {from}"
        );
        patched = patched.replace(from, to);
    }
    let pdf_dir = std::env::temp_dir().join(format!("nail_browser_pdf_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&pdf_dir).expect("create pdf dir");
    patched = patched.replace(
        "pdf_storage_path = \"/home/qkun/nail/data/pdf\"",
        &format!("pdf_storage_path = \"{}\"", pdf_dir.display()),
    );
    std::fs::write(conf_dir.join("server.toml"), patched).expect("write server.toml");
}

fn write_sink_smtp_toml(conf_dir: &Path, sink_port: u16) {
    let toml = format!(
        "host = \"127.0.0.1\"\n\
         port = {sink_port}\n\
         username = \"\"\n\
         password = \"\"\n\
         from_email = \"nail-test@localhost\"\n\
         from_name = \"nail\"\n\
         timeout_secs = 10\n\
         wall_clock_timeout_secs = 30\n\
         starttls = false\n"
    );
    std::fs::write(conf_dir.join("smtp.toml"), toml).expect("write smtp.toml");
}

fn write_patched_proxy_conf(prod_proxy_conf: PathBuf) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nail_pingap_conf_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).expect("create temp pingap conf dir");
    for name in ["locations.toml", "plugins.toml", "servers.toml"] {
        std::fs::copy(prod_proxy_conf.join(name), dir.join(name)).expect("copy proxy conf file");
    }
    let upstream = std::fs::read_to_string(prod_proxy_conf.join("upstreams.toml"))
        .expect("read upstreams.toml");
    let patched = upstream.replace(
        "addrs = [\"localhost:3000\"]\ndiscovery = \"static\"",
        "addrs = [\"localhost:3000\"]\ndiscovery = \"static\"\n\
         health_check = \"http://localhost:3000/meta/limits?check_frequency=1&consecutive_success=1&consecutive_failure=1\"",
    );
    assert!(
        patched.contains("health_check"),
        "upstreams.toml patch anchor missing"
    );
    std::fs::write(dir.join("upstreams.toml"), patched).expect("write upstreams.toml");
    dir
}

async fn wait_for_http_ok(url: &str, timeout: Duration) {
    let client = reqwest::Client::new();
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(resp) = client.get(url).send().await
            && resp.status().is_success()
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "backend never healthy at {url}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn wait_for_non_503(url: String, timeout: Duration) {
    let client = reqwest::Client::new();
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(resp) = client.get(&url).send().await
            && resp.status() != reqwest::StatusCode::SERVICE_UNAVAILABLE
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "proxy never healthy at {url}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
