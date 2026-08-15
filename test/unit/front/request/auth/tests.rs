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
fn authenticate_email_requests_a_create_user_token() {
    let pow = sample_pow("alice@example.com");
    let request = super::create_user_token_request(pow.clone());
    assert_eq!(request.purpose, TokenPurpose::CreateUser);
    assert_eq!(request.pow.as_ref(), Some(&pow));
    assert!(request.old_email_pow.is_none());
    assert!(request.new_email_pow.is_none());
}
