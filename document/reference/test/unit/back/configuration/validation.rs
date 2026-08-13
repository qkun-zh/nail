
use std::path::PathBuf;

use crate::other::conf::AppConfig;

const MINIMAL_SERVER: &str = r#"
listen_addr = "127.0.0.1:3000"
db_path = "/tmp/nail_db"
db_namespace = "ns"
db_database = "db"
search_index_path = "/tmp/nail_search_index"
pow_difficulty_iterations = 8192
token_ttl_seconds = 8000
session_ttl_seconds = 8000
pdf_storage_path = "/tmp/nail_pdf"
max_pdf_size_bytes = 10485760
download_token_ttl_seconds = 60
max_tags_per_article = 8
max_title_chars = 200
max_summary_chars = 2000
search_page_size = 8
max_search_page_size = 200
max_search_query_chars = 512
email_cooldown_seconds = 60
max_page = 10000
max_search_pages = 1024
max_id_filter_count = 64
max_comment_body_chars = 1024
max_version_note_chars = 1024
max_comment_tree_depth = 64
max_text_field_bytes = 1048576
challenge_ttl_seconds = 300
token_cache_capacity = 100000
log_prune_interval_secs = 1800
log_dir = "/tmp/nail_log"
log_retention_days = 7
log_max_file_count = 500
user_zero_email = "qkun-zh@qq.com"
"#;

const MINIMAL_SMTP: &str = r#"
host = "127.0.0.1"
port = 25
username = ""
password = "s3cret"
from_email = "x@example.com"
from_name = "nail"
timeout_secs = 10
wall_clock_timeout_secs = 30
"#;

const MINIMAL_EMAIL: &str = r#"
allowed_domains = [ "qq.com", " QQ.com ", "@foxmail.com", "GMAIL.COM", " " ]
"#;

fn write_config(dir: &PathBuf, server: &str, smtp: &str, email: &str) {
    std::fs::create_dir_all(dir).expect("创建目录");
    std::fs::write(dir.join("server.toml"), server).expect("写 server.toml");
    std::fs::write(dir.join("smtp.toml"), smtp).expect("写 smtp.toml");
    std::fs::write(dir.join("email.toml"), email).expect("写 email.toml");
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nail_conf_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_from_parses_all_three_files() {
    let dir = temp_dir("ok");
    write_config(&dir, MINIMAL_SERVER, MINIMAL_SMTP, MINIMAL_EMAIL);
    let config = AppConfig::load_from(&dir).expect("加载成功");
    assert_eq!(config.server.listen_addr, "127.0.0.1:3000");
    assert_eq!(config.server.pow_difficulty_iterations, 8192);
    assert_eq!(config.server.max_tags_per_article, 8);
    assert_eq!(config.smtp.port, 25);
    assert_eq!(
        config.email.allowed_domains,
        vec!["qq.com", "qq.com", "foxmail.com", "gmail.com"],
        "条目按输入顺序归一化（重复保留，匹配逻辑对重复无害）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn missing_file_is_rejected() {
    let dir = temp_dir("missing");
    write_config(&dir, MINIMAL_SERVER, MINIMAL_SMTP, MINIMAL_EMAIL);
    std::fs::remove_file(dir.join("email.toml")).expect("删除 email.toml");
    let err = AppConfig::load_from(&dir).expect_err("缺文件必须报错");
    assert!(
        err.to_string().contains("failed to read config file"),
        "错误应含读取失败语义，实际: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_validation_rejects_bad_values() {
    let cases: Vec<(&str, &str, &str, &str)> = vec![
        (
            "pow",
            "pow_difficulty_iterations = 8192",
            "pow_difficulty_iterations = 0",
            "pow_difficulty_iterations must be in 1..=10_000",
        ),
        (
            "pow_high",
            "pow_difficulty_iterations = 8192",
            "pow_difficulty_iterations = 99999",
            "pow_difficulty_iterations must be in 1..=10_000",
        ),
        (
            "page_size",
            "search_page_size = 8",
            "search_page_size = 201",
            "search_page_size must not exceed",
        ),
        (
            "field_bytes",
            "max_text_field_bytes = 1048576",
            "max_text_field_bytes = 20000000",
            "max_text_field_bytes must not exceed",
        ),
        (
            "listen",
            "listen_addr = \"127.0.0.1:3000\"",
            "listen_addr = \"\"",
            "listen_addr must not be empty",
        ),
        (
            "db",
            "db_namespace = \"ns\"",
            "db_namespace = \"\"",
            "db_namespace",
        ),
        (
            "user_zero",
            "user_zero_email = \"qkun-zh@qq.com\"",
            "user_zero_email = \"\"",
            "user_zero_email must not be empty",
        ),
    ];
    for (tag, old_line, new_line, expected) in cases {
        let dir = temp_dir(tag);
        let server = MINIMAL_SERVER.replace(old_line, new_line);
        write_config(&dir, &server, MINIMAL_SMTP, MINIMAL_EMAIL);
        let err = AppConfig::load_from(&dir).expect_err(&format!("{tag} 必须校验失败"));
        assert!(
            err.to_string().contains(expected),
            "{tag}: 错误应含 {expected:?}，实际: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn smtp_validation_rejects_bad_values() {
    let dir = temp_dir("port0");
    let smtp = MINIMAL_SMTP.replace("port = 25", "port = 0");
    write_config(&dir, MINIMAL_SERVER, &smtp, MINIMAL_EMAIL);
    let err = AppConfig::load_from(&dir).expect_err("port=0 必须校验失败");
    assert!(
        err.to_string().contains("smtp.port must not be 0"),
        "实际: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);

    let dir = temp_dir("wall");
    let smtp = MINIMAL_SMTP.replace(
        "wall_clock_timeout_secs = 30",
        "wall_clock_timeout_secs = 5",
    );
    write_config(&dir, MINIMAL_SERVER, &smtp, MINIMAL_EMAIL);
    let err = AppConfig::load_from(&dir).expect_err("wall_clock < timeout 必须校验失败");
    assert!(
        err.to_string()
            .contains("wall_clock_timeout_secs must be >= timeout_secs"),
        "实际: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn smtp_debug_redacts_password() {
    let dir = temp_dir("redact");
    write_config(&dir, MINIMAL_SERVER, MINIMAL_SMTP, MINIMAL_EMAIL);
    let config = AppConfig::load_from(&dir).expect("加载");
    let debug = format!("{:?}", config.smtp);
    assert!(!debug.contains("s3cret"), "Debug 输出不得泄露密码: {debug}");
    assert!(
        debug.contains("***"),
        "Debug 输出应以 *** 掩码密码: {debug}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
