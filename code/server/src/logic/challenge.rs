use cache::Challenge as CacheChallenge;
use common::pow::{Challenge, issue_challenge};

use crate::infrastructure::config::AppConfig;

pub fn create_challenge(config: &AppConfig, cache: &cache::Cache) -> Challenge {
    let challenge = issue_challenge(config.server.pow_difficulty_iterations);
    cache
        .challenge
        .insert(&challenge.id.to_string(), CacheChallenge);
    challenge
}
