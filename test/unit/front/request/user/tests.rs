use nail_common::request::TokenPurpose;

#[test]
fn change_email_requests_an_update_user_email_token() {
    let request = super::update_user_email_token_request(
        "alice@example.com".to_string(),
        "alice-new@example.com".to_string(),
    );
    assert_eq!(request.purpose, TokenPurpose::UpdateUserEmail);
    assert!(request.email.is_none());
    assert_eq!(request.old_email.as_deref(), Some("alice@example.com"));
    assert_eq!(request.new_email.as_deref(), Some("alice-new@example.com"));
}

#[test]
fn deregister_requests_a_delete_user_token() {
    let request = super::delete_user_token_request("alice@example.com".to_string());
    assert_eq!(request.purpose, TokenPurpose::DeleteUser);
    assert_eq!(request.email.as_deref(), Some("alice@example.com"));
    assert!(request.old_email.is_none());
    assert!(request.new_email.is_none());
}
