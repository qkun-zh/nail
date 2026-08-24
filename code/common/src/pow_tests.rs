use super::{Challenge, HASH_MULTIPLIER, MAX_DIFFICULTY, Pow, issue_challenge, prove, verify};
use uuid::Uuid;

fn sample_challenge() -> Challenge {
    Challenge {
        id: Uuid::parse_str("0197c0b0-1234-7000-8000-000000000001").expect("valid uuid"),
        difficulty: 1,
    }
}

#[test]
fn issue_challenge_returns_requested_difficulty() {
    for d in [1, 2, 100, 1000, MAX_DIFFICULTY] {
        assert_eq!(issue_challenge(d).difficulty, d);
    }
}

#[test]
fn issue_challenge_generates_distinct_uuids() {
    let a = issue_challenge(1);
    let b = issue_challenge(1);
    assert_ne!(a.id, b.id);
    assert_eq!(a.difficulty, b.difficulty);
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
fn pow_nonce_defaults_to_zero_for_old_payloads() {
    let json = r#"{"challenge":{"id":"0197c0b0-1234-7000-8000-000000000001","difficulty":1},"solution":"ababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababab"}"#;
    let pow: Pow = serde_json::from_str(json).expect("deserialize old payload");
    assert_eq!(pow.nonce, 0);
}

#[test]
fn prove_rejects_difficulty_zero() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: 0,
    };
    let err = prove(&ch).unwrap_err();
    assert!(err.to_string().contains("must be > 0"));
}

#[test]
fn prove_rejects_difficulty_above_max() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: MAX_DIFFICULTY + 1,
    };
    let err = prove(&ch).unwrap_err();
    assert!(err.to_string().contains("MAX_DIFFICULTY"));
}

#[test]
fn prove_solution_is_96_byte_hex() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert_eq!(pow.solution.len(), 192);
    assert!(pow.solution.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn prove_nonce_is_reasonably_small_for_low_difficulty() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: 1,
    };
    let pow = prove(&ch).expect("prove");
    let expected_trials = HASH_MULTIPLIER * ch.difficulty;
    assert!(
        pow.nonce < expected_trials * 16,
        "nonce too large for difficulty 1: {}",
        pow.nonce
    );
}

#[test]
fn prove_copies_challenge_into_pow() {
    let ch = sample_challenge();
    let pow = prove(&ch).expect("prove");
    assert_eq!(pow.challenge, ch);
}

#[test]
fn prove_is_deterministic_for_the_same_challenge() {
    let a = prove(&sample_challenge()).expect("a");
    let b = prove(&sample_challenge()).expect("b");
    assert_eq!(a.nonce, b.nonce);
    assert_eq!(a.solution, b.solution);
}

#[test]
fn prove_different_challenges_yield_different_nonces() {
    let a = prove(&sample_challenge()).expect("a");
    let b = prove(&issue_challenge(1)).expect("b");
    assert_ne!(a.nonce, b.nonce);
}

#[test]
fn prove_works_at_various_difficulties() {
    for d in [1, 2, 10, 50, 100, 500, 1000] {
        let ch = Challenge {
            id: Uuid::now_v7(),
            difficulty: d,
        };
        let pow = prove(&ch).expect("prove");
        assert_eq!(pow.challenge.difficulty, d);
        assert_eq!(pow.solution.len(), 192);
    }
}

#[test]
fn prove_works_at_max_difficulty() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: MAX_DIFFICULTY,
    };
    let pow = prove(&ch).expect("prove");
    assert_eq!(pow.solution.len(), 192);
}

#[test]
fn verify_accepts_freshly_proved_pow() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(verify(&pow, 1));
}

#[test]
fn verify_accepts_at_various_difficulties() {
    for d in [1, 5, 50, 200, 1000] {
        let ch = Challenge {
            id: Uuid::now_v7(),
            difficulty: d,
        };
        let pow = prove(&ch).expect("prove");
        assert!(verify(&pow, d), "failed at difficulty {d}");
    }
}

