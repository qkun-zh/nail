pub mod email;
pub mod server;
pub mod smtp;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use email::EmailConfig;
use server::ServerConfig;
use smtp::SmtpConfig;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub smtp: SmtpConfig,
    pub email: EmailConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let directory = config_directory()?;
        Self::load_from(&directory)
    }

    pub fn load_from(directory: &Path) -> Result<Self> {
        let server: ServerConfig = toml::from_str(&read_config(directory, "server.toml")?)?;
        server.validate()?;
        let smtp: SmtpConfig = toml::from_str(&read_config(directory, "smtp.toml")?)?;
        smtp.validate()?;
        let mut email: EmailConfig = toml::from_str(&read_config(directory, "email.toml")?)?;
        email.allowed_domains = email
            .allowed_domains
            .into_iter()
            .map(|domain| domain.trim().trim_start_matches('@').to_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect();
        Ok(Self {
            server,
            smtp,
            email,
        })
    }
}

fn read_config(directory: &Path, name: &str) -> Result<String> {
    let path = directory.join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))
}

fn config_directory() -> Result<PathBuf> {
    if let Ok(directory) = env::var("CONF_DIR") {
        let path = PathBuf::from(directory);
        if path.join("server.toml").is_file() {
            return Ok(path);
        }
        bail!("CONF_DIR {path:?} does not contain server.toml");
    }
    let current = env::current_dir().context("failed to read current directory")?;
    for ancestor in current.ancestors() {
        let candidate = ancestor.join("configuration");
        if candidate.join("server.toml").is_file() {
            return Ok(candidate);
        }
    }
    bail!("cannot locate configuration/ from {current:?} (set CONF_DIR to override)")
}
