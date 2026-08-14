use crate::pow::Challenge;
use crate::pow::Pow;
use crate::pow::ProveInput;

fn sample_challenge() -> anyhow::Result<Challenge> {
    Ok(Challenge {
        id: uuid::Uuid::parse_str("0197c0b0-1234-7000-8000-000000000001")?,
        difficulty: 1,
    })
}

#[test]
fn challenge_round_trips_on_the_wire() -> anyhow::Result<()> {
    let challenge = sample_challenge()?;
    let json = serde_json::to_string(&challenge)?;
    assert_eq!(
        json,
        r##"{"id":"0197c0b0-1234-7000-8000-000000000001","difficulty":1}"##
    );
    let decoded: Challenge = serde_json::from_str(&json)?;
    assert_eq!(decoded, challenge);
    Ok(())
}

#[test]
fn pow_round_trips_on_the_wire() -> anyhow::Result<()> {
    let pow = Pow {
        challenge: sample_challenge()?,
        solution: "ab".repeat(96),
        payload: "hello".to_string(),
    };
    let json = serde_json::to_string(&pow)?;
    let decoded: Pow = serde_json::from_str(&json)?;
    assert_eq!(decoded, pow);
    Ok(())
}

#[test]
fn prove_input_round_trips_on_the_wire() -> anyhow::Result<()> {
    let input = ProveInput {
        challenge: sample_challenge()?,
        payload: "hello".to_string(),
    };
    let json = serde_json::to_string(&input)?;
    let decoded: ProveInput = serde_json::from_str(&json)?;
    assert_eq!(decoded, input);
    Ok(())
}

#[test]
fn prove_produces_a_96_byte_hex_solution() -> anyhow::Result<()> {
    let pow = crate::pow::prove(ProveInput {
        challenge: sample_challenge()?,
        payload: "hello".to_string(),
    })?;
    assert_eq!(pow.solution.len(), 192);
    assert!(pow.solution.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(pow.challenge, sample_challenge()?);
    assert_eq!(pow.payload, "hello");
    Ok(())
}

#[test]
fn verify_accepts_a_round_tripped_proof() -> anyhow::Result<()> {
    let challenge = sample_challenge()?;
    let pow = crate::pow::prove(ProveInput {
        challenge: challenge.clone(),
        payload: "hello".to_string(),
    })?;
    assert!(crate::pow::verify(&pow, 1));
    Ok(())
}

#[test]
fn verify_rejects_difficulty_mismatch() -> anyhow::Result<()> {
    let challenge = sample_challenge()?;
    let pow = crate::pow::prove(ProveInput {
        challenge: challenge.clone(),
        payload: "hello".to_string(),
    })?;
    assert!(!crate::pow::verify(&pow, 2));
    Ok(())
}

#[test]
fn verify_rejects_non_hex_solution() -> anyhow::Result<()> {
    let pow = Pow {
        challenge: sample_challenge()?,
        solution: "zz".repeat(96),
        payload: "hello".to_string(),
    };
    assert!(!crate::pow::verify(&pow, 1));
    Ok(())
}

#[test]
fn verify_rejects_solution_with_wrong_byte_length() -> anyhow::Result<()> {
    for byte_count in [95usize, 97, 0, 48] {
        let pow = Pow {
            challenge: sample_challenge()?,
            solution: "ab".repeat(byte_count),
            payload: "hello".to_string(),
        };
        assert!(!crate::pow::verify(&pow, 1), "{byte_count} bytes");
    }
    Ok(())
}

#[test]
fn verify_rejects_random_solution_bytes() -> anyhow::Result<()> {
    let random_solution: String = (0..96)
        .map(|index| format!("{:02x}", (index * 7) % 256))
        .collect();
    let pow = Pow {
        challenge: sample_challenge()?,
        solution: random_solution,
        payload: "hello".to_string(),
    };
    assert!(!crate::pow::verify(&pow, 1));
    Ok(())
}

#[test]
fn verify_rejects_oversized_solution_and_payload() -> anyhow::Result<()> {
    let oversized_solution = Pow {
        challenge: sample_challenge()?,
        solution: "ab".repeat(2049),
        payload: "hello".to_string(),
    };
    assert!(!crate::pow::verify(&oversized_solution, 1));
    let oversized_payload = Pow {
        challenge: sample_challenge()?,
        solution: "ab".repeat(96),
        payload: "x".repeat(4097),
    };
    assert!(!crate::pow::verify(&oversized_payload, 1));
    Ok(())
}

#[test]
fn prove_accepts_payload_at_the_byte_cap() -> anyhow::Result<()> {
    let at_cap = "x".repeat(4096);
    let pow = crate::pow::prove(ProveInput {
        challenge: sample_challenge()?,
        payload: at_cap.clone(),
    })?;
    assert_eq!(pow.payload, at_cap);
    assert!(crate::pow::verify(&pow, 1));
    Ok(())
}

#[test]
fn prove_rejects_payload_beyond_the_byte_cap() -> anyhow::Result<()> {
    let result = crate::pow::prove(ProveInput {
        challenge: sample_challenge()?,
        payload: "x".repeat(4097),
    });
    assert!(result.is_err());
    Ok(())
}
