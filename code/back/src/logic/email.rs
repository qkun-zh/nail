use email_address::{EmailAddress, Options};
use nail_common::pow::Pow;
use nail_common::request::EmailReadIntent;
use uuid::Uuid;

use crate::infrastructure::email::SendEmailError;
use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::logic::pow::verify_issued_pow;
use crate::repository::cache::{AuthenticateTokenEntry, token_key};

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub fn parse_intent(value: &str) -> Option<EmailReadIntent> {
    match value {
        "authenticate" => Some(EmailReadIntent::Authenticate),
        "change_email" => Some(EmailReadIntent::ChangeEmail),
        "deregister" => Some(EmailReadIntent::Deregister),
        _ => None,
    }
}

pub fn validate_email(email: &str, allowed_domains: &[String]) -> bool {
    if email.len() > 254 {
        return false;
    }
    let Ok(parsed) = EmailAddress::parse_with_options(email, Options::default()) else {
        return false;
    };
    let domain = parsed.domain().to_lowercase();
    allowed_domains.iter().any(|allowed| allowed == &domain)
}

pub async fn handle_email_read(
    state: &AppState,
    intent: EmailReadIntent,
    request: nail_common::request::EmailReadRequest,
) -> Result<serde_json::Value, LogicError> {
    match intent {
        EmailReadIntent::Authenticate => {
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required"))?;
            let email_subject = handle_email_auth_request(state, &pow).await?;
            Ok(serde_json::json!({ "email_subject": email_subject }))
        }
        EmailReadIntent::ChangeEmail | EmailReadIntent::Deregister => {
            Err(LogicError::bad_request("email intent is not supported yet"))
        }
    }
}

async fn handle_email_auth_request(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    let email = normalize_email(&pow.payload);
    if !validate_email(&email, &state.config.email.allowed_domains) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }
    verify_issued_pow(state, pow)?;

    let token = Uuid::now_v7().to_string();
    let email_subject = Uuid::now_v7().to_string();

    match state
        .email
        .send_email(&email, &email_subject, &token)
        .await
    {
        Ok(()) => {}
        Err(SendEmailError::RateLimited) => {
            return Err(LogicError::bad_request(
                "email already sent recently, check your inbox",
            ));
        }
        Err(SendEmailError::Transport(error)) => {
            tracing::warn!(target: "email", error = %error, "failed to send authenticate email");
            return Err(LogicError::internal("failed to send authenticate email"));
        }
    }

    let email_address_hash = nail_common::hash::email(&email);
    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash auth token: {error}")))?;
    state.caches.authenticate.insert(
        &key,
        AuthenticateTokenEntry {
            email_address_hash: email_address_hash.clone(),
            email_subject: email_subject.clone(),
        },
    );
    tracing::info!(email_hash = %email_address_hash, "auth email sent");
    Ok(email_subject)
}
