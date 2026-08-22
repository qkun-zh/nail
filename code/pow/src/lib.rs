//! Proof-of-work primitives: issue challenges, produce proofs, verify them.
//!
//! This crate is stateless. Challenge storage (issue tracking, single-use
//! enforcement) belongs to the caller; this crate only derives and checks
//! VDF proofs bound to a challenge id.

use anyhow::Context;
use ascon_xof128::{AsconCxof128, ExtendableOutput, TryCustomizedInit, Update, XofReader};
use pso_vdf::{
    Vdf,
    minroot::{MinRootProof, MinRootVdf},
    types::{VdfInput, VdfOutput},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_SOLUTION_HEX_LEN: usize = 4096;

/// Upper bound for VDF difficulty: keeps prove time bounded (~1M iterations).
pub const MAX_DIFFICULTY: u64 = 1 << 20;

const VDF_OUTPUT_BYTES: usize = 48;
const VDF_PROOF_BYTES: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub difficulty: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pow {
    pub challenge: Challenge,
    pub solution: String,
    #[serde(default)]
    pub nonce: u64,
}

#[must_use]
pub fn issue_challenge(difficulty: u64) -> Challenge {
    Challenge {
        id: Uuid::now_v7(),
        difficulty,
    }
}

const HASH_MULTIPLIER: u64 = 64;

fn hash_meets_target(bytes: &[u8; 32], difficulty: u64) -> bool {
    if difficulty == 0 {
        return true;
    }
    let scalar = u128::from(difficulty.saturating_mul(HASH_MULTIPLIER).max(1));
    let hash_prefix = u128::from_be_bytes(bytes[0..16].try_into().unwrap_or([0xff; 16]));
    let threshold = u128::MAX / scalar;
    hash_prefix < threshold
}

fn cxof_bytes(challenge_id: &Uuid, nonce: u64) -> anyhow::Result<[u8; 32]> {
    let mut nonce_custom = Vec::with_capacity(challenge_id.as_bytes().len() + 8);
    nonce_custom.extend_from_slice(challenge_id.as_bytes());
    nonce_custom.extend_from_slice(&nonce.to_le_bytes());
    let mut cxof =
        AsconCxof128::try_new_customized(&nonce_custom).context("failed to init Ascon CXOF")?;
    cxof.update(challenge_id.as_bytes());
    let mut output = [0u8; 32];
    cxof.finalize_xof().read(&mut output);
    Ok(output)
}

fn vdf_prove(raw_input: [u8; 32], difficulty: u64) -> (Vec<u8>, Vec<u8>) {
    let input = VdfInput::from_bytes(raw_input);
    let (output, proof) = MinRootVdf::eval(&input, difficulty);
    (output.0, proof.inner)
}

fn vdf_verify(raw_input: [u8; 32], difficulty: u64, output: &[u8], proof: &[u8]) -> bool {
    if output.len() != VDF_OUTPUT_BYTES || proof.len() != VDF_PROOF_BYTES {
        return false;
    }
    let input = VdfInput::from_bytes(raw_input);
    let output_obj = VdfOutput(output.to_vec());
    let proof_obj = MinRootProof {
        inner: proof.to_vec(),
    };
    MinRootVdf::verify(&input, &output_obj, &proof_obj, difficulty)
}

/// Produces a proof-of-work [`Pow`] for a challenge.
///
/// # Errors
/// Returns an error if the Ascon CXOF cannot be initialized.
pub fn prove(challenge: &Challenge) -> anyhow::Result<Pow> {
    anyhow::ensure!(challenge.difficulty > 0, "difficulty must be > 0");
    anyhow::ensure!(
        challenge.difficulty <= MAX_DIFFICULTY,
        "difficulty exceeds MAX_DIFFICULTY"
    );
    let mut nonce = 0u64;
    let input = loop {
        let candidate = cxof_bytes(&challenge.id, nonce)?;
        if hash_meets_target(&candidate, challenge.difficulty) {
            break candidate;
        }
        nonce = nonce.wrapping_add(1);
    };
    let (output, proof) = vdf_prove(input, challenge.difficulty);
    let mut solution = Vec::with_capacity(output.len() + proof.len());
    solution.extend_from_slice(&output);
    solution.extend_from_slice(&proof);
    Ok(Pow {
        challenge: challenge.clone(),
        solution: hex::encode(solution),
        nonce,
    })
}

#[must_use]
pub fn verify(pow: &Pow, server_difficulty: u64) -> bool {
    if pow.challenge.difficulty != server_difficulty {
        return false;
    }
    if server_difficulty == 0 || server_difficulty > MAX_DIFFICULTY {
        return false;
    }
    if pow.solution.len() > MAX_SOLUTION_HEX_LEN {
        return false;
    }
    let Ok(bytes) = hex::decode(&pow.solution) else {
        return false;
    };
    if bytes.len() != VDF_OUTPUT_BYTES + VDF_PROOF_BYTES {
        return false;
    }
    let Ok(input) = cxof_bytes(&pow.challenge.id, pow.nonce) else {
        return false;
    };
    if !hash_meets_target(&input, server_difficulty) {
        return false;
    }
    vdf_verify(input, server_difficulty, &bytes[..48], &bytes[48..])
}

#[cfg(test)]
mod tests;
