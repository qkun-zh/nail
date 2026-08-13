
use super::*;
use crate::pow::{Challenge, Pow};
use uuid::Uuid;

fn pow() -> Pow {
    Pow {
        challenge: Challenge {
            id: Uuid::now_v7(),
            difficulty: 16,
        },
        solution: hex::encode(vec![0x42u8; 96]),
        payload: "p".to_string(),
    }
}

fn assert_roundtrip<T>(value: &T, expected_json: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).unwrap();
    assert_eq!(
        json, expected_json,
        "serialized shape must be the wire contract"
    );
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn token_request_wire_shape() {
    let pow_value = pow();
    let expected = format!(
        "{{\"pow\":{{\"challenge\":{{\"id\":\"{}\",\"difficulty\":16}},\
\"solution\":\"{}\",\"payload\":\"p\"}}}}",
        pow_value.challenge.id, pow_value.solution
    );
    assert_roundtrip(&TokenRequest { pow: pow_value }, &expected);
}

#[test]
fn email_update_send_request_has_two_pow_fields() {
    let req = EmailUpdateSendRequest {
        old_email_pow: pow(),
        new_email_pow: pow(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.starts_with("{\"old_email_pow\":{"));
    assert!(json.contains(",\"new_email_pow\":{"));
    let back: EmailUpdateSendRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn email_update_confirm_request_roundtrip() {
    let pow_value = pow();
    assert_roundtrip(
        &EmailUpdateConfirmRequest {
            pow: pow_value.clone(),
            old_email_token: "old-token".to_string(),
            new_email_token: "new-token".to_string(),
        },
        &format!(
            "{{\"pow\":{},\"old_email_token\":\"old-token\",\"new_email_token\":\"new-token\"}}",
            serde_json::to_string(&pow_value).unwrap()
        ),
    );
}

#[test]
fn simple_pow_requests_roundtrip() {
    for req in [
        serde_json::to_value(&CheckEmailRequest { pow: pow() }).unwrap(),
        serde_json::to_value(&DeregisterUserRequest { pow: pow() }).unwrap(),
        serde_json::to_value(&DeregisterUserConfirmRequest { pow: pow() }).unwrap(),
        serde_json::to_value(&NameSetRequest { pow: pow() }).unwrap(),
        serde_json::to_value(&LogoutRequest { pow: pow() }).unwrap(),
    ] {
        let obj = req.as_object().unwrap();
        assert_eq!(obj.len(), 1, "single pow field only");
        assert!(obj.contains_key("pow"));
    }
    let pow_value = pow();
    assert_roundtrip(
        &NameSetRequest {
            pow: pow_value.clone(),
        },
        &format!("{{\"pow\":{}}}", serde_json::to_string(&pow_value).unwrap()),
    );
}

#[test]
fn empty_body_requests_serialize_to_empty_object() {
    for value in [
        serde_json::to_value(&VerifySessionRequest {}).unwrap(),
        serde_json::to_value(&DeleteArticleRequest {}).unwrap(),
        serde_json::to_value(&DeleteCommentRequest {}).unwrap(),
    ] {
        assert_eq!(
            value,
            serde_json::json!({}),
            "empty struct must serialize to {{}}"
        );
    }
    let back: VerifySessionRequest = serde_json::from_str("{}").expect("{} must deserialize");
    assert_eq!(serde_json::to_string(&back).unwrap(), "{}");
}

#[test]
fn update_article_request_roundtrip() {
    assert_roundtrip(
        &UpdateArticleRequest {
            title: "t".to_string(),
            summary: "s".to_string(),
            tags: "#x#y".to_string(),
        },
        "{\"title\":\"t\",\"summary\":\"s\",\"tags\":\"#x#y\"}",
    );
    let back: UpdateArticleRequest =
        serde_json::from_str("{\"title\":\"t\",\"summary\":\"s\"}").unwrap();
    assert_eq!(back.tags, "");
}

#[test]
fn create_comment_request_roundtrip() {
    assert_roundtrip(
        &CreateCommentRequest {
            content: "hello".to_string(),
        },
        "{\"content\":\"hello\"}",
    );
}
