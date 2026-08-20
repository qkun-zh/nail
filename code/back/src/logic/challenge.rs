use cache::Challenge as CacheChallenge;
use nail_common::pow::Challenge;
use uuid::Uuid;

use crate::infrastructure::state::Configurator;

pub fn create_challenge(configurator: &Configurator, cache: &cache::Caches) -> Challenge {
    let id = Uuid::now_v7();
    cache.challenge.insert(&id.to_string(), CacheChallenge);
    Challenge {
        id,
        difficulty: configurator.pow_difficulty_iterations(),
    }
}
