use crate::{Challenge, Pow, issue_challenge, prove, verify};
use uuid::Uuid;

fn sample_challenge() -> Challenge {
    Challenge {
        id: Uuid::parse_str("0197c0b0-1234-7000-8000-000000000001").expect("valid uuid"),
        difficulty: 1,
    }
}

#[test]
fn challenge_round_trips_on_the_wire() {
    let challenge = sample_challenge();
    let json = serde_json::to_string(&challenge).expect("serialize");
    assert_eq!(
        json,
        r#"{"id":"0197c0b0-1234-7000-8000-000000000001","difficulty":1}"#
    );
    let decoded: Challenge = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, challenge);
}

#[test]
fn pow_round_trips_on_the_wire() {
    let pow = Pow {
        challenge: sample_challenge(),
        solution: "ab".repeat(96),
        nonce: 0,
    };
    let json = serde_json::to_string(&pow).expect("serialize");
    let decoded: Pow = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, pow);
}

#[test]
fn issue_challenge_generates_distinct_uuids_with_the_requested_difficulty() {
    let a = issue_challenge(3);
    let b = issue_challenge(3);
    assert_eq!(a.difficulty, 3);
    assert_eq!(b.difficulty, 3);
    assert_ne!(a.id, b.id);
}

#[test]
fn prove_produces_a_96_byte_hex_solution() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert_eq!(pow.solution.len(), 192);
    assert!(pow.solution.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(pow.challenge, sample_challenge());
}

#[test]
fn verify_accepts_a_freshly_proved_pow() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(verify(&pow, 1));
}

#[test]
fn verify_rejects_difficulty_mismatch() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(!verify(&pow, 2));
}

#[test]
fn verify_rejects_non_hex_solution() {
    let pow = Pow {
        challenge: sample_challenge(),
        solution: "zz".repeat(96),
        nonce: 0,
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn verify_rejects_solution_with_wrong_byte_length() {
    for byte_count in [95usize, 97, 0, 48] {
        let pow = Pow {
            challenge: sample_challenge(),
            solution: "ab".repeat(byte_count),
            nonce: 0,
        };
        assert!(!verify(&pow, 1), "{byte_count} bytes");
    }
}

#[test]
fn verify_rejects_random_solution_bytes() {
    let mut random_solution = String::with_capacity(192);
    for index in 0..96 {
        let _ = std::fmt::Write::write_fmt(
            &mut random_solution,
            format_args!("{:02x}", (index * 7) % 256),
        );
    }
    let pow = Pow {
        challenge: sample_challenge(),
        solution: random_solution,
        nonce: 0,
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn verify_rejects_oversized_solution() {
    let pow = Pow {
        challenge: sample_challenge(),
        solution: "ab".repeat(2049),
        nonce: 0,
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn prove_is_deterministic_for_the_same_challenge() {
    let a = prove(&sample_challenge()).expect("prove a");
    let b = prove(&sample_challenge()).expect("prove b");
    assert_eq!(a.solution, b.solution);
}
