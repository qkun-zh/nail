use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_user_creation_ttl_seconds")]
    pub user_creation_ttl_seconds: u64,
    #[serde(default = "default_session_ttl_seconds")]
    pub session_ttl_seconds: u64,
    #[serde(default = "default_email_update_ttl_seconds")]
    pub email_update_ttl_seconds: u64,
    #[serde(default = "default_user_deletion_ttl_seconds")]
    pub user_deletion_ttl_seconds: u64,
    #[serde(default = "default_challenge_ttl_seconds")]
    pub challenge_ttl_seconds: u64,
    #[serde(default = "default_download_ttl_seconds")]
    pub download_ttl_seconds: u64,
    #[serde(default = "default_cache_capacity")]
    pub cache_capacity: u64,
}

fn default_user_creation_ttl_seconds() -> u64 {
    8000
}

fn default_session_ttl_seconds() -> u64 {
    8000
}

fn default_email_update_ttl_seconds() -> u64 {
    8000
}

fn default_user_deletion_ttl_seconds() -> u64 {
    8000
}

fn default_challenge_ttl_seconds() -> u64 {
    300
}

fn default_download_ttl_seconds() -> u64 {
    60
}

fn default_cache_capacity() -> u64 {
    100_000
}

impl CacheConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or fails validation.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("failed to read {}", path.as_ref().display()))?;
        let config: Self = toml::from_str(&content).context("failed to parse cache config")?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any field is zero.
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("user_creation_ttl_seconds", self.user_creation_ttl_seconds),
            ("session_ttl_seconds", self.session_ttl_seconds),
            ("email_update_ttl_seconds", self.email_update_ttl_seconds),
            ("user_deletion_ttl_seconds", self.user_deletion_ttl_seconds),
            ("challenge_ttl_seconds", self.challenge_ttl_seconds),
            ("download_ttl_seconds", self.download_ttl_seconds),
            ("cache_capacity", self.cache_capacity),
        ] {
            if value == 0 {
                anyhow::bail!("cache: {name} must be > 0");
            }
        }
        Ok(())
    }
}
