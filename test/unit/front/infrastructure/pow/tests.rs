use crate::infrastructure::pow::prove;
use nail_common::pow::{Challenge, Pow, ProveInput};
use uuid::Uuid;

fn challenge(difficulty: u64) -> Challenge {
    Challenge {
        id: Uuid::parse_str("01932a52-0000-7000-8000-000000000000").expect("valid uuid"),
        difficulty,
    }
}

#[test]
fn proves_an_input_at_minimal_difficulty() {
    let input = ProveInput {
        challenge: challenge(1),
        payload: "hello".to_string(),
    };
    let pow = prove(input).expect("prove must succeed");
    assert_eq!(pow.challenge.difficulty, 1);
    assert_eq!(pow.payload, "hello");
    assert_eq!(pow.solution.len(), 192);
    assert!(pow.solution.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn proves_are_deterministic_per_challenge_and_payload() {
    let first = prove(ProveInput {
        challenge: challenge(1),
        payload: "same".to_string(),
    })
    .expect("prove");
    let second = prove(ProveInput {
        challenge: challenge(1),
        payload: "same".to_string(),
    })
    .expect("prove");
    assert_eq!(first, second);
}

#[test]
fn maps_prove_errors_to_strings() {
    let oversized = "x".repeat(4097);
    let result = prove(ProveInput {
        challenge: challenge(1),
        payload: oversized,
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("proof of work"));
}

#[test]
fn solution_is_a_valid_hex_of_ninety_six_bytes() {
    let pow: Pow = prove(ProveInput {
        challenge: challenge(1),
        payload: "probe".to_string(),
    })
    .expect("prove");
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
