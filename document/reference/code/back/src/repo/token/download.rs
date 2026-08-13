use common::hash;

use crate::repo::token::TokenCaches;
use crate::repo::types::DownloadTokenEntry;

pub fn create_download_token(caches: &TokenCaches, token: &str, version_id: &str, user_id: &str) {
    caches.download.insert(
        hash::token(token),
        DownloadTokenEntry {
            version_id: version_id.to_string(),
            user_id: user_id.to_string(),
        },
    );
}

pub fn find_download_token(caches: &TokenCaches, token: &str) -> Option<DownloadTokenEntry> {
    caches.download.get(&hash::token(token))
}

pub fn consume_download_token(caches: &TokenCaches, token: &str) -> Option<DownloadTokenEntry> {
    let result = caches
        .download
        .entry(hash::token(token))
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(_) => moka::ops::compute::Op::Remove,
            None => moka::ops::compute::Op::Nop,
        });
    let moka::ops::compute::CompResult::Removed(entry) = result else {
        return None;
    };
    Some(entry.into_value())
}
