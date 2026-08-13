
use crate::repo::token::TokenCaches;

pub fn create_challenge(caches: &TokenCaches, id: &str) {
    caches.challenge.insert(id.to_string(), ());
}

pub fn consume_challenge(caches: &TokenCaches, id: &str) -> bool {
    let result = caches
        .challenge
        .entry(id.to_string())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(_) => moka::ops::compute::Op::Remove,
            None => moka::ops::compute::Op::Nop,
        });
    matches!(result, moka::ops::compute::CompResult::Removed(_))
}
