pub mod logging;
pub mod server;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use logging::LoggingConfig;
use server::ServerConfig;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub emailer: emailer::EmailerConfig,
    pub cache: cache::CacheConfig,
    pub email_allowed_domains: Vec<String>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let directory = config_directory()?;
        Self::load_from(&directory)
    }

    pub fn load_from(directory: &Path) -> Result<Self> {
        let server_content = read_config(directory, "server.toml")?;
        let server: ServerConfig = toml::from_str(&server_content)?;
        server.validate()?;
        let logging_section = extract_section(&server_content, "logging");
        let logging: LoggingConfig = toml::from_str(&logging_section)?;
        logging.validate()?;
        let emailer = emailer::EmailerConfig::load(directory.join("emailer.toml"))?;
        let cache = cache::CacheConfig::load(directory.join("cache.toml"))?;
        let mut email_allowed_domains: Vec<String> =
            toml::from_str::<EmailDomainConfig>(&read_config(directory, "email.toml")?)?
                .allowed_domains;
        email_allowed_domains = email_allowed_domains
            .into_iter()
            .map(|domain| domain.trim().trim_start_matches('@').to_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect();
        Ok(Self {
            server,
            logging,
            emailer,
            cache,
            email_allowed_domains,
        })
    }

    pub fn db_path(&self) -> &str {
        &self.server.db_path
    }

    pub fn search_index_path(&self) -> &str {
        &self.server.search_index_path
    }

    pub fn pdf_storage_path(&self) -> &str {
        &self.server.pdf_storage_path
    }

    pub fn user_zero_email(&self) -> &str {
        &self.server.user_zero_email
    }
}

#[derive(serde::Deserialize)]
struct EmailDomainConfig {
    allowed_domains: Vec<String>,
}

fn read_config(directory: &Path, name: &str) -> Result<String> {
    let path = directory.join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))
}

fn extract_section(content: &str, section: &str) -> String {
    let header = format!("[{section}]");
    let Some(start) = content.find(&header) else {
        return String::new();
    };
    let after_header = start + header.len();
    let rest = &content[after_header..];
    let end = rest.find("\n[").unwrap_or(rest.len());
    format!("{}\n{}", &rest[..end], header)
}

fn config_directory() -> Result<PathBuf> {
    if let Ok(directory) = env::var("CONF_DIR") {
        let path = PathBuf::from(directory);
        if path.join("server.toml").is_file() {
            return Ok(path);
        }
        bail!("CONF_DIR {} does not contain server.toml", path.display());
    }
    let current = env::current_dir().context("failed to read current directory")?;
    for ancestor in current.ancestors() {
        let candidate = ancestor.join("configuration");
        if candidate.join("server.toml").is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "cannot locate configuration/ from {} (set CONF_DIR to override)",
        current.display()
    )
}
