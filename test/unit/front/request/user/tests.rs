use nail_common::pow::{Challenge, Pow};
use nail_common::request::TokenPurpose;

fn sample_pow(payload: &str) -> Pow {
    Pow {
        challenge: Challenge {
            id: uuid::Uuid::parse_str("01932a52-0000-7000-8000-000000000000").expect("uuid"),
            difficulty: 1,
        },
        solution: "ab".repeat(96),
        payload: payload.to_string(),
    }
}

#[test]
fn change_email_requests_an_update_user_email_token() {
    let old_pow = sample_pow("alice@example.com");
    let new_pow = sample_pow("alice-new@example.com");
    let request = super::update_user_email_token_request(old_pow.clone(), new_pow.clone());
    assert_eq!(request.purpose, TokenPurpose::UpdateUserEmail);
    assert!(request.pow.is_none());
    assert_eq!(request.old_email_pow.as_ref(), Some(&old_pow));
    assert_eq!(request.new_email_pow.as_ref(), Some(&new_pow));
}

#[test]
fn deregister_requests_a_delete_user_token() {
    let pow = sample_pow("alice@example.com");
    let request = super::delete_user_token_request(pow.clone());
    assert_eq!(request.purpose, TokenPurpose::DeleteUser);
    assert_eq!(request.pow.as_ref(), Some(&pow));
    assert!(request.old_email_pow.is_none());
    assert!(request.new_email_pow.is_none());
}