#[test]
fn verify_accepts_at_max_difficulty() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: MAX_DIFFICULTY,
    };
    let pow = prove(&ch).expect("prove");
    assert!(verify(&pow, MAX_DIFFICULTY));
}

#[test]
fn verify_rejects_difficulty_mismatch() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(!verify(&pow, 2));
    assert!(!verify(&pow, 100));
}

#[test]
fn verify_rejects_server_difficulty_zero() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(!verify(&pow, 0));
}

#[test]
fn verify_rejects_server_difficulty_above_max() {
    let pow = prove(&sample_challenge()).expect("prove");
    assert!(!verify(&pow, MAX_DIFFICULTY + 1));
}

#[test]
fn verify_rejects_tampered_nonce() {
    let mut pow = prove(&issue_challenge(100)).expect("prove");
    pow.nonce = pow.nonce.wrapping_add(1);
    assert!(!verify(&pow, 100));
}

#[test]
fn verify_rejects_tampered_solution() {
    let mut pow = prove(&sample_challenge()).expect("prove");
    if pow.solution.starts_with('a') {
        pow.solution.replace_range(0..1, "b");
    } else {
        pow.solution.replace_range(0..1, "a");
    }
    assert!(!verify(&pow, 1));
}

#[test]
fn verify_rejects_tampered_challenge_difficulty() {
    let mut pow = prove(&sample_challenge()).expect("prove");
    pow.challenge.difficulty = 2;
    assert!(!verify(&pow, 2));
}

#[test]
fn verify_rejects_tampered_challenge_id() {
    let mut pow = prove(&sample_challenge()).expect("prove");
    pow.challenge.id = Uuid::now_v7();
    assert!(!verify(&pow, 1));
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
fn verify_rejects_wrong_byte_length() {
    for byte_count in [0usize, 48, 95, 97, 100] {
        let pow = Pow {
            challenge: sample_challenge(),
            solution: "ab".repeat(byte_count),
            nonce: 0,
        };
        assert!(!verify(&pow, 1), "should reject {byte_count} bytes");
    }
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
fn verify_rejects_random_solution_bytes() {
    let mut solution = String::with_capacity(192);
    for index in 0..96 {
        let _ =
            std::fmt::Write::write_fmt(&mut solution, format_args!("{:02x}", (index * 7) % 256));
    }
    let pow = Pow {
        challenge: sample_challenge(),
        solution,
        nonce: 0,
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn prove_nonce_increases_until_target_met() {
    let ch = Challenge {
        id: Uuid::now_v7(),
        difficulty: 1000,
    };
    let pow = prove(&ch).expect("prove");
    assert!(pow.nonce < u64::MAX);
    assert!(verify(&pow, 1000));
}

#[test]
fn high_difficulty_requires_higher_nonce() {
    let low = prove(&Challenge {
        id: Uuid::now_v7(),
        difficulty: 1,
    })
    .unwrap();
    let high = prove(&Challenge {
        id: Uuid::now_v7(),
        difficulty: 1000,
    })
    .unwrap();
    assert!(high.nonce >= low.nonce);
}

#[test]
fn full_round_trip_various_difficulties() {
    for d in [1, 3, 7, 13, 64, 255, 256, 1000] {
        let ch = Challenge {
            id: Uuid::now_v7(),
            difficulty: d,
        };
        let pow = prove(&ch).expect("prove");
        assert!(verify(&pow, d), "round trip failed at difficulty {d}");
    }
}

#[test]
fn ten_proves_at_same_difficulty_all_verify() {
    for _ in 0..10 {
        let ch = Challenge {
            id: Uuid::now_v7(),
            difficulty: 50,
        };
        let pow = prove(&ch).expect("prove");
        assert!(verify(&pow, 50));
    }
}

#[test]
fn solution_is_always_96_bytes_hex_encoded() {
    for d in [1, 100, 1000] {
        let ch = Challenge {
            id: Uuid::now_v7(),
            difficulty: d,
        };
        let pow = prove(&ch).expect("prove");
        let decoded = hex::decode(&pow.solution).expect("valid hex");
        assert_eq!(decoded.len(), 96, "decoded len at difficulty {d}");
    }
}
