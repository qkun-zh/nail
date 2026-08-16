use anyhow::Context;
use ascon_xof128::{AsconCxof128, ExtendableOutput, TryCustomizedInit, Update, XofReader};
use pso_vdf::{
    Vdf,
    minroot::{MinRootProof, MinRootVdf},
    types::{VdfInput, VdfOutput},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub difficulty: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pow {
    pub challenge: Challenge,
    pub solution: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProveInput {
    pub challenge: Challenge,
    pub payload: String,
}

const MAX_SOLUTION_HEX_LEN: usize = 4096;

const MAX_PAYLOAD_BYTES: usize = 4096;

fn pow_cxof(raw_data: &str, nonce: &str) -> anyhow::Result<[u8; 32]> {
    let nonce_bytes = if nonce.len() > 1024 {
        &nonce.as_bytes()[..1024]
    } else {
        nonce.as_bytes()
    };
    let mut cxof =
        AsconCxof128::try_new_customized(nonce_bytes).context("failed to init Ascon CXOF")?;
    cxof.update(raw_data.as_bytes());
    let mut output = [0u8; 32];
    cxof.finalize_xof().read(&mut output);
    Ok(output)
}

fn pow_vdf_prove(raw_input: [u8; 32], difficulty: u64) -> (Vec<u8>, Vec<u8>) {
    let input = VdfInput::from_bytes(raw_input);
    let (output, proof) = MinRootVdf::eval(&input, difficulty);
    (output.0, proof.inner)
}

fn pow_vdf_verify(raw_input: [u8; 32], difficulty: u64, output: &[u8], proof: &[u8]) -> bool {
    if output.len() != 48 || proof.len() != 48 {
        return false;
    }
    let input = VdfInput::from_bytes(raw_input);
    let output_obj = VdfOutput(output.to_vec());
    let proof_obj = MinRootProof {
        inner: proof.to_vec(),
    };
    MinRootVdf::verify(&input, &output_obj, &proof_obj, difficulty)
}

fn pow_prove_internal(id: &str, raw_data: &str, difficulty: u64) -> anyhow::Result<String> {
    if raw_data.len() > MAX_PAYLOAD_BYTES {
        anyhow::bail!("payload too long (max {MAX_PAYLOAD_BYTES} bytes)");
    }
    let cxof_bytes = pow_cxof(raw_data, id)?;
    let (output, proof) = pow_vdf_prove(cxof_bytes, difficulty);
    let mut out = Vec::with_capacity(output.len() + proof.len());
    out.extend_from_slice(&output);
    out.extend_from_slice(&proof);
    Ok(hex::encode(out))
}

/// Produces a proof-of-work [`Pow`] for a challenge.
///
/// # Errors
/// Returns an error if the payload is too large or the ascon CXOF cannot be
/// initialized.
pub fn prove(input: ProveInput) -> anyhow::Result<Pow> {
    let id_str = input.challenge.id.to_string();
    let solution_hex = pow_prove_internal(&id_str, &input.payload, input.challenge.difficulty)?;
    Ok(Pow {
        challenge: input.challenge,
        solution: solution_hex,
        payload: input.payload,
    })
}

fn pow_verify_internal(id: &str, raw_data: &str, difficulty: u64, solution_hex: &str) -> bool {
    if solution_hex.len() > MAX_SOLUTION_HEX_LEN {
        return false;
    }
    if raw_data.len() > MAX_PAYLOAD_BYTES {
        return false;
    }
    let Ok(bytes) = hex::decode(solution_hex) else {
        return false;
    };
    if bytes.len() != 96 {
        return false;
    }
    let output = &bytes[..48];
    let proof = &bytes[48..];
    let Ok(cxof_bytes) = pow_cxof(raw_data, id) else {
        return false;
    };
    pow_vdf_verify(cxof_bytes, difficulty, output, proof)
}

#[must_use]
pub fn verify(pow: &Pow, server_difficulty: u64) -> bool {
    if pow.challenge.difficulty != server_difficulty {
        return false;
    }
    let id_str = pow.challenge.id.to_string();
    pow_verify_internal(&id_str, &pow.payload, server_difficulty, &pow.solution)
}

#[cfg(test)]
#[path = "../../../test/unit/common/pow/tests.rs"]
mod tests;
