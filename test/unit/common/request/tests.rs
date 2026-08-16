use crate::request::{DeleteMode, TokenPurpose};

#[test]
fn delete_mode_serializes_as_lowercase_strings() -> anyhow::Result<()> {
    assert_eq!(
        serde_json::to_string(&DeleteMode::Transfer)?,
        r#""transfer""#
    );
    assert_eq!(serde_json::to_string(&DeleteMode::Hard)?, r#""hard""#);
    assert_eq!(serde_json::to_string(&DeleteMode::Soft)?, r#""soft""#);
    Ok(())
}

#[test]
fn delete_mode_deserializes_from_lowercase_strings() -> anyhow::Result<()> {
    assert_eq!(
        serde_json::from_str::<DeleteMode>(r#""transfer""#)?,
        DeleteMode::Transfer
    );
    assert_eq!(
        serde_json::from_str::<DeleteMode>(r#""hard""#)?,
        DeleteMode::Hard
    );
    assert_eq!(
        serde_json::from_str::<DeleteMode>(r#""soft""#)?,
        DeleteMode::Soft
    );
    Ok(())
}

#[test]
fn delete_mode_rejects_unknown_values() {
    for value in [r#""Transfer""#, r#""HARD""#, r#""SOFT""#, r#""""#] {
        let result = serde_json::from_str::<DeleteMode>(value);
        assert!(result.is_err(), "value {value} must be rejected");
    }
}

#[test]
fn token_purpose_serializes_as_snake_case_strings() -> anyhow::Result<()> {
    assert_eq!(
        serde_json::to_string(&TokenPurpose::CreateUser)?,
        r#""create_user""#
    );
    assert_eq!(
        serde_json::to_string(&TokenPurpose::UpdateUserEmail)?,
        r#""update_user_email""#
    );
    assert_eq!(
        serde_json::to_string(&TokenPurpose::DeleteUser)?,
        r#""delete_user""#
    );
    Ok(())
}

#[test]
fn token_purpose_deserializes_from_snake_case_strings() -> anyhow::Result<()> {
    assert_eq!(
        serde_json::from_str::<TokenPurpose>(r#""create_user""#)?,
        TokenPurpose::CreateUser
    );
    assert_eq!(
        serde_json::from_str::<TokenPurpose>(r#""update_user_email""#)?,
        TokenPurpose::UpdateUserEmail
    );
    assert_eq!(
        serde_json::from_str::<TokenPurpose>(r#""delete_user""#)?,
        TokenPurpose::DeleteUser
    );
    Ok(())
}

#[test]
fn token_purpose_rejects_unknown_values() {
    for value in [
        r#""create-user""#,
        r#""updateUserEmail""#,
        r#""CreateUser""#,
        r#""""#,
    ] {
        let result = serde_json::from_str::<TokenPurpose>(value);
        assert!(result.is_err(), "value {value} must be rejected");
    }
}

fn sample_pow() -> anyhow::Result<crate::pow::Pow> {
    Ok(crate::pow::Pow {
        challenge: crate::pow::Challenge {
            id: uuid::Uuid::parse_str("0197c0b0-1234-7000-8000-000000000001")?,
            difficulty: 1,
        },
        solution: "ab".repeat(96),
        payload: "hello".to_string(),
    })
}

#[test]
fn delete_body_round_trips_with_mode() -> anyhow::Result<()> {
    let body = crate::request::DeleteBody {
        mode: Some(DeleteMode::Transfer),
    };
    assert_eq!(serde_json::to_string(&body)?, r#"{"mode":"transfer"}"#);
    let decoded: crate::request::DeleteBody = serde_json::from_str(r#"{"mode":"transfer"}"#)?;
    assert_eq!(decoded, body);
    Ok(())
}

#[test]
fn delete_body_round_trips_without_mode() -> anyhow::Result<()> {
    let body = crate::request::DeleteBody { mode: None };
    let json = serde_json::to_string(&body)?;
    let decoded: crate::request::DeleteBody = serde_json::from_str(&json)?;
    assert_eq!(decoded, body);
    let from_missing: crate::request::DeleteBody = serde_json::from_str("{}")?;
    assert_eq!(from_missing.mode, None);
    Ok(())
}

#[test]
fn delete_body_rejects_invalid_mode_value() {
    let result = serde_json::from_str::<crate::request::DeleteBody>(r#"{"mode":"shred"}"#);
    assert!(result.is_err());
}

#[test]
fn user_delete_request_round_trips_with_mode_and_pow() -> anyhow::Result<()> {
    let request = crate::request::UserDeleteRequest {
        mode: Some(DeleteMode::Hard),
        pow: sample_pow()?,
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::UserDeleteRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    Ok(())
}

#[test]
fn create_token_request_round_trips_single_pow() -> anyhow::Result<()> {
    let request = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        pow: Some(sample_pow()?),
        old_email_pow: None,
        new_email_pow: None,
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::CreateTokenRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    Ok(())
}

#[test]
fn create_token_request_round_trips_dual_pow_pair() -> anyhow::Result<()> {
    let request = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        pow: None,
        old_email_pow: Some(sample_pow()?),
        new_email_pow: Some(sample_pow()?),
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::CreateTokenRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    Ok(())
}

#[test]
fn create_token_request_requires_a_purpose() {
    let result = serde_json::from_str::<crate::request::CreateTokenRequest>("{}");
    assert!(result.is_err(), "a missing purpose must be rejected");
}

#[test]
fn create_token_request_uses_purpose_as_the_wire_field_name() -> anyhow::Result<()> {
    let request = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        pow: Some(sample_pow()?),
        old_email_pow: None,
        new_email_pow: None,
    };
    let value = serde_json::to_value(&request)?;
    assert_eq!(value["purpose"], serde_json::json!("create_user"));
    assert!(value.get("pow").is_some());
    assert!(
        value.get("intent").is_none(),
        "the legacy intent field must not appear on the wire"
    );
    Ok(())
}

#[test]
fn create_token_request_dual_pair_is_consistent_only_when_both_or_neither() -> anyhow::Result<()> {
    let both = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        pow: None,
        old_email_pow: Some(sample_pow()?),
        new_email_pow: Some(sample_pow()?),
    };
    assert!(both.has_consistent_email_pow_pair());
    let neither = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        pow: Some(sample_pow()?),
        old_email_pow: None,
        new_email_pow: None,
    };
    assert!(neither.has_consistent_email_pow_pair());
    let only_old = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        pow: None,
        old_email_pow: Some(sample_pow()?),
        new_email_pow: None,
    };
    assert!(!only_old.has_consistent_email_pow_pair());
    let only_new = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        pow: None,
        old_email_pow: None,
        new_email_pow: Some(sample_pow()?),
    };
    assert!(!only_new.has_consistent_email_pow_pair());
    Ok(())
}

#[test]
fn single_pow_requests_round_trip() -> anyhow::Result<()> {
    let token_request = crate::request::TokenRequest { pow: sample_pow()? };
    let json = serde_json::to_string(&token_request)?;
    let decoded: crate::request::TokenRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, token_request);
    let logout_request = crate::request::LogoutRequest { pow: sample_pow()? };
    let json = serde_json::to_string(&logout_request)?;
    let decoded: crate::request::LogoutRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, logout_request);
    let name_set_request = crate::request::NameSetRequest { pow: sample_pow()? };
    let json = serde_json::to_string(&name_set_request)?;
    let decoded: crate::request::NameSetRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, name_set_request);
    let deregister_request = crate::request::DeregisterUserRequest { pow: sample_pow()? };
    let json = serde_json::to_string(&deregister_request)?;
    let decoded: crate::request::DeregisterUserRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, deregister_request);
    let deregister_confirm = crate::request::DeregisterUserConfirmRequest { pow: sample_pow()? };
    let json = serde_json::to_string(&deregister_confirm)?;
    let decoded: crate::request::DeregisterUserConfirmRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, deregister_confirm);
    Ok(())
}

#[test]
fn user_update_request_round_trips_all_branches() -> anyhow::Result<()> {
    let admin_rename = crate::request::UserUpdateRequest {
        pow: None,
        name: Some("new name".to_string()),
        old_email_token: None,
        new_email_token: None,
    };
    let json = serde_json::to_string(&admin_rename)?;
    let decoded: crate::request::UserUpdateRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, admin_rename);
    let email_confirm = crate::request::UserUpdateRequest {
        pow: Some(sample_pow()?),
        name: None,
        old_email_token: Some("token-a".to_string()),
        new_email_token: Some("token-b".to_string()),
    };
    let json = serde_json::to_string(&email_confirm)?;
    let decoded: crate::request::UserUpdateRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, email_confirm);
    let empty: crate::request::UserUpdateRequest = serde_json::from_str("{}")?;
    assert_eq!(empty, crate::request::UserUpdateRequest::default());
    Ok(())
}

