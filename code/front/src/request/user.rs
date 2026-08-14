use nail_common::pow::Pow;
use nail_common::request::{DeleteMode, EmailReadRequest, UserDeleteRequest, UserUpdateRequest};
use nail_common::response::EmptyView;
use nail_common::response::email::{EmailSubjectView, EmailSubjectsView};
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::UserNameView;

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn update_self_name(user_id: &str, pow: Pow) -> RequestResult<UserNameView> {
    let path = url::build_path_with_query(&["user", user_id, "update"], &[]);
    let body = UserUpdateRequest {
        pow: Some(pow),
        ..UserUpdateRequest::default()
    };
    http::post_json(&path, &body, true).await
}

pub async fn confirm_email_change(
    user_id: &str,
    pow: Pow,
    old_token: &str,
    new_token: &str,
) -> RequestResult<SessionTokenView> {
    let path = url::build_path_with_query(&["user", user_id, "update"], &[]);
    let body = UserUpdateRequest {
        pow: Some(pow),
        old_email_token: Some(old_token.to_string()),
        new_email_token: Some(new_token.to_string()),
        ..UserUpdateRequest::default()
    };
    http::post_json(&path, &body, true).await
}

pub async fn send_change_email(old_pow: Pow, new_pow: Pow) -> RequestResult<EmailSubjectsView> {
    let path = url::build_path_with_query(&["email", "read"], &[("intent", "change_email")]);
    let body = EmailReadRequest {
        old_email_pow: Some(old_pow),
        new_email_pow: Some(new_pow),
        ..EmailReadRequest::default()
    };
    http::post_json(&path, &body, true).await
}

pub async fn send_deregister_email(pow: Pow) -> RequestResult<EmailSubjectView> {
    let path = url::build_path_with_query(&["email", "read"], &[("intent", "deregister")]);
    let body = EmailReadRequest {
        pow: Some(pow),
        ..EmailReadRequest::default()
    };
    http::post_json(&path, &body, true).await
}

pub async fn deregister_self(user_id: &str, pow: Pow) -> RequestResult<EmptyView> {
    let path = url::build_path_with_query(&["user", user_id, "delete"], &[]);
    let body = UserDeleteRequest {
        mode: Some(DeleteMode::Transfer),
        pow,
    };
    http::post_json(&path, &body, true).await
}
