use anyhow::bail;

#[derive(Clone, serde::Deserialize)]
pub struct SmtpConfig {
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

impl SmtpConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
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
