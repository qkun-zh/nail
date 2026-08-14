use nail_common::pow::{Pow, ProveInput};

pub fn prove(input: ProveInput) -> Result<Pow, String> {
    nail_common::pow::prove(input).map_err(|error| format!("proof of work failed: {error}"))
}

#[cfg(test)]
#[path = "../../../../test/unit/front/infrastructure/pow/tests.rs"]
mod tests;
