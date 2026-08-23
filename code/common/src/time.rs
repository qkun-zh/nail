use uuid::{Uuid, Version};

#[must_use]
pub fn uuidv7_timestamp_ms(uuid_string: &str) -> Option<u64> {
    let uuid = Uuid::parse_str(uuid_string).ok()?;
    if uuid.get_version() != Some(Version::SortRand) {
        return None;
    }
    let timestamp = uuid.get_timestamp()?;
    let (seconds, nanos) = timestamp.to_unix();
    Some(seconds * 1000 + u64::from(nanos / 1_000_000))
}

#[must_use]
pub fn uuidv7_timestamp_secs(uuid_string: &str) -> Option<u64> {
    uuidv7_timestamp_ms(uuid_string).map(|millis| millis / 1000)
}

#[must_use]
pub fn uuidv7_secs_or_zero(id: &str) -> u64 {
    uuidv7_timestamp_secs(id).unwrap_or(0)
}

pub fn now_ms() -> Result<u64, std::time::SystemTimeError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
}

#[must_use]
pub fn uuidv7_min_for_ms(millis: u64) -> String {
    uuidv7_for_ms(millis, 0x00)
}

#[must_use]
pub fn uuidv7_max_for_ms(millis: u64) -> String {
    uuidv7_for_ms(millis, 0xff)
}

fn uuidv7_for_ms(millis: u64, fill: u8) -> String {
    let timestamp_ms = millis & 0x0000_FFFF_FFFF_FFFF;
    let timestamp_bytes = timestamp_ms.to_be_bytes();
    let mut bytes = [fill; 16];
    bytes[..6].copy_from_slice(&timestamp_bytes[2..8]);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    Uuid::from_bytes(bytes).hyphenated().to_string()
}

pub fn format_rfc3339_with_offset(utc_ms: u64, offset_seconds: i32) -> anyhow::Result<String> {
    use time::format_description::well_known::Rfc3339;
    use time::{OffsetDateTime, UtcOffset};
    let offset = UtcOffset::from_whole_seconds(offset_seconds)?;
    let seconds = i64::try_from(utc_ms / 1000)?;
    let datetime = OffsetDateTime::from_unix_timestamp(seconds)?.to_offset(offset);
    Ok(datetime.format(&Rfc3339)?)
}

pub fn format_rfc3339_utc(utc_ms: u64) -> anyhow::Result<String> {
    format_rfc3339_with_offset(utc_ms, 0)
}

#[must_use]
pub fn parse_iso8601_utc_secs(input: &str) -> Option<i64> {
    use time::format_description::well_known::Iso8601;
    use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(datetime) = OffsetDateTime::parse(trimmed, &Iso8601::DEFAULT) {
        return Some(datetime.to_offset(UtcOffset::UTC).unix_timestamp());
    }
    if let Ok(datetime) = PrimitiveDateTime::parse(trimmed, &Iso8601::DEFAULT) {
        return Some(datetime.assume_utc().unix_timestamp());
    }
    if let Ok(date) = Date::parse(trimmed, &Iso8601::DEFAULT) {
        return Some(date.midnight().assume_utc().unix_timestamp());
    }
    None
}

#[cfg(test)]
#[path = "time_tests.rs"]
mod tests;
