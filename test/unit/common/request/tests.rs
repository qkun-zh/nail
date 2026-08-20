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
fn create_token_request_round_trips_with_email() -> anyhow::Result<()> {
    let request = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        email: Some("alice@example.com".to_string()),
        old_email: None,
        new_email: None,
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::CreateTokenRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    Ok(())
}

#[test]
fn create_token_request_round_trips_email_change_pair() -> anyhow::Result<()> {
    let request = crate::request::CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("alice@example.com".to_string()),
        new_email: Some("alice-new@example.com".to_string()),
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
        email: Some("alice@example.com".to_string()),
        old_email: None,
        new_email: None,
    };
    let value = serde_json::to_value(&request)?;
    assert_eq!(value["purpose"], serde_json::json!("create_user"));
    assert_eq!(value["email"], serde_json::json!("alice@example.com"));
    assert!(
        value.get("intent").is_none(),
        "the legacy intent field must not appear on the wire"
    );
    assert!(
        value.get("pow").is_none(),
        "proof-of-work must travel in the x-pow header, not the body"
    );
    Ok(())
}

#[test]
fn token_request_round_trips_a_token() -> anyhow::Result<()> {
    let request = crate::request::TokenRequest {
        token: "token-value".to_string(),
    };
    let json = serde_json::to_string(&request)?;
    let decoded: crate::request::TokenRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, request);
    Ok(())
}

#[test]
fn user_delete_query_round_trips_mode_and_token() -> anyhow::Result<()> {
    let query = crate::request::UserDeleteQuery {
        mode: Some(DeleteMode::Transfer),
        token: Some("token-value".to_string()),
    };
    assert_eq!(
        serde_json::to_string(&query)?,
        r#"{"mode":"transfer","token":"token-value"}"#
    );
    let decoded: crate::request::UserDeleteQuery =
        serde_json::from_str(r#"{"mode":"transfer","token":"token-value"}"#)?;
    assert_eq!(decoded, query);
    let no_mode: crate::request::UserDeleteQuery =
        serde_json::from_str(r#"{"token":"token-value"}"#)?;
    assert_eq!(no_mode.mode, None);
    let no_token: crate::request::UserDeleteQuery = serde_json::from_str(r#"{"mode":"hard"}"#)?;
    assert_eq!(no_token.token, None);
    Ok(())
}

#[test]
fn user_update_request_round_trips_all_branches() -> anyhow::Result<()> {
    let admin_rename = crate::request::UserUpdateRequest {
        name: Some("new name".to_string()),
        old_email_token: None,
        new_email_token: None,
    };
    let json = serde_json::to_string(&admin_rename)?;
    let decoded: crate::request::UserUpdateRequest = serde_json::from_str(&json)?;
    assert_eq!(decoded, admin_rename);
    let email_confirm = crate::request::UserUpdateRequest {
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
