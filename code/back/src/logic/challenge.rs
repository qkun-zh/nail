use nail_common::pow::Challenge;
use uuid::Uuid;

use crate::infrastructure::config::server::ServerConfig;
use crate::repository::cache::{ChallengeEntry, TokenCaches};

pub fn issue_challenge(config: &ServerConfig, caches: &TokenCaches) -> Challenge {
    let id = Uuid::now_v7();
    caches.challenge.insert(&id.to_string(), ChallengeEntry);
    tracing::info!(
        challenge_id = %id,
        difficulty = config.pow_difficulty_iterations,
        "challenge issued"
    );
    Challenge {
        id,
        difficulty: config.pow_difficulty_iterations,
    }
}
