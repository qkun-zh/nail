pub fn format_timestamp(created_at_secs: u64) -> String {
    common::time::format_rfc3339_utc(created_at_secs.saturating_mul(1000))
        .unwrap_or_else(|_| created_at_secs.to_string())
}

#[cfg(test)]
#[path = "time_format_tests.rs"]
mod tests;
