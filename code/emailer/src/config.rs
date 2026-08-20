use std::fmt;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct EmailerConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_wall_clock_timeout_secs")]
    pub wall_clock_timeout_secs: u64,
    #[serde(default = "default_starttls")]
    pub starttls: bool,
    #[serde(default = "default_per_recipient_cooldown_secs")]
    pub per_recipient_cooldown_secs: u64,
    #[serde(default = "default_global_max_per_minute")]
    pub global_max_per_minute: u32,
}

fn default_timeout_secs() -> u64 {
    10
}

fn default_wall_clock_timeout_secs() -> u64 {
    30
}

fn default_starttls() -> bool {
    true
}

fn default_per_recipient_cooldown_secs() -> u64 {
    60
}

fn default_global_max_per_minute() -> u32 {
    30
}

impl EmailerConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or fails validation.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("failed to read {}", path.as_ref().display()))?;
        let config: Self = toml::from_str(&content).context("failed to parse emailer config")?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any field is invalid or inconsistent.
    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            anyhow::bail!("emailer: host must not be empty");
        }
        if self.port == 0 {
            anyhow::bail!("emailer: port must not be 0");
        }
        if self.timeout_secs == 0 {
            anyhow::bail!("emailer: timeout_secs must be > 0");
        }
        if self.wall_clock_timeout_secs < self.timeout_secs {
            anyhow::bail!("emailer: wall_clock_timeout_secs must be >= timeout_secs");
        }
        if self.global_max_per_minute == 0 {
            anyhow::bail!("emailer: global_max_per_minute must be > 0");
        }
        Ok(())
    }
}

impl fmt::Debug for EmailerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmailerConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"***")
            .field("from_email", &self.from_email)
            .field("from_name", &self.from_name)
            .field("timeout_secs", &self.timeout_secs)
            .field("wall_clock_timeout_secs", &self.wall_clock_timeout_secs)
            .field("starttls", &self.starttls)
            .field(
                "per_recipient_cooldown_secs",
                &self.per_recipient_cooldown_secs,
            )
            .field("global_max_per_minute", &self.global_max_per_minute)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let toml_str = r#"
host = "smtp.example.com"
port = 587
username = "user"
password = "pass"
from_email = "noreply@example.com"
from_name = "test"
"#;
        let config: EmailerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.wall_clock_timeout_secs, 30);
        assert!(config.starttls);
        assert_eq!(config.per_recipient_cooldown_secs, 60);
        assert_eq!(config.global_max_per_minute, 30);
    }

    #[test]
    fn validate_empty_host() {
        let config = EmailerConfig {
            host: String::new(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: String::new(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: true,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 30,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_zero_port() {
        let config = EmailerConfig {
            host: "smtp.example.com".to_string(),
            port: 0,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: String::new(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: true,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 30,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_wall_clock_less_than_timeout() {
        let config = EmailerConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: String::new(),
            timeout_secs: 30,
            wall_clock_timeout_secs: 10,
            starttls: true,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 30,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_zero_global_max() {
        let config = EmailerConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: String::new(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: true,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn debug_masks_password() {
        let config = EmailerConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "user".to_string(),
            password: "secret123".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: "test".to_string(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: true,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 30,
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("secret123"));
        assert!(debug.contains("***"));
    }
}
