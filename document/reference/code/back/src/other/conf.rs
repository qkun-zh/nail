
use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub db_path: String,
    pub db_namespace: String,
    pub db_database: String,
    pub search_index_path: String,
    pub pow_difficulty_iterations: u64,
    pub token_ttl_seconds: u64,
    pub session_ttl_seconds: u64,
    pub pdf_storage_path: String,
    pub max_pdf_size_bytes: u64,
    pub download_token_ttl_seconds: u64,
    pub max_tags_per_article: usize,
    pub max_title_chars: u64,
    pub max_summary_chars: u64,
    pub search_page_size: u64,
    pub max_search_page_size: u64,
    pub max_search_query_chars: u64,
    pub email_cooldown_seconds: u64,
    pub max_page: u64,
    pub max_search_pages: u64,
    pub max_id_filter_count: usize,
    pub max_comment_body_chars: u64,
    pub max_version_note_chars: u64,
    pub max_comment_tree_depth: u64,
    pub max_text_field_bytes: u64,
    pub challenge_ttl_seconds: u64,
    pub token_cache_capacity: u64,
    pub log_prune_interval_secs: u64,
    pub log_dir: String,
    pub log_retention_days: u64,
    pub log_max_file_count: usize,
    #[serde(default = "default_log_filter")]
    pub log_filter: String,
    pub user_zero_email: String,
}

impl ServerConfig {
    fn validate(&self) -> Result<()> {
        if self.listen_addr.is_empty() {
            bail!("config: listen_addr must not be empty");
        }
        if self.db_path.is_empty() || self.pdf_storage_path.is_empty() || self.log_dir.is_empty() {
            bail!("config: db_path / pdf_storage_path / log_dir must not be empty");
        }
        if self.search_index_path.is_empty() {
            bail!("config: search_index_path must not be empty");
        }
        if self.log_filter.trim().is_empty() {
            bail!("config: log_filter must not be empty");
        }
        if self.user_zero_email.trim().is_empty() {
            bail!("config: user_zero_email must not be empty");
        }
        if !self.user_zero_email.contains('@') {
            bail!("config: user_zero_email must be a valid email address");
        }
        if self.db_namespace.is_empty() || self.db_database.is_empty() {
            bail!("config: db_namespace / db_database must not be empty");
        }
        if !(1..=10_000).contains(&self.pow_difficulty_iterations) {
            bail!("config: pow_difficulty_iterations must be in 1..=10_000");
        }
        for (name, value) in [
            ("token_ttl_seconds", self.token_ttl_seconds),
            ("session_ttl_seconds", self.session_ttl_seconds),
            (
                "download_token_ttl_seconds",
                self.download_token_ttl_seconds,
            ),
        ] {
            if value == 0 {
                bail!("config: {name} must be > 0");
            }
        }
        for (name, value) in [
            ("max_pdf_size_bytes", self.max_pdf_size_bytes),
            ("max_tags_per_article", self.max_tags_per_article as u64),
            ("max_title_chars", self.max_title_chars),
            ("max_summary_chars", self.max_summary_chars),
            ("search_page_size", self.search_page_size),
            ("max_search_page_size", self.max_search_page_size),
            ("max_search_query_chars", self.max_search_query_chars),
            ("email_cooldown_seconds", self.email_cooldown_seconds),
            ("max_page", self.max_page),
            ("max_search_pages", self.max_search_pages),
            ("max_id_filter_count", self.max_id_filter_count as u64),
            ("max_comment_body_chars", self.max_comment_body_chars),
            ("max_version_note_chars", self.max_version_note_chars),
            ("max_comment_tree_depth", self.max_comment_tree_depth),
            ("max_text_field_bytes", self.max_text_field_bytes),
            ("challenge_ttl_seconds", self.challenge_ttl_seconds),
            ("token_cache_capacity", self.token_cache_capacity),
            ("log_prune_interval_secs", self.log_prune_interval_secs),
        ] {
            if value == 0 {
                bail!("config: {name} must be > 0");
            }
        }
        if self.search_page_size > self.max_search_page_size {
            bail!("config: search_page_size must not exceed max_search_page_size");
        }
        if self.max_text_field_bytes > self.max_pdf_size_bytes {
            bail!("config: max_text_field_bytes must not exceed max_pdf_size_bytes");
        }
        Ok(())
    }
}

fn default_log_filter() -> String {
    "warn,nail_back=info,common=info".to_string()
}

impl SmtpConfig {
    fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            bail!("config: smtp.host must not be empty");
        }
        if self.port == 0 {
            bail!("config: smtp.port must not be 0");
        }
        if self.timeout_secs == 0 {
            bail!("config: smtp.timeout_secs must be > 0");
        }
        if self.wall_clock_timeout_secs < self.timeout_secs {
            bail!("config: smtp.wall_clock_timeout_secs must be >= timeout_secs");
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    #[serde(default = "default_smtp_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_smtp_wall_clock_timeout_secs")]
    pub wall_clock_timeout_secs: u64,
    #[serde(default = "default_smtp_starttls")]
    pub starttls: bool,
}

fn default_smtp_starttls() -> bool {
    true
}

fn default_smtp_timeout_secs() -> u64 {
    10
}

fn default_smtp_wall_clock_timeout_secs() -> u64 {
    30
}

impl std::fmt::Debug for SmtpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmtpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"***")
            .field("from_email", &self.from_email)
            .field("from_name", &self.from_name)
            .field("timeout_secs", &self.timeout_secs)
            .field("wall_clock_timeout_secs", &self.wall_clock_timeout_secs)
            .field("starttls", &self.starttls)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct EmailConfig {
    pub allowed_domains: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub smtp: SmtpConfig,
    pub email: EmailConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let dir = config_dir()?;
        Self::load_from(&dir)
    }

    pub fn load_from(dir: &Path) -> Result<Self> {
        let server: ServerConfig = toml::from_str(&read_config(dir, "server.toml")?)?;
        server.validate()?;
        let smtp: SmtpConfig = toml::from_str(&read_config(dir, "smtp.toml")?)?;
        smtp.validate()?;
        let mut email: EmailConfig = toml::from_str(&read_config(dir, "email.toml")?)?;
        email.allowed_domains = email
            .allowed_domains
            .into_iter()
            .map(|d| d.trim().trim_start_matches('@').to_lowercase())
            .filter(|d| !d.is_empty())
            .collect();

        Ok(Self {
            server,
            smtp,
            email,
        })
    }
}

fn read_config(dir: &Path, name: &str) -> Result<String> {
    let path = dir.join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))
}

fn config_dir() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CONF_DIR") {
        let path = PathBuf::from(dir);
        if path.join("server.toml").is_file() {
            return Ok(path);
        }
        bail!("CONF_DIR {:?} does not contain server.toml", path.display());
    }
    let cwd = env::current_dir().context("failed to read current directory")?;
    for ancestor in cwd.ancestors() {
        let candidate = ancestor.join("conf/back");
        if candidate.join("server.toml").is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "cannot locate conf/back from {} (set CONF_DIR to override)",
        cwd.display()
    )
}
