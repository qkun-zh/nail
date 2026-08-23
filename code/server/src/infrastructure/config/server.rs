use anyhow::bail;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub db_path: String,
    pub search_index_path: String,
    pub pdf_storage_path: String,
    pub pow_difficulty_iterations: u64,
    pub email_cooldown_seconds: u64,
    pub user_zero_email: String,
    pub max_pdf_size_bytes: u64,
    pub max_tags_per_article: usize,
    pub max_title_chars: u64,
    pub max_summary_chars: u64,
    pub max_comment_body_chars: u64,
    pub max_version_note_chars: u64,
    pub max_text_field_bytes: u64,
    pub max_search_query_chars: u64,
    pub search_page_size: u64,
    pub max_search_pages: u64,
}

impl ServerConfig {
    pub fn max_request_body_bytes(&self) -> u64 {
        self.max_pdf_size_bytes
            .saturating_add(self.max_text_field_bytes.saturating_mul(5))
            .saturating_add(64 * 1024)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.listen_addr.is_empty() {
            bail!("config: listen_addr must not be empty");
        }
        if self.db_path.is_empty() {
            bail!("config: db_path must not be empty");
        }
        if self.search_index_path.is_empty() || self.pdf_storage_path.is_empty() {
            bail!("config: search_index_path / pdf_storage_path must not be empty");
        }
        if self.max_text_field_bytes > self.max_pdf_size_bytes {
            bail!("config: max_text_field_bytes must not exceed max_pdf_size_bytes");
        }
        if self.user_zero_email.trim().is_empty()
            || email_address::EmailAddress::parse_with_options(
                &self.user_zero_email,
                email_address::Options::default(),
            )
            .is_err()
        {
            bail!("config: user_zero_email must be a valid email address");
        }
        if !(1..=10_000).contains(&self.pow_difficulty_iterations) {
            bail!("config: pow_difficulty_iterations must be in 1..=10_000");
        }
        for (name, value) in [("email_cooldown_seconds", self.email_cooldown_seconds)] {
            if value == 0 {
                bail!("config: {name} must be > 0");
            }
        }
        for (name, value) in [
            ("max_pdf_size_bytes", self.max_pdf_size_bytes),
            ("max_tags_per_article", self.max_tags_per_article as u64),
            ("max_title_chars", self.max_title_chars),
            ("max_summary_chars", self.max_summary_chars),
            ("max_comment_body_chars", self.max_comment_body_chars),
            ("max_version_note_chars", self.max_version_note_chars),
            ("max_text_field_bytes", self.max_text_field_bytes),
            ("max_search_query_chars", self.max_search_query_chars),
            ("search_page_size", self.search_page_size),
            ("max_search_pages", self.max_search_pages),
        ] {
            if value == 0 {
                bail!("config: {name} must be > 0");
            }
        }
        Ok(())
    }
}
