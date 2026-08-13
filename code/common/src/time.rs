use uuid::{Uuid, Version};

pub fn uuidv7_timestamp_ms(uuid_string: &str) -> Option<u64> {
    let uuid = Uuid::parse_str(uuid_string).ok()?;
    if uuid.get_version() != Some(Version::SortRand) {
        return None;
    }
    let timestamp = uuid.get_timestamp()?;
    let (seconds, nanos) = timestamp.to_unix();
    Some(seconds * 1000 + u64::from(nanos / 1_000_000))
}

pub fn uuidv7_timestamp_secs(uuid_string: &str) -> Option<u64> {
    uuidv7_timestamp_ms(uuid_string).map(|millis| millis / 1000)
}

pub fn now_ms() -> Result<u64, std::time::SystemTimeError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
}

pub fn uuidv7_min_for_ms(millis: u64) -> String {
    uuidv7_for_ms(millis, 0x00)
}

pub fn uuidv7_max_for_ms(millis: u64) -> String {
    uuidv7_for_ms(millis, 0xff)
}

fn uuidv7_for_ms(millis: u64, fill: u8) -> String {
    let timestamp_ms = millis & 0x0000_FFFF_FFFF_FFFF;
    let mut bytes = [fill; 16];
    bytes[0] = (timestamp_ms >> 40) as u8;
    bytes[1] = (timestamp_ms >> 32) as u8;
    bytes[2] = (timestamp_ms >> 24) as u8;
    bytes[3] = (timestamp_ms >> 16) as u8;
    bytes[4] = (timestamp_ms >> 8) as u8;
    bytes[5] = timestamp_ms as u8;
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    Uuid::from_bytes(bytes).hyphenated().to_string()
}

pub fn format_rfc3339_with_offset(utc_ms: u64, offset_seconds: i32) -> anyhow::Result<String> {
    use time::format_description::well_known::Rfc3339;
    use time::{OffsetDateTime, UtcOffset};
    let offset = UtcOffset::from_whole_seconds(offset_seconds)?;
    let seconds = (utc_ms / 1000) as i64;
    let datetime = OffsetDateTime::from_unix_timestamp(seconds)?.to_offset(offset);
    Ok(datetime.format(&Rfc3339)?)
}

#[cfg(test)]
#[path = "../../../test/unit/common/time/tests.rs"]
mod tests;
