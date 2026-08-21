use common::request::{CreateTokenRequest, DeleteMode, TokenPurpose, UserUpdateRequest};
use common::response::EmptyView;
use common::response::ListPage;
use common::response::email::{EmailSubjectView, EmailSubjectsView};
use common::response::session::SessionTokenView;
use common::response::user::{UserListItem, UserNameView, UserView};

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_user(user_id: &str) -> RequestResult<UserView> {
    let pow = prove_pow().await?;
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(
        &["users", &user_id],
        &[("name", "true"), ("email_hash", "true"), ("roles", "true")],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_users(page: u64, limit: u64) -> RequestResult<ListPage<UserListItem>> {
    let pow = prove_pow().await?;
    let path = url::build_path_with_query(
        &["users"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn update_self_name(user_id: &str, name: String) -> RequestResult<UserNameView> {
    let pow = prove_pow().await?;
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id], &[]);
    let body = UserUpdateRequest {
        name: Some(name),
        old_email_token: None,
        new_email_token: None,
    };
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn confirm_email_change(
    user_id: &str,
    old_token: &str,
    new_token: &str,
) -> RequestResult<SessionTokenView> {
    let pow = prove_pow().await?;
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id], &[]);
    let body = UserUpdateRequest {
        name: None,
        old_email_token: Some(old_token.to_string()),
        new_email_token: Some(new_token.to_string()),
    };
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn send_change_email(
    old_email: String,
    new_email: String,
) -> RequestResult<EmailSubjectsView> {
    let pow = prove_pow().await?;
    let body = update_user_email_token_request(old_email, new_email);
    http::post_json("/tokens", &body, true, Some(&pow)).await
}

fn update_user_email_token_request(old_email: String, new_email: String) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some(old_email),
        new_email: Some(new_email),
    }
}

pub async fn send_deregister_email(email: String) -> RequestResult<EmailSubjectView> {
    let pow = prove_pow().await?;
    let body = delete_user_token_request(email);
    http::post_json("/tokens", &body, true, Some(&pow)).await
}

fn delete_user_token_request(email: String) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::DeleteUser,
        email: Some(email),
        old_email: None,
        new_email: None,
    }
}

pub async fn deregister_self(
    user_id: &str,
    delete_token: String,
    mode: DeleteMode,
) -> RequestResult<EmptyView> {
    let pow = prove_pow().await?;
    let user_id = validate_id(user_id, "user_id")?;
    let mode_str = serde_json::to_string(&mode).unwrap_or_default();
    let path = url::build_path_with_query(
        &["users", &user_id],
        &[("mode", &mode_str), ("token", &delete_token)],
    );
    http::delete_json(&path, true, Some(&pow)).await
}

pub async fn undelete_soft_user(user_id: &str) -> RequestResult<EmptyView> {
    let pow = prove_pow().await?;
    let user_id = validate_id(user_id, "user_id")?;
    let path = url::build_path_with_query(&["users", &user_id, "restore"], &[]);
    http::post_json(&path, &(), true, Some(&pow)).await
}

#[cfg(test)]
#[path = "../../../../test/unit/client/request/user/tests.rs"]
mod tests;
