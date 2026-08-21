use crate::page::time_format::format_timestamp;

#[test]
fn renders_in_utc_with_z_suffix() {
    assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_timestamp(1_700_000_000), "2023-11-14T22:13:20Z");
}

#[test]
fn falls_back_to_raw_seconds_when_formatting_fails() {
    assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
}
