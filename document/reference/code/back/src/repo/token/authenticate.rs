use std::time::Instant;

use crate::repo::token::{TokenCaches, reverse_add, reverse_remove};
use crate::repo::types::AuthenticateTokenEntry;

fn key(token: &str) -> String {
    common::hash::token(token)
}

pub fn create_authenticate_token(
    caches: &TokenCaches,
    token: &str,
    email_address_hash: &str,
    email_subject: &str,
) {
    let key = key(token);
    let now = Instant::now();
    let ttl = caches
        .authenticate
        .policy()
        .time_to_live()
        .expect("authenticate cache must be TTL-bound");
    reverse_add(
        &caches.authenticate_by_email_hash,
        email_address_hash,
        &key,
        now + ttl,
    );
    caches.authenticate.insert(
        key,
        AuthenticateTokenEntry {
            email_address_hash: email_address_hash.to_string(),
            email_subject: email_subject.to_string(),
        },
    );
}

pub fn consume_authenticate_token(
    caches: &TokenCaches,
    token: &str,
) -> Option<AuthenticateTokenEntry> {
    let key = key(token);
    let result = caches
        .authenticate
        .entry(key.clone())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(_) => moka::ops::compute::Op::Remove,
            None => moka::ops::compute::Op::Nop,
        });
    let moka::ops::compute::CompResult::Removed(entry) = result else {
        return None;
    };
    let entry = entry.into_value();
    reverse_remove(
        &caches.authenticate_by_email_hash,
        &entry.email_address_hash,
        &key,
    );
    Some(entry)
}

pub fn delete_authenticate_tokens_by_email_address_hash(
    caches: &TokenCaches,
    email_address_hash: &str,
) -> u64 {
    let Some(members) = caches.authenticate_by_email_hash.get(email_address_hash) else {
        return 0;
    };
    let count = members.len() as u64;
    caches
        .authenticate_by_email_hash
        .invalidate(email_address_hash);
    for member in &members {
        caches.authenticate.invalidate(&member.token);
    }
    count
}
