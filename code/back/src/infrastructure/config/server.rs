use anyhow::bail;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub db_path: String,
    pub search_index_path: String,
    pub pdf_storage_path: String,
    pub pow_difficulty_iterations: u64,
    pub token_ttl_seconds: u64,
    pub session_ttl_seconds: u64,
    pub challenge_ttl_seconds: u64,
    pub download_token_ttl_seconds: u64,
    pub token_cache_capacity: u64,
    pub email_cooldown_seconds: u64,
    pub timezone_offset_seconds: i32,
    pub user_zero_email: String,
    pub max_pdf_size_bytes: u64,
    pub max_tags_per_article: usize,
    pub max_title_chars: u64,
    pub max_summary_chars: u64,
    pub max_version_note_chars: u64,
    pub max_text_field_bytes: u64,
    pub max_search_query_chars: u64,
    pub log_dir: String,
    pub log_retention_days: u64,
    pub log_max_file_count: usize,
    pub log_prune_interval_secs: u64,
    #[serde(default = "default_log_filter")]
    pub log_filter: String,
}

fn default_log_filter() -> String {
    "warn,nail_back=info,common=info".to_string()
}

impl ServerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.listen_addr.is_empty() {
            bail!("config: listen_addr must not be empty");
        }
        if self.db_path.is_empty() || self.log_dir.is_empty() {
            bail!("config: db_path / log_dir must not be empty");
        }
        if self.search_index_path.is_empty() || self.pdf_storage_path.is_empty() {
            bail!("config: search_index_path / pdf_storage_path must not be empty");
        }
        if self.max_text_field_bytes > self.max_pdf_size_bytes {
            bail!("config: max_text_field_bytes must not exceed max_pdf_size_bytes");
        }
        if self.log_filter.trim().is_empty() {
            bail!("config: log_filter must not be empty");
        }
        if self.user_zero_email.trim().is_empty() || !self.user_zero_email.contains('@') {
            bail!("config: user_zero_email must be a valid email address");
        }
        if !(1..=10_000).contains(&self.pow_difficulty_iterations) {
            bail!("config: pow_difficulty_iterations must be in 1..=10_000");
        }
        if self.timezone_offset_seconds % 60 != 0
            || !(-86_340..=86_340).contains(&self.timezone_offset_seconds)
        {
            bail!("config: timezone_offset_seconds must be a whole number of minutes within -23:59..=+23:59");
        }
        for (name, value) in [
            ("token_ttl_seconds", self.token_ttl_seconds),
            ("session_ttl_seconds", self.session_ttl_seconds),
            ("challenge_ttl_seconds", self.challenge_ttl_seconds),
            ("download_token_ttl_seconds", self.download_token_ttl_seconds),
            ("token_cache_capacity", self.token_cache_capacity),
            ("email_cooldown_seconds", self.email_cooldown_seconds),
            ("log_prune_interval_secs", self.log_prune_interval_secs),
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
            ("max_version_note_chars", self.max_version_note_chars),
            ("max_text_field_bytes", self.max_text_field_bytes),
            ("max_search_query_chars", self.max_search_query_chars),
        ] {
            if value == 0 {
                bail!("config: {name} must be > 0");
            }
        }
        Ok(())
    }
}
