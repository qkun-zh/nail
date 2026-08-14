use super::context::TestCtx;
use crate::logic::error::LogicError;

#[tokio::test]
async fn accepts_an_issued_and_valid_proof() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("payload");
    assert!(crate::logic::pow::verify_issued_pow(&context.state, &pow).is_ok());
}

#[tokio::test]
async fn rejects_a_proof_whose_challenge_was_never_issued() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.client_pow("payload");
    let error = crate::logic::pow::verify_issued_pow(&context.state, &pow).unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("challenge not issued, expired, or already used")
    );
}

#[tokio::test]
async fn rejects_an_already_consumed_challenge() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("payload");
    assert!(crate::logic::pow::verify_issued_pow(&context.state, &pow).is_ok());
    let error = crate::logic::pow::verify_issued_pow(&context.state, &pow).unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("challenge not issued, expired, or already used")
    );
}

#[tokio::test]
async fn rejects_a_tampered_solution() {
    let context = TestCtx::new().await.expect("test context");
    let mut pow = context.issued_pow("payload");
    pow.solution = format!(
        "{}{}",
        if pow.solution.starts_with('0') {
            '1'
        } else {
            '0'
        },
        &pow.solution[1..]
    );
    let error = crate::logic::pow::verify_issued_pow(&context.state, &pow).unwrap_err();
    assert_eq!(error, LogicError::bad_request("PoW verification failed"));
}

#[tokio::test]
async fn rejects_a_proof_with_a_different_difficulty() {
    let context = TestCtx::new().await.expect("test context");
    let mut pow = context.issued_pow("payload");
    pow.challenge.difficulty += 1;
    let error = crate::logic::pow::verify_issued_pow(&context.state, &pow).unwrap_err();
    assert_eq!(error, LogicError::bad_request("PoW verification failed"));
}
