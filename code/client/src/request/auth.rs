use common::request::{CreateTokenRequest, TokenPurpose, TokenRequest};
use common::response::EmptyView;
use common::response::email::EmailSubjectView;
use common::response::session::{SessionTokenView, SessionView};

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::{http, url};

pub async fn send_authenticate_email(email: String) -> RequestResult<EmailSubjectView> {
    let pow = prove_pow().await?;
    let body = create_user_token_request(email);
    http::post_json("/tokens", &body, false, Some(&pow)).await
}

fn create_user_token_request(email: String) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        email: Some(email),
        old_email: None,
        new_email: None,
    }
}

pub async fn redeem_token(token: String) -> RequestResult<SessionTokenView> {
    let pow = prove_pow().await?;
    http::post_json("/users", &TokenRequest { token }, false, Some(&pow)).await
}

pub async fn read_session(id: bool, name: bool) -> RequestResult<SessionView> {
    let pow = prove_pow().await?;
    let mut query = Vec::new();
    if id {
        query.push(("id", "true"));
    }
    if name {
        query.push(("name", "true"));
    }
    let path = url::build_path_with_query(&["user"], &query);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn delete_session() -> RequestResult<EmptyView> {
    let pow = prove_pow().await?;
    http::delete_json("/session", true, Some(&pow)).await
}

#[cfg(test)]
#[path = "../../../../test/unit/client/request/auth/tests.rs"]
mod tests;
