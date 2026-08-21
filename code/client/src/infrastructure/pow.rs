use common::pow::{Challenge, Pow};

pub fn prove(challenge: &Challenge) -> Result<Pow, String> {
    common::pow::prove(challenge).map_err(|error| format!("proof of work failed: {error}"))
}

#[cfg(test)]
#[path = "pow_tests.rs"]
mod tests;
