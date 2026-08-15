use crate::repo::token::TokenCaches;
use crate::repo::types::EmailUpdateTokenEntry;

pub fn create_email_update_token(
    caches: &TokenCaches,
    user_id: &str,
    old_email_address_hash: &str,
    new_email_address_hash: &str,
    token_from_old_email_hash: &str,
    token_from_new_email_hash: &str,
) {
    caches.email_update.insert(
        user_id.to_string(),
        EmailUpdateTokenEntry {
            old_email_address_hash: old_email_address_hash.to_string(),
            new_email_address_hash: new_email_address_hash.to_string(),
            token_from_old_email_hash: token_from_old_email_hash.to_string(),
            token_from_new_email_hash: token_from_new_email_hash.to_string(),
        },
    );
}

pub fn read_email_update_token(
    caches: &TokenCaches,
    user_id: &str,
) -> Option<EmailUpdateTokenEntry> {
    caches.email_update.get(user_id)
}

pub fn consume_email_update_token_if_matches(
    caches: &TokenCaches,
    user_id: &str,
    token_from_old_email_hash: &str,
    token_from_new_email_hash: &str,
) -> Option<EmailUpdateTokenEntry> {
    let result = caches
        .email_update
        .entry(user_id.to_string())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(entry) => {
                let row = entry.into_value();
                if row.token_from_old_email_hash == token_from_old_email_hash
                    && row.token_from_new_email_hash == token_from_new_email_hash
                {
                    moka::ops::compute::Op::Remove
                } else {
                    moka::ops::compute::Op::Nop
                }
            }
            None => moka::ops::compute::Op::Nop,
        });
    let moka::ops::compute::CompResult::Removed(entry) = result else {
        return None;
    };
    Some(entry.into_value())
}

pub fn delete_email_update_token(caches: &TokenCaches, user_id: &str) -> u64 {
    u64::from(caches.email_update.remove(user_id).is_some())
}
