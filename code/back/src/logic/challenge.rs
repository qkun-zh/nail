use nail_common::pow::Challenge;
use uuid::Uuid;

use crate::infrastructure::state::Configurator;
use crate::repository::cache::{ChallengeEntry, TokenCaches};

pub fn create_challenge(configurator: &Configurator, cache: &TokenCaches) -> Challenge {
    let id = Uuid::now_v7();
    cache.challenge.insert(&id.to_string(), ChallengeEntry);
    Challenge {
        id,
        difficulty: configurator.pow_difficulty_iterations(),
    }
}
