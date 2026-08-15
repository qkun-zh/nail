use uuid::Uuid;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn uuidv7_timestamp_ms(uuid_string: &str) -> Option<u64> {
    let uuid = Uuid::parse_str(uuid_string).ok()?;
    let timestamp = uuid.get_timestamp()?;
    let (seconds, nanos) = timestamp.to_unix();
    Some(seconds * 1000 + u64::from(nanos / 1_000_000))
}

pub fn uuidv7_timestamp_secs(uuid_string: &str) -> Option<u64> {
    uuidv7_timestamp_ms(uuid_string).map(|ms| ms / 1000)
}

pub fn uuidv7_min_for_ms(ms: u64) -> String {
    uuidv7_for_ms(ms, 0x00)
}

pub fn uuidv7_max_for_ms(ms: u64) -> String {
    uuidv7_for_ms(ms, 0xff)
}

fn uuidv7_for_ms(ms: u64, fill: u8) -> String {
    let timestamp_ms = ms & 0x0000_FFFF_FFFF_FFFF;
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

#[cfg(test)]
#[path = "../../../test/unit/common/time/tests.rs"]
mod tests;