#[test]
fn article_comment_and_role_requests_round_trip() -> anyhow::Result<()> {
    let update_article = crate::request::UpdateArticleRequest {
        title: "Title".to_string(),
        summary: "Summary".to_string(),
        tags: "a b".to_string(),
    };
    let json = serde_json::to_string(&update_article)?;
    let decoded: crate::request::UpdateArticleRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, update_article);
    let no_tags: crate::request::UpdateArticleRequest =
        serde_json::from_str(r#"{"title":"T","summary":"S"}"#)?;
    assert_eq!(no_tags.tags, "");
    let create_comment = crate::request::CreateCommentRequest {
        content: "A comment".to_string(),
    };
    let json = serde_json::to_string(&create_comment)?;
    let decoded: crate::request::CreateCommentRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, create_comment);
    let create_role = crate::request::CreateRoleRequest {
        name: "editor".to_string(),
    };
    let json = serde_json::to_string(&create_role)?;
    let decoded: crate::request::CreateRoleRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, create_role);
    Ok(())
}

#[test]
fn role_update_request_round_trips_change_lists() -> anyhow::Result<()> {
    let request = crate::request::RoleUpdateRequest {
        permissions: Some(crate::request::ChangeList {
            add: vec!["Article::Create".to_string()],
            remove: Vec::new(),
        }),
        users: None,
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::RoleUpdateRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    let empty: crate::request::RoleUpdateRequest = serde_json::from_str("{}")?;
    assert_eq!(empty, crate::request::RoleUpdateRequest::default());
    Ok(())
}

#[test]
fn article_search_params_round_trip_all_fields() -> anyhow::Result<()> {
    let params = crate::request::ArticleSearchParams {
        q: Some("rust".to_string()),
        ranges: Some("title,author".to_string()),
        from: Some("2023-11-14T22:13:20Z".to_string()),
        to: Some("2023-11-14T23:00:00Z".to_string()),
        limit: Some(8),
        page: Some(1),
    };
    let json = serde_json::to_string(&params)?;
    let decoded: crate::request::ArticleSearchParams = serde_json::from_str(&json)?;
    assert_eq!(decoded, params);
    Ok(())
}

#[test]
fn article_search_params_default_to_all_none() -> anyhow::Result<()> {
    let decoded: crate::request::ArticleSearchParams = serde_json::from_str("{}")?;
    assert_eq!(decoded, crate::request::ArticleSearchParams::default());
    Ok(())
}
