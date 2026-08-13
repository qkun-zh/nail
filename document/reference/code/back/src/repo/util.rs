
pub(crate) fn strip_record_id(record_id: &str) -> String {
    let key = record_id
        .rsplit_once(':')
        .map(|(_, key)| key)
        .unwrap_or(record_id);
    key.trim_matches('`').to_string()
}

pub(crate) fn content_hash_rel_path(hash: &str) -> Option<String> {
    let valid = hash.len() == 32
        && hash
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if !valid {
        return None;
    }
    Some(format!("{}/{}/{}.pdf", &hash[0..2], &hash[2..4], hash))
}
