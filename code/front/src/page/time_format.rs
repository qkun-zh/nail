pub fn format_timestamp(created_at_secs: u64) -> String {
    nail_common::time::format_rfc3339_utc(created_at_secs.saturating_mul(1000))
        .unwrap_or_else(|_| created_at_secs.to_string())
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/time_format/tests.rs"]
mod tests;
