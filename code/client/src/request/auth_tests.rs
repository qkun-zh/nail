use common::request::TokenPurpose;

#[test]
fn authenticate_email_requests_a_create_user_token() {
    let request = super::create_user_token_request("alice@example.com".to_string());
    assert_eq!(request.purpose, TokenPurpose::CreateUser);
    assert_eq!(request.email.as_deref(), Some("alice@example.com"));
    assert!(request.old_email.is_none());
    assert!(request.new_email.is_none());
}
