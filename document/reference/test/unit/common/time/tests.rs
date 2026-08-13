
use super::*;
use uuid::Uuid;

#[test]
fn now_ms_is_recent_and_monotone_over_calls() {
    let first_sample = now_ms();
    let second_sample = now_ms();
    let third_sample = now_ms();
    assert!(
        second_sample >= first_sample,
        "now_ms must never go backwards"
    );
    assert!(third_sample >= second_sample);
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(
        wall.abs_diff(third_sample) < 5_000,
        "now_ms must track wall clock"
    );
}

#[test]
fn uuidv7_timestamp_roundtrips_millisecond_precision() {
    let now = now_ms();
    let uuid = Uuid::now_v7();
    let parsed = uuidv7_timestamp_ms(&uuid.to_string()).unwrap();
    assert!(now.abs_diff(parsed) <= 2, "parsed {parsed} vs now {now}");
    assert_eq!(
        uuidv7_timestamp_secs(&uuid.to_string()).unwrap(),
        parsed / 1000
    );
}

#[test]
fn uuidv7_timestamp_rejects_non_uuid_and_non_v7() {
    assert_eq!(uuidv7_timestamp_ms("not-a-uuid"), None);
    assert_eq!(uuidv7_timestamp_ms(""), None);
    let v4 = Uuid::new_v4().to_string();
    assert_eq!(uuidv7_timestamp_ms(&v4), None);
}

#[test]
fn min_max_window_pins_the_same_millisecond() {
    let ms = 1_700_000_000_123u64;
    let min = uuidv7_min_for_ms(ms);
    let max = uuidv7_max_for_ms(ms);
    assert_eq!(uuidv7_timestamp_ms(&min).unwrap(), ms);
    assert_eq!(uuidv7_timestamp_ms(&max).unwrap(), ms);
    assert!(min < max, "min {min} must sort before max {max}");
    assert!(uuidv7_max_for_ms(ms) < uuidv7_min_for_ms(ms + 1));
}

#[test]
fn min_max_are_version7_variant_rfc() {
    for s in [uuidv7_min_for_ms(1), uuidv7_max_for_ms(1)] {
        let u = Uuid::parse_str(&s).unwrap();
        assert_eq!(u.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }
}
