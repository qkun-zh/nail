use std::path::Path;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::config::server::ServerConfig;

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
}

#[test]
fn server_config_rejects_an_invalid_difficulty() {
    assert_invalid_server(|server| server.pow_difficulty_iterations = 0);
    assert_invalid_server(|server| server.pow_difficulty_iterations = 10_001);
}

#[test]
fn server_config_rejects_zero_cooldown() {
    assert_invalid_server(|server| server.email_cooldown_seconds = 0);
}

#[test]
fn server_config_rejects_empty_search_and_pdf_paths() {
    assert_invalid_server(|server| server.search_index_path.clear());
    assert_invalid_server(|server| server.pdf_storage_path.clear());
}

#[test]
fn server_config_rejects_zero_content_limits() {
    assert_invalid_server(|server| server.max_pdf_size_bytes = 0);
    assert_invalid_server(|server| server.max_tags_per_article = 0);
    assert_invalid_server(|server| server.max_title_chars = 0);
    assert_invalid_server(|server| server.max_summary_chars = 0);
    assert_invalid_server(|server| server.max_comment_body_chars = 0);
    assert_invalid_server(|server| server.max_version_note_chars = 0);
    assert_invalid_server(|server| server.max_text_field_bytes = 0);
    assert_invalid_server(|server| server.max_search_query_chars = 0);
}

#[test]
fn server_config_rejects_text_field_bytes_exceeding_pdf_size() {
    assert_invalid_server(|server| {
        server.max_pdf_size_bytes = 100;
        server.max_text_field_bytes = 101;
    });
}

#[test]
fn server_config_rejects_zero_pagination_limits() {
    assert_invalid_server(|server| server.search_page_size = 0);
    assert_invalid_server(|server| server.max_search_pages = 0);
    assert_invalid_server(|server| server.tag_page_size = 0);
}

#[test]
fn server_config_rejects_an_invalid_user_zero_email() {
    assert_invalid_server(|server| server.user_zero_email.clear());
    assert_invalid_server(|server| server.user_zero_email = "no-at-sign".to_string());
}

#[test]
fn emailer_config_rejects_invalid_shapes() {
    let valid = test_config().emailer;
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

    let mut zero_global = valid.clone();
    zero_global.global_max_per_minute = 0;
    assert!(zero_global.validate().is_err());

    assert!(
        emailer::EmailerConfig {
            host: "localhost".to_string(),
            port: 25,
            username: String::new(),
            password: String::new(),
            from_email: "noreply@example.com".to_string(),
            from_name: "nail".to_string(),
            timeout_secs: 10,
            wall_clock_timeout_secs: 30,
            starttls: false,
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 30,
        }
        .validate()
        .is_ok()
    );
}

#[test]
fn load_from_parses_tomls_and_normalizes_domains() {
    let directory = std::env::temp_dir().join(format!("nail_config_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).expect("create dir");
    write_configs(&directory);

    let config = AppConfig::load_from(&directory).unwrap_or_else(|e| {
        let content = std::fs::read_to_string(directory.join("server.toml")).unwrap_or_default();
        panic!("load: {e:#}\nserver.toml content:\n{content}");
    });
    assert_eq!(config.server.pow_difficulty_iterations, 8192);
    assert_eq!(config.email_allowed_domains, vec!["qq.com", "example.com"]);
    assert_eq!(config.cache.download_ttl_seconds, 60);
    assert_eq!(config.cache.cache_capacity, 100_000);

    let _ = std::fs::remove_dir_all(&directory);
}

fn write_configs(directory: &Path) {
    let server = r#"
listen_addr = "127.0.0.1:3000"
db_path = "memory"
search_index_path = "/tmp/search"
pdf_storage_path = "/tmp/pdf"
pow_difficulty_iterations = 8192
email_cooldown_seconds = 60
user_zero_email = "admin@example.com"
max_pdf_size_bytes = 33554432
max_tags_per_article = 8
max_title_chars = 200
max_summary_chars = 2000
max_comment_body_chars = 1024
max_version_note_chars = 1024
max_text_field_bytes = 1048576
max_search_query_chars = 512
search_page_size = 8
max_search_pages = 1024
tag_page_size = 8

[logging]
dir = "log/back"
retention_days = 7
filter = "warn"
"#;
    let emailer = r#"
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
    let cache = "download_ttl_seconds = 60\ncache_capacity = 100000\n";
    std::fs::write(directory.join("server.toml"), server).expect("server.toml");
    std::fs::write(directory.join("emailer.toml"), emailer).expect("emailer.toml");
    std::fs::write(directory.join("email.toml"), email).expect("email.toml");
    std::fs::write(directory.join("cache.toml"), cache).expect("cache.toml");
}
