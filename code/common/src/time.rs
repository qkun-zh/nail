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

/// Returns the current time as milliseconds since the Unix epoch.
///
/// # Errors
/// Returns an error if the system clock predates the Unix epoch.
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

/// Formats a UTC-millis timestamp as RFC 3339 in a given UTC offset.
///
/// # Errors
/// Returns an error if the offset is invalid, the timestamp is out of range,
/// or formatting fails.
pub fn format_rfc3339_with_offset(utc_ms: u64, offset_seconds: i32) -> anyhow::Result<String> {
    use time::format_description::well_known::Rfc3339;
    use time::{OffsetDateTime, UtcOffset};
    let offset = UtcOffset::from_whole_seconds(offset_seconds)?;
    let seconds = i64::try_from(utc_ms / 1000)?;
    let datetime = OffsetDateTime::from_unix_timestamp(seconds)?.to_offset(offset);
    Ok(datetime.format(&Rfc3339)?)
}

/// Formats a UTC-millis timestamp as RFC 3339 in UTC.
///
/// # Errors
/// Returns an error if the timestamp is out of range or formatting fails.
pub fn format_rfc3339_utc(utc_ms: u64) -> anyhow::Result<String> {
    format_rfc3339_with_offset(utc_ms, 0)
}

#[must_use]
pub fn parse_iso8601_utc_secs(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (body, offset) = split_timezone(trimmed)?;
    let (date_part, time_part) = match body.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (body.as_str(), None),
    };
    let mut date_parts = date_part.split('-');
    let year_raw = date_parts.next()?;
    let month_raw = date_parts.next().unwrap_or("01");
    let day_raw = date_parts.next().unwrap_or("01");
    if year_raw.len() != 4 || month_raw.len() != 2 || day_raw.len() != 2 {
        return None;
    }
    let year: i32 = year_raw.parse().ok()?;
    let month: u8 = month_raw.parse().ok()?;
    let day: u8 = day_raw.parse().ok()?;
    let (hour, minute, second) = match time_part {
        None => (0, 0, 0),
        Some(time) => {
            let mut time_parts = time.split(':');
            let hour: u8 = time_parts.next()?.parse().ok()?;
            let minute: u8 = time_parts.next().unwrap_or("0").parse().ok()?;
            let second: u8 = time_parts.next().unwrap_or("0").parse().ok()?;
            (hour, minute, second)
        }
    };
    let month = time::Month::try_from(month).ok()?;
    let date = time::Date::from_calendar_date(year, month, day).ok()?;
    let time = time::Time::from_hms(hour, minute, second).ok()?;
    let datetime = time::PrimitiveDateTime::new(date, time);
    let utc = match offset {
        Some(offset) => datetime
            .assume_offset(offset)
            .to_offset(time::UtcOffset::UTC),
        None => datetime.assume_utc(),
    };
    Some(utc.unix_timestamp())
}

fn split_timezone(input: &str) -> Option<(String, Option<time::UtcOffset>)> {
    if let Some(rest) = input.strip_suffix('Z').or_else(|| input.strip_suffix('z')) {
        return Some((rest.to_string(), None));
    }
    let Some((date_part, time_part)) = input.split_once('T') else {
        return Some((input.to_string(), None));
    };
    let timezone_index = time_part.rfind('+').or_else(|| time_part.rfind('-'));
    let Some(index) = timezone_index else {
        return Some((input.to_string(), None));
    };
    let sign = if time_part.as_bytes()[index] == b'+' {
        1
    } else {
        -1
    };
    let tail = &time_part[index + 1..];
    let (hours, minutes) = parse_offset_parts(tail)?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    let offset = time::UtcOffset::from_hms(sign * hours, sign * minutes, 0).ok()?;
    let body = format!("{}T{}", date_part, &time_part[..index]);
    Some((body, Some(offset)))
}

fn parse_offset_parts(tail: &str) -> Option<(i8, i8)> {
    match tail.len() {
        2 => {
            let hours = tail.parse::<i8>().ok()?;
            Some((hours, 0))
        }
        4 => {
            let hours = tail[..2].parse::<i8>().ok()?;
            let minutes = tail[2..].parse::<i8>().ok()?;
            Some((hours, minutes))
        }
        5 => {
            let (hour_part, minute_part) = tail.split_once(':')?;
            let hours = hour_part.parse::<i8>().ok()?;
            let minutes = minute_part.parse::<i8>().ok()?;
            Some((hours, minutes))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/time/tests.rs"]
mod tests;
