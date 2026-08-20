use nail_common::pow::Pow;
use nail_common::request::{CreateTokenRequest, DeleteMode, TokenPurpose, UserUpdateRequest};
use nail_common::response::EmptyView;
use nail_common::response::ListPage;
use nail_common::response::email::{EmailSubjectView, EmailSubjectsView};
use nail_common::response::session::SessionTokenView;
use nail_common::response::user::{UserListItem, UserNameView, UserView};

use crate::request::error::RequestResult;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_user(user_id: &str) -> RequestResult<UserView> {
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(
        &["users", &user_id],
        &[("name", "true"), ("email_hash", "true"), ("roles", "true")],
    );
    http::get_json(&path, true).await
}

pub async fn read_users(page: u64, limit: u64) -> RequestResult<ListPage<UserListItem>> {
    let path = url::build_path_with_query(
        &["users"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn update_self_name(user_id: &str, pow: Pow) -> RequestResult<UserNameView> {
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id], &[]);
    let body = UserUpdateRequest {
        pow: Some(pow),
        ..UserUpdateRequest::default()
    };
    http::patch_json(&path, &body, true).await
}

pub async fn confirm_email_change(
    user_id: &str,
    pow: Pow,
    old_token: &str,
    new_token: &str,
) -> RequestResult<SessionTokenView> {
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id], &[]);
    let body = UserUpdateRequest {
        pow: Some(pow),
        old_email_token: Some(old_token.to_string()),
        new_email_token: Some(new_token.to_string()),
        ..UserUpdateRequest::default()
    };
    http::patch_json(&path, &body, true).await
}

pub async fn send_change_email(old_pow: Pow, new_pow: Pow) -> RequestResult<EmailSubjectsView> {
    let body = update_user_email_token_request(old_pow, new_pow);
    http::post_json("/tokens", &body, true).await
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
    http::post_json("/tokens", &body, true).await
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
    let user_id = validate_id(user_id, "user_id")?;
    let mode_str = serde_json::to_string(&mode).unwrap_or_default();
    let pow_str = serde_json::to_string(&pow).unwrap_or_default();
    let path = url::build_path_with_query(
        &["users", &user_id],
        &[("mode", &mode_str), ("pow", &pow_str)],
    );
    http::delete_json(&path, true).await
}

pub async fn undelete_soft_user(user_id: &str) -> RequestResult<EmptyView> {
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id, "restore"], &[]);
    http::post_json(&path, &(), true).await
}

#[cfg(test)]
#[path = "../../../../test/unit/front/request/user/tests.rs"]
mod tests;
