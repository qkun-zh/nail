
use super::*;

const TEST_POW_DIFFICULTY: u64 = 16;

#[test]
fn verify_rejects_oversized_solution() {
    let pow = Pow {
        challenge: Challenge {
            id: Uuid::now_v7(),
            difficulty: 1,
        },
        solution: "ab".repeat(3000),
        payload: "p".to_string(),
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn verify_rejects_oversized_payload() {
    let pow = Pow {
        challenge: Challenge {
            id: Uuid::now_v7(),
            difficulty: 1,
        },
        solution: hex::encode(vec![0x42u8; 96]),
        payload: "x".repeat(5000),
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn prove_rejects_oversized_payload() {
    let challenge = Challenge {
        id: Uuid::now_v7(),
        difficulty: 1,
    };
    assert!(
        prove(ProveInput {
            challenge,
            payload: "x".repeat(5000),
        })
        .is_err()
    );
}

#[test]
fn verify_rejects_wrong_length_solution() {
    for len in [1usize, 47, 48, 95, 97, 100] {
        let pow = Pow {
            challenge: Challenge {
                id: Uuid::now_v7(),
                difficulty: 1,
            },
            solution: hex::encode(vec![0u8; len]),
            payload: "p".to_string(),
        };
        assert!(!verify(&pow, 1), "solution of {len} bytes must be rejected");
    }
}

#[test]
fn verify_accepts_valid_length_but_garbage_solution() {
    let pow = Pow {
        challenge: Challenge {
            id: Uuid::now_v7(),
            difficulty: 1,
        },
        solution: hex::encode(vec![0x42u8; 96]),
        payload: "p".to_string(),
    };
    assert!(!verify(&pow, 1));
}

#[test]
fn prove_and_verify_roundtrip() {
    let challenge = Challenge {
        id: Uuid::now_v7(),
        difficulty: TEST_POW_DIFFICULTY,
    };
    let pow = prove(ProveInput {
        challenge,
        payload: "roundtrip".to_string(),
    })
    .unwrap();
    assert!(verify(&pow, TEST_POW_DIFFICULTY));
}
