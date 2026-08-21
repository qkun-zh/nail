use crate::infrastructure::pow::prove;
use common::pow::{Challenge, Pow};
use uuid::Uuid;

fn challenge(difficulty: u64) -> Challenge {
    Challenge {
        id: Uuid::parse_str("01932a52-0000-7000-8000-000000000000").expect("valid uuid"),
        difficulty,
    }
}

#[test]
fn proves_a_challenge_at_minimal_difficulty() {
    let pow = prove(&challenge(1)).expect("prove must succeed");
    assert_eq!(pow.challenge.difficulty, 1);
    assert_eq!(pow.solution.len(), 192);
    assert!(pow.solution.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn proves_are_deterministic_per_challenge() {
    let first = prove(&challenge(1)).expect("prove");
    let second = prove(&challenge(1)).expect("prove");
    assert_eq!(first, second);
}

#[test]
fn solution_is_a_valid_hex_of_ninety_six_bytes() {
    let pow: Pow = prove(&challenge(1)).expect("prove");
    let decoded: Vec<u8> = pow
        .solution
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hex = std::str::from_utf8(pair).expect("ascii");
            u8::from_str_radix(hex, 16).expect("hex digit")
        })
        .collect();
    assert_eq!(decoded.len(), 96);
}
