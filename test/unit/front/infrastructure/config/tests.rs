use crate::infrastructure::config::validate_api_base_url;

#[test]
fn empty_base_means_same_origin() {
    assert_eq!(validate_api_base_url(""), Ok(String::new()));
}

#[test]
fn accepts_http_and_https_origins() {
    assert_eq!(
        validate_api_base_url("http://localhost:3000"),
        Ok("http://localhost:3000".to_string())
    );
    assert_eq!(
        validate_api_base_url("https://api.example.com"),
        Ok("https://api.example.com".to_string())
    );
}

#[test]
fn trims_trailing_slashes() {
    assert_eq!(
        validate_api_base_url("https://api.example.com/"),
        Ok("https://api.example.com".to_string())
    );
    assert_eq!(
        validate_api_base_url("http://host:8080///"),
        Ok("http://host:8080".to_string())
    );
}

#[test]
fn rejects_non_http_schemes() {
    assert!(validate_api_base_url("ws://localhost").is_err());
    assert!(validate_api_base_url("ftp://localhost").is_err());
    assert!(validate_api_base_url("localhost:3000").is_err());
    assert!(validate_api_base_url("api.example.com").is_err());
}

#[test]
fn rejects_uppercase_scheme() {
    assert!(validate_api_base_url("HTTP://localhost").is_err());
    assert!(validate_api_base_url("HTTPS://localhost").is_err());
}

#[test]
fn rejects_http_prefix_without_scheme_separator() {
    assert!(validate_api_base_url("https:/api.example.com").is_err());
}
