use nail_common::pow::Pow;
use nail_common::request::{
    CreateTokenRequest, DeleteMode, TokenPurpose, UserDeleteRequest, UserUpdateRequest,
};
use nail_common::response::EmptyView;
use nail_common::response::email::{EmailSubjectView, EmailSubjectsView};
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::{UserNameView, UserView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_user(user_id: &str) -> RequestResult<UserView> {
    let path = url::build_path_with_query(
        &["user", user_id, "read"],
        &[("name", "true"), ("email_hash", "true"), ("roles", "true")],
    );
    http::get_json(&path, true).await
}

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
    let body = update_user_email_token_request(old_pow, new_pow);
    http::post_json("/token/create", &body, true).await
}

fn update_user_email_token_request(old_pow: Pow, new_pow: Pow) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        pow: None,
        old_email_pow: Some(old_pow),
        new_email_pow: Some(new_pow),
    }
}

pub async fn send_deregister_email(pow: Pow) -> RequestResult<EmailSubjectView> {
    let body = delete_user_token_request(pow);
    http::post_json("/token/create", &body, true).await
}

fn delete_user_token_request(pow: Pow) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::DeleteUser,
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    }
}

pub async fn deregister_self(
    user_id: &str,
    pow: Pow,
    mode: DeleteMode,
) -> RequestResult<EmptyView> {
    let path = url::build_path_with_query(&["user", user_id, "delete"], &[]);
    let body = UserDeleteRequest {
        mode: Some(mode),
        pow,
    };
    http::post_json(&path, &body, true).await
}

pub async fn undelete_soft_user(user_id: &str) -> RequestResult<EmptyView> {
    let path = url::build_path_with_query(&["user", user_id, "undelete-soft"], &[]);
    http::post_json(&path, &(), true).await
}

#[cfg(test)]
#[path = "../../../../test/unit/front/request/user/tests.rs"]
mod tests;
