
use super::*;

fn assert_roundtrip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) {
    let json = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn ok_response_ok_shape_is_nullable_session_token() {
    assert_eq!(
        serde_json::to_string(&OkResponse::ok()).unwrap(),
        r#"{"ok":true,"session_token":null}"#
    );
    let with_session = OkResponse {
        ok: true,
        session_token: Some("0196f71a-4c1c-7f00-8000-000000000001".to_string()),
    };
    assert_roundtrip(&with_session);
}

#[test]
fn error_response_shape() {
    assert_eq!(
        serde_json::to_string(&ErrorResponse {
            ok: false,
            reason: "bad token".to_string(),
        })
        .unwrap(),
        r#"{"ok":false,"reason":"bad token"}"#
    );
    assert_roundtrip(&ErrorResponse {
        ok: false,
        reason: String::new(),
    });
}

#[test]
fn pow_response_shape() {
    assert_eq!(
        serde_json::to_string(&PowResponse {
            ok: true,
            email_subject: Some("subject-1".to_string()),
        })
        .unwrap(),
        r#"{"ok":true,"email_subject":"subject-1"}"#
    );
    assert_roundtrip(&PowResponse {
        ok: true,
        email_subject: None,
    });
}

#[test]
fn multi_field_responses_roundtrip() {
    let cases: Vec<(&str, String)> = vec![
        (
            "email_update_send",
            serde_json::to_string(&EmailUpdateSendResponse {
                ok: true,
                reason: None,
                old_email_subject: Some("o".to_string()),
                new_email_subject: Some("n".to_string()),
            })
            .unwrap(),
        ),
        (
            "email_update_confirm",
            serde_json::to_string(&EmailUpdateConfirmResponse {
                ok: true,
                session_token: Some("s".to_string()),
                reason: None,
            })
            .unwrap(),
        ),
        (
            "check_email",
            serde_json::to_string(&CheckEmailResponse {
                ok: true,
                matches: true,
                reason: None,
            })
            .unwrap(),
        ),
        (
            "deregister_user",
            serde_json::to_string(&DeregisterUserResponse {
                ok: true,
                reason: None,
                email_subject: Some("d".to_string()),
            })
            .unwrap(),
        ),
        (
            "name",
            serde_json::to_string(&NameResponse {
                ok: true,
                name: "alice".to_string(),
                reason: None,
            })
            .unwrap(),
        ),
    ];
    for (label, json) in cases {
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["ok"], true, "{label}: ok key must serialize");
        match label {
            "email_update_send" => {
                let v: EmailUpdateSendResponse = serde_json::from_str(&json).unwrap();
                assert_eq!(serde_json::to_string(&v).unwrap(), json);
            }
            "email_update_confirm" => {
                let v: EmailUpdateConfirmResponse = serde_json::from_str(&json).unwrap();
                assert_eq!(serde_json::to_string(&v).unwrap(), json);
            }
            "check_email" => {
                let v: CheckEmailResponse = serde_json::from_str(&json).unwrap();
                assert_eq!(serde_json::to_string(&v).unwrap(), json);
            }
            "deregister_user" => {
                let v: DeregisterUserResponse = serde_json::from_str(&json).unwrap();
                assert_eq!(serde_json::to_string(&v).unwrap(), json);
            }
            "name" => {
                let v: NameResponse = serde_json::from_str(&json).unwrap();
                assert_eq!(serde_json::to_string(&v).unwrap(), json);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn display_shapes_are_compact() {
    assert_eq!(OkResponse::ok().to_string(), "{ok: true}");
    assert_eq!(
        ErrorResponse {
            ok: false,
            reason: "boom".to_string(),
        }
        .to_string(),
        "{ok: false, reason: boom}"
    );
    assert_eq!(
        PowResponse {
            ok: false,
            email_subject: None,
        }
        .to_string(),
        "{ok: false}"
    );
}
