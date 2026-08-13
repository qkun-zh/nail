use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub api_base_url: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let config = toml::from_str::<Self>(include_str!("../../../conf/front/config.toml"))
            .expect("failed to parse conf/front/config.toml (embedded at compile time via include_str!); fix the file and rebuild the WASM frontend");
        config.validate_api_base_url();
        config
    }

    fn validate_api_base_url(&self) {
        let base = &self.api_base_url;
        let valid = base.is_empty() || base.starts_with("http://") || base.starts_with("https://");
        if !valid {
            panic!(
                "conf/front/config.toml: api_base_url must be empty (same-origin deployment) \
                 or start with http:// or https://, got: {base:?}"
            );
        }
    }
}
