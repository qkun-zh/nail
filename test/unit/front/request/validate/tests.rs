use uuid::Uuid;

use crate::request::error::RequestErrorKind;
use crate::request::validate::validate_id;

#[test]
fn accepts_canonical_uuid() {
    let id = "01a018c7-f177-7da1-a821-5f1000648383";
    let validated = validate_id(id, "user_id").expect("valid uuid accepted");
    assert_eq!(validated, id);
}

#[test]
fn accepts_uppercase_and_normalizes_to_lowercase() {
    let id = "01A018C7-F177-7DA1-A821-5F1000648383";
    let validated = validate_id(id, "user_id").expect("uppercase uuid accepted");
    assert_eq!(validated, "01a018c7-f177-7da1-a821-5f1000648383");
}

#[test]
fn accepts_whitespace_padding() {
    let validated = validate_id(" 01a018c7-f177-7da1-a821-5f1000648383 ", "user_id")
        .expect("whitespace-padded uuid accepted");
    assert_eq!(validated, "01a018c7-f177-7da1-a821-5f1000648383");
}

#[test]
fn rejects_invalid_format() {
    for raw in [
        "hi",
        "abc123",
        "not-a-uuid",
        "",
        "  ",
        "01a018c7-f177-7da1-a821-5f10006483",
    ] {
        let error = validate_id(raw, "user_id").expect_err("invalid id rejected");
        assert_eq!(error.kind, RequestErrorKind::Status);
    }
}

#[test]
fn rejects_legit_looking_but_wrong_length() {
    let error = validate_id("01a018c7-f177-7da1-a821-5f10006483830000", "user_id")
        .expect_err("wrong-length uuid rejected");
    assert_eq!(error.kind, RequestErrorKind::Status);
}

#[test]
fn parse_roundtrip_matches_uuid_crate() {
    let id = "01a018c7-f177-7da1-a821-5f1000648383";
    let parsed = Uuid::parse_str(id).expect("uuid parses");
    assert_eq!(
        validate_id(id, "user_id").expect("valid"),
        parsed.to_string()
    );
}
