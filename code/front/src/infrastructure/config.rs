use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct FrontendConfig {
    #[serde(default)]
    api_base_url: String,
}

const EMBEDDED_TOML: &str = include_str!("../../../../configuration/front.toml");

pub fn validate_api_base_url(raw: &str) -> Result<String, String> {
    let normalized = raw.trim_end_matches('/');
    if normalized.is_empty() {
        return Ok(String::new());
    }
    if normalized.starts_with("http://") || normalized.starts_with("https://") {
        return Ok(normalized.to_string());
    }
    Err(format!(
        "api_base_url must be empty (same-origin) or start with http:// or https://, got {raw:?}"
    ))
}

fn load() -> &'static FrontendConfig {
    static CONFIG: OnceLock<FrontendConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let parsed: FrontendConfig = match toml::from_str(EMBEDDED_TOML) {
            Ok(config) => config,
            Err(error) => {
                web_sys::console::error_1(
                    &format!("frontend config: failed to parse embedded configuration/front.toml: {error}")
                        .into(),
                );
                return FrontendConfig::default();
            }
        };
        let base = match validate_api_base_url(&parsed.api_base_url) {
            Ok(url) => url,
            Err(reason) => {
                web_sys::console::error_1(&format!("frontend config: {reason}").into());
                return FrontendConfig::default();
            }
        };
        FrontendConfig { api_base_url: base }
    })
}

pub fn api_base_url() -> &'static str {
    &load().api_base_url
}

#[cfg(test)]
#[path = "../../../../test/unit/front/infrastructure/config/tests.rs"]
mod tests;
