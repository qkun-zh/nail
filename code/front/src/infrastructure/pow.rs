use nail_common::pow::{Challenge, Pow};

pub fn prove(challenge: &Challenge) -> Result<Pow, String> {
    nail_common::pow::prove(challenge).map_err(|error| format!("proof of work failed: {error}"))
}

#[cfg(test)]
#[path = "../../../../test/unit/front/infrastructure/pow/tests.rs"]
mod tests;
