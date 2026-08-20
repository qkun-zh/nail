use nail_common::pow::Pow;
use nail_common::request::{CreateTokenRequest, TokenPurpose, TokenRequest};
use nail_common::response::EmptyView;
use nail_common::response::email::EmailSubjectView;
use nail_common::response::session::{SessionTokenView, SessionView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn send_authenticate_email(pow: Pow) -> RequestResult<EmailSubjectView> {
    let body = create_user_token_request(pow);
    http::post_json("/tokens", &body, false).await
}

fn create_user_token_request(pow: Pow) -> CreateTokenRequest {
    CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    }
}

pub async fn redeem_token(pow: Pow) -> RequestResult<SessionTokenView> {
    http::post_json("/users", &TokenRequest { pow }, false).await
}

pub async fn read_session(id: bool, name: bool) -> RequestResult<SessionView> {
    let mut query = Vec::new();
    if id {
        query.push(("id", "true"));
    }
    if name {
        query.push(("name", "true"));
    }
    let path = url::build_path_with_query(&["user"], &query);
    http::get_json(&path, true).await
}

pub async fn delete_session(pow: Pow) -> RequestResult<EmptyView> {
    let pow_str = serde_json::to_string(&pow).unwrap_or_default();
    let path = url::build_path_with_query(&["session"], &[("pow", &pow_str)]);
    http::delete_json(&path, true).await
}

#[cfg(test)]
#[path = "../../../../test/unit/front/request/auth/tests.rs"]
mod tests;
