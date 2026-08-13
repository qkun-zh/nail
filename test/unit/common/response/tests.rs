use crate::response::ResponseEnvelope;

#[test]
fn ok_constructor_carries_code_data_and_message() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::ok(200, 42, "ok");
    assert_eq!(envelope.code, 200);
    assert_eq!(envelope.data, Some(42));
    assert_eq!(envelope.message, "ok");
}

#[test]
fn err_constructor_carries_code_and_message_with_null_data() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::err(404, "article not found");
    assert_eq!(envelope.code, 404);
    assert_eq!(envelope.data, None);
    assert_eq!(envelope.message, "article not found");
}

#[test]
fn ok_envelope_serializes_with_data_present() {
    let envelope = ResponseEnvelope::ok(200, 42u64, "ok");
    let json = serde_json::to_string(&envelope).expect("serialize ok envelope");
    assert_eq!(json, r##"{"code":200,"data":42,"message":"ok"}"##);
}

#[test]
fn err_envelope_serializes_with_null_data() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::err(404, "article not found");
    let json = serde_json::to_string(&envelope).expect("serialize err envelope");
    assert_eq!(
        json,
        r##"{"code":404,"data":null,"message":"article not found"}"##
    );
}

#[test]
fn envelope_round_trips_through_json() {
    let original = ResponseEnvelope::ok(201, vec!["a".to_string(), "b".to_string()], "created");
    let json = serde_json::to_string(&original).expect("serialize envelope");
    let decoded: ResponseEnvelope<Vec<String>> =
        serde_json::from_str(&json).expect("deserialize envelope");
    assert_eq!(decoded.code, original.code);
    assert_eq!(decoded.data, original.data);
    assert_eq!(decoded.message, original.message);
}
