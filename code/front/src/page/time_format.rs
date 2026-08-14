pub fn format_timestamp(created_at_secs: u64, offset_seconds: i32) -> String {
    nail_common::time::format_rfc3339_with_offset(
        created_at_secs.saturating_mul(1000),
        offset_seconds,
    )
    .unwrap_or_else(|_| created_at_secs.to_string())
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/time_format/tests.rs"]
mod tests;
