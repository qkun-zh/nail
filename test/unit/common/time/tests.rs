use crate::time::uuidv7_timestamp_ms;

#[test]
fn extracts_millisecond_timestamp_from_uuidv7() {
    let result = uuidv7_timestamp_ms("00000000-0001-7000-8000-000000000000");
    assert_eq!(result, Some(1));
}

#[test]
fn returns_none_for_non_uuid_strings() {
    assert_eq!(uuidv7_timestamp_ms("not-a-uuid"), None);
    assert_eq!(uuidv7_timestamp_ms(""), None);
}

#[test]
fn returns_none_for_non_v7_uuids() {
    assert_eq!(uuidv7_timestamp_ms("00000000-0000-4000-8000-000000000000"), None);
    assert_eq!(uuidv7_timestamp_ms("00000000-0000-1000-8000-000000000000"), None);
    assert_eq!(uuidv7_timestamp_ms("00000000-0000-6000-8000-000000000000"), None);
}

#[test]
fn extracts_second_timestamp_from_uuidv7() {
    let result = crate::time::uuidv7_timestamp_secs("00000000-0001-7000-8000-000000000000");
    assert_eq!(result, Some(0));
    let result = crate::time::uuidv7_timestamp_secs("00000000-03e8-7000-8000-000000000000");
    assert_eq!(result, Some(1));
}

#[test]
fn seconds_are_truncated_down_from_millis() {
    let uuid_string = "00000000-03e8-7000-8000-000000000000";
    assert_eq!(uuidv7_timestamp_ms(uuid_string), Some(1000));
    assert_eq!(crate::time::uuidv7_timestamp_secs(uuid_string), Some(1));
}

#[test]
fn min_uuidv7_for_ms_matches_known_literals() {
    assert_eq!(
        crate::time::uuidv7_min_for_ms(0),
        "00000000-0000-7000-8000-000000000000"
    );
    assert_eq!(
        crate::time::uuidv7_min_for_ms(1),
        "00000000-0001-7000-8000-000000000000"
    );
}

#[test]
fn max_uuidv7_for_ms_matches_known_literals() {
    assert_eq!(
        crate::time::uuidv7_max_for_ms(0),
        "00000000-0000-7fff-bfff-ffffffffffff"
    );
    assert_eq!(
        crate::time::uuidv7_max_for_ms(1),
        "00000000-0001-7fff-bfff-ffffffffffff"
    );
}

#[test]
fn min_and_max_preserve_the_encoded_millis() {
    for millis in [0u64, 1, 1000, 1_700_000_000_000] {
        let min = crate::time::uuidv7_min_for_ms(millis);
        let max = crate::time::uuidv7_max_for_ms(millis);
        assert_eq!(uuidv7_timestamp_ms(&min), Some(millis), "min for {millis}");
        assert_eq!(uuidv7_timestamp_ms(&max), Some(millis), "max for {millis}");
    }
}

#[test]
fn min_uuidv7_precedes_max_uuidv7() {
    for millis in [0u64, 1, 1000, 1_700_000_000_000] {
        let min = crate::time::uuidv7_min_for_ms(millis);
        let max = crate::time::uuidv7_max_for_ms(millis);
        assert!(min < max, "min {min} should precede max {max}");
    }
}

#[test]
fn bounds_contain_real_time_uuids() -> Result<(), std::time::SystemTimeError> {
    let before = crate::time::now_ms()?;
    let real = uuid::Uuid::now_v7().to_string();
    let after = crate::time::now_ms()?;
    let min = crate::time::uuidv7_min_for_ms(before);
    let max = crate::time::uuidv7_max_for_ms(after);
    assert!(min <= real, "{min} should precede {real}");
    assert!(real <= max, "{real} should precede {max}");
    Ok(())
}

#[test]
fn formats_epoch_with_zero_offset_as_utc_z() {
    let formatted = crate::time::format_rfc3339_with_offset(0, 0).expect("format epoch");
    assert_eq!(formatted, "1970-01-01T00:00:00Z");
}

#[test]
fn formats_with_positive_and_negative_offsets() {
    let plus_eight = crate::time::format_rfc3339_with_offset(0, 28_800).expect("format +08:00");
    assert_eq!(plus_eight, "1970-01-01T08:00:00+08:00");
    let minus_one = crate::time::format_rfc3339_with_offset(0, -3_600).expect("format -01:00");
    assert_eq!(minus_one, "1969-12-31T23:00:00-01:00");
    let india = crate::time::format_rfc3339_with_offset(0, 19_800).expect("format +05:30");
    assert_eq!(india, "1970-01-01T05:30:00+05:30");
}

#[test]
fn formats_a_known_wall_clock_time() {
    let utc = crate::time::format_rfc3339_with_offset(1_700_000_000_000, 0).expect("format utc");
    assert_eq!(utc, "2023-11-14T22:13:20Z");
    let beijing = crate::time::format_rfc3339_with_offset(1_700_000_000_000, 28_800)
        .expect("format beijing");
    assert_eq!(beijing, "2023-11-15T06:13:20+08:00");
}

#[test]
fn truncates_sub_second_millis_to_whole_seconds() {
    let exact = crate::time::format_rfc3339_with_offset(1_700_000_000_000, 0).expect("exact");
    let with_fraction = crate::time::format_rfc3339_with_offset(1_700_000_000_999, 0)
        .expect("with fraction");
    assert_eq!(with_fraction, exact);
}

#[test]
fn rejects_offsets_not_representable_in_rfc3339() {
    for offset in [86_400, 86_399, 90_000, -86_400, -86_399, -90_000, i32::MAX, i32::MIN] {
        let result = crate::time::format_rfc3339_with_offset(0, offset);
        assert!(result.is_err(), "offset {offset} must be rejected");
    }
}

#[test]
fn accepts_the_extreme_valid_offsets() {
    let max = crate::time::format_rfc3339_with_offset(0, 86_340).expect("max offset");
    assert_eq!(max, "1970-01-01T23:59:00+23:59");
    let min = crate::time::format_rfc3339_with_offset(0, -86_340).expect("min offset");
    assert_eq!(min, "1969-12-31T00:01:00-23:59");
}
