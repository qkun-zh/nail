use std::time::Instant;

use crate::repo::token::{TokenCaches, reverse_add, reverse_remove};
use crate::repo::types::DeregisterTokenEntry;

fn key(token: &str) -> String {
    common::hash::token(token)
}

pub fn create_deregister_token(
    caches: &TokenCaches,
    token: &str,
    user_id: &str,
    email_address_hash: &str,
) {
    let key = key(token);
    let now = Instant::now();
    let ttl = caches
        .deregister
        .policy()
        .time_to_live()
        .expect("deregister cache must be TTL-bound");
    reverse_add(&caches.deregister_by_user, user_id, &key, now + ttl);
    caches.deregister.insert(
        key,
        DeregisterTokenEntry {
            user_id: user_id.to_string(),
            email_address_hash: email_address_hash.to_string(),
        },
    );
}

pub fn consume_deregister_token(caches: &TokenCaches, token: &str) -> Option<DeregisterTokenEntry> {
    let key = key(token);
    let result = caches
        .deregister
        .entry(key.clone())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(_) => moka::ops::compute::Op::Remove,
            None => moka::ops::compute::Op::Nop,
        });
    let moka::ops::compute::CompResult::Removed(entry) = result else {
        return None;
    };
    let entry = entry.into_value();
    reverse_remove(&caches.deregister_by_user, &entry.user_id, &key);
    Some(entry)
}

pub fn find_user_id_by_deregister_token(caches: &TokenCaches, token: &str) -> Option<String> {
    caches
        .deregister
        .get(&key(token))
        .map(|entry| entry.user_id)
}

pub fn delete_deregister_tokens_by_user_id(caches: &TokenCaches, user_id: &str) -> u64 {
    let Some(members) = caches.deregister_by_user.get(user_id) else {
        return 0;
    };
    let count = members.len() as u64;
    caches.deregister_by_user.invalidate(user_id);
    for member in &members {
        caches.deregister.invalidate(&member.token);
    }
    count
}
