#[derive(Clone, Debug, serde::Deserialize)]
pub struct EmailConfig {
    pub allowed_domains: Vec<String>,
}
