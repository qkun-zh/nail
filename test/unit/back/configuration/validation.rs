use std::path::PathBuf;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::config::server::ServerConfig;
use crate::infrastructure::config::smtp::SmtpConfig;

use super::context::test_config;

fn valid_server() -> ServerConfig {
    test_config().server
}

fn assert_invalid_server(mutate: impl FnOnce(&mut ServerConfig)) {
    let mut server = valid_server();
    mutate(&mut server);
    assert!(server.validate().is_err());
}

#[test]
fn server_config_accepts_a_valid_shape() {
    assert!(valid_server().validate().is_ok());
}

#[test]
fn server_config_rejects_empty_paths() {
    assert_invalid_server(|server| server.listen_addr.clear());
    assert_invalid_server(|server| server.db_path.clear());
    assert_invalid_server(|server| server.log_dir.clear());
}

#[test]
fn server_config_rejects_an_invalid_difficulty() {
    assert_invalid_server(|server| server.pow_difficulty_iterations = 0);
    assert_invalid_server(|server| server.pow_difficulty_iterations = 10_001);
}

#[test]
fn server_config_rejects_zero_ttls_and_capacity() {
    assert_invalid_server(|server| server.token_ttl_seconds = 0);
    assert_invalid_server(|server| server.session_ttl_seconds = 0);
    assert_invalid_server(|server| server.challenge_ttl_seconds = 0);
    assert_invalid_server(|server| server.token_cache_capacity = 0);
    assert_invalid_server(|server| server.email_cooldown_seconds = 0);
    assert_invalid_server(|server| server.log_prune_interval_secs = 0);
}

#[test]
fn server_config_rejects_an_invalid_timezone_offset() {
    assert_invalid_server(|server| server.timezone_offset_seconds = 90);
    assert_invalid_server(|server| server.timezone_offset_seconds = 86_400);
    assert_invalid_server(|server| server.timezone_offset_seconds = -86_400);
}

#[test]
fn server_config_rejects_an_invalid_user_zero_email() {
    assert_invalid_server(|server| server.user_zero_email.clear());
    assert_invalid_server(|server| server.user_zero_email = "no-at-sign".to_string());
}

#[test]
fn smtp_config_rejects_invalid_shapes() {
    let valid = test_config().smtp;
    let mut empty_host = valid.clone();
    empty_host.host.clear();
    assert!(empty_host.validate().is_err());

    let mut zero_port = valid.clone();
    zero_port.port = 0;
    assert!(zero_port.validate().is_err());

    let mut zero_timeout = valid.clone();
    zero_timeout.timeout_secs = 0;
    assert!(zero_timeout.validate().is_err());

    let mut bad_wall_clock = valid.clone();
    bad_wall_clock.timeout_secs = 30;
    bad_wall_clock.wall_clock_timeout_secs = 10;
    assert!(bad_wall_clock.validate().is_err());

    assert!(SmtpConfig {
        host: "localhost".to_string(),
        port: 25,
        username: String::new(),
        password: String::new(),
        from_email: "noreply@example.com".to_string(),
        from_name: "nail".to_string(),
        timeout_secs: 10,
        wall_clock_timeout_secs: 30,
        starttls: false,
    }
    .validate()
    .is_ok());
}

#[test]
fn load_from_parses_tomls_and_normalizes_domains() {
    let directory = std::env::temp_dir().join(format!("nail_config_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).expect("create dir");
    write_configs(&directory);

    let config = AppConfig::load_from(&directory).expect("load");
    assert_eq!(config.server.pow_difficulty_iterations, 8192);
    assert_eq!(config.email.allowed_domains, vec!["qq.com", "example.com"]);

    let _ = std::fs::remove_dir_all(&directory);
}

fn write_configs(directory: &PathBuf) {
    let server = r#"
listen_addr = "127.0.0.1:3000"
db_path = "memory"
pow_difficulty_iterations = 8192
token_ttl_seconds = 8000
session_ttl_seconds = 8000
challenge_ttl_seconds = 300
token_cache_capacity = 100000
email_cooldown_seconds = 60
timezone_offset_seconds = 28800
user_zero_email = "admin@example.com"
log_dir = "log/back"
log_retention_days = 7
log_max_file_count = 10080
log_prune_interval_secs = 1800
"#;
    let smtp = r#"
host = "localhost"
port = 25
username = "u"
password = "p"
from_email = "noreply@example.com"
from_name = "nail"
"#;
    let email = r#"
allowed_domains = ["qq.com", "@Example.com"]
"#;
    std::fs::write(directory.join("server.toml"), server).expect("server.toml");
    std::fs::write(directory.join("smtp.toml"), smtp).expect("smtp.toml");
    std::fs::write(directory.join("email.toml"), email).expect("email.toml");
}
