use super::context::test_config;
use crate::infrastructure::config::server::ServerConfig;

#[test]
fn max_request_body_bytes_sums_pdf_and_five_text_fields_plus_64kib() {
    let mut config = test_config();
    config.server.max_pdf_size_bytes = 4096;
    config.server.max_text_field_bytes = 64;
    assert_eq!(
        config.server.max_request_body_bytes(),
        4096 + 5 * 64 + 64 * 1024
    );
}

#[test]
fn max_request_body_bytes_is_saturating() {
    let config = ServerConfig {
        max_pdf_size_bytes: u64::MAX,
        max_text_field_bytes: u64::MAX,
        ..test_config().server
    };
    assert_eq!(config.max_request_body_bytes(), u64::MAX);
}
