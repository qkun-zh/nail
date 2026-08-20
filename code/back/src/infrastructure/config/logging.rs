use anyhow::bail;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct LoggingConfig {
    pub dir: String,
    pub retention_days: u64,
    #[serde(default = "default_filter")]
    pub filter: String,
}

fn default_filter() -> String {
    "warn,nail_back=info,common=info".to_string()
}

impl LoggingConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.dir.trim().is_empty() {
            bail!("logging config: dir must not be empty");
        }
        if self.filter.trim().is_empty() {
            bail!("logging config: filter must not be empty");
        }
        Ok(())
    }
}
