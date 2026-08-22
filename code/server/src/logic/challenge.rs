use cache::Challenge as CacheChallenge;
use common::pow::{Challenge, issue_challenge};

use crate::infrastructure::state::Configurator;

pub fn create_challenge(configurator: &Configurator, cache: &cache::Cache) -> Challenge {
    let challenge = issue_challenge(configurator.pow_difficulty_iterations());
    cache
        .challenge
        .insert(&challenge.id.to_string(), CacheChallenge);
    challenge
}
