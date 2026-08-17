use axum::http::StatusCode;

use crate::logic::error::LogicError;

#[test]
fn every_variant_maps_to_its_status_code() {
    let cases = [
        (LogicError::bad_request("bad"), StatusCode::BAD_REQUEST),
        (LogicError::unauthorized("unauth"), StatusCode::UNAUTHORIZED),
        (LogicError::forbidden("denied"), StatusCode::FORBIDDEN),
        (LogicError::not_found("missing"), StatusCode::NOT_FOUND),
        (
            LogicError::internal("boom"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.status(), expected);
    }
}

#[test]
fn internal_error_is_masked_in_the_envelope_pair() {
    let (status, message) = LogicError::internal("secret database detail").into_pair();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(message, "internal server error");
}

#[test]
fn non_internal_errors_keep_their_message_in_the_envelope_pair() {
    let (status, message) = LogicError::not_found("article not found").into_pair();
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(message, "article not found");
}

#[test]
fn message_exposes_the_reason_for_every_variant() {
    assert_eq!(LogicError::bad_request("x").message(), "x");
    assert_eq!(LogicError::unauthorized("y").message(), "y");
    assert_eq!(LogicError::forbidden("z").message(), "z");
    assert_eq!(LogicError::not_found("w").message(), "w");
    assert_eq!(LogicError::internal("v").message(), "v");
}

#[test]
fn display_delegates_to_message() {
    let error = LogicError::bad_request("something went wrong");
    assert_eq!(format!("{error}"), "something went wrong");
}

#[test]
fn database_error_wraps_the_cause_as_internal() {
    let error = crate::logic::error::database_error("connection refused");
    assert_eq!(
        error,
        LogicError::internal("database query failed: connection refused")
    );
}
