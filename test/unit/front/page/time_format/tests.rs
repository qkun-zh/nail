use crate::page::time_format::format_timestamp;

#[test]
fn renders_with_the_configured_offset() {
    assert_eq!(format_timestamp(0, 28_800), "1970-01-01T08:00:00+08:00");
    assert_eq!(format_timestamp(0, 0), "1970-01-01T00:00:00Z");
    assert_eq!(
        format_timestamp(1_700_000_000, 28_800),
        "2023-11-15T06:13:20+08:00"
    );
}

#[test]
fn falls_back_to_raw_millis_when_formatting_fails() {
    assert_eq!(format_timestamp(0, 86_400), "0");
}
