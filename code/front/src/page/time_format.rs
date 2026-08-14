pub fn format_timestamp(utc_ms: u64, offset_seconds: i32) -> String {
    nail_common::time::format_rfc3339_with_offset(utc_ms, offset_seconds)
        .unwrap_or_else(|_| utc_ms.to_string())
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/time_format/tests.rs"]
mod tests;
