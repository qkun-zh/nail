use crate::request::envelope::{is_success, parse_envelope, unwrap_envelope};
use crate::request::error::{RequestError, RequestErrorKind};
use common::response::ResponseEnvelope;

#[test]
fn recognizes_success_codes_inclusively_below_three_hundred() {
    assert!(is_success(200));
    assert!(is_success(201));
    assert!(is_success(299));
    assert!(!is_success(300));
    assert!(!is_success(400));
    assert!(!is_success(500));
}

#[test]
fn unwraps_a_success_envelope_to_its_payload() {
    let envelope = ResponseEnvelope::ok(200, "payload".to_string(), "ok");
    assert_eq!(unwrap_envelope(envelope), Ok("payload".to_string()));
}

#[test]
fn empty_data_on_success_is_an_error() {
    let envelope = ResponseEnvelope::<String> {
        code: 200,
        data: None,
        message: "ok".to_string(),
    };
    let error = unwrap_envelope(envelope).unwrap_err();
    assert_eq!(error.kind, RequestErrorKind::EmptyData);
}

#[test]
fn non_success_code_surfaces_status_and_message() {
    let envelope = ResponseEnvelope::<String>::err(400, "bad request");
    let error = unwrap_envelope(envelope).unwrap_err();
    assert_eq!(error, RequestError::status(400, "bad request"));
    assert_eq!(error.kind, RequestErrorKind::Status);
}

#[test]
fn internal_error_message_is_surfaced() {
    let envelope = ResponseEnvelope::<String>::err(500, "internal server error");
    let error = unwrap_envelope(envelope).unwrap_err();
    assert_eq!(error.message, "[HTTP 500] internal server error");
}

#[test]
fn parses_a_wire_envelope() {
    let text = r#"{"code":200,"data":{"name":"alice"},"message":"ok"}"#;
    let envelope: ResponseEnvelope<serde_json::Value> = parse_envelope(text).expect("parse");
    assert_eq!(envelope.code, 200);
    assert!(envelope.data.is_some());
}

#[test]
fn rejects_invalid_json() {
    let error = parse_envelope::<serde_json::Value>("not json").unwrap_err();
    assert_eq!(error.kind, RequestErrorKind::Network);
}
