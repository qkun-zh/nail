use email_address::{EmailAddress, Options};
use nail_common::pow::Pow;
use nail_common::request::{CreateTokenRequest, TokenPurpose};
use nail_common::response::email::{EmailSubjectView, EmailSubjectsView};
use uuid::Uuid;

use crate::infrastructure::email::SendEmailError;
use crate::infrastructure::state::AppState;
use crate::logic::error::{LogicError, database_error};
use crate::logic::pow::verify_issued_pow;
use crate::logic::session::{create_session, normalize_token, read_session};
use crate::repository::cache::{
    CreateUserTokenEntry, DeleteUserTokenEntry, EmailUpdateTokenEntry, token_key,
};
use crate::repository::user::{
    UserWriteError, read_user, read_user_by_email_address_hash,
    update_user_email as write_user_email,
};

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
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

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum CreateTokenView {
    Subject(EmailSubjectView),
    Subjects(EmailSubjectsView),
}

pub async fn create_token(
    state: &AppState,
    request: CreateTokenRequest,
    session_token: Option<String>,
) -> Result<CreateTokenView, LogicError> {
    match request.purpose {
        TokenPurpose::CreateUser => {
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required"))?;
            let email_subject = send_create_user_email(state, &pow).await?;
            Ok(CreateTokenView::Subject(EmailSubjectView { email_subject }))
        }
        TokenPurpose::UpdateUserEmail => {
            let user_id = read_session_user(state, session_token)?;
            let old_email_pow = request
                .old_email_pow
                .ok_or_else(|| LogicError::bad_request("old_email_pow is required"))?;
            let new_email_pow = request
                .new_email_pow
                .ok_or_else(|| LogicError::bad_request("new_email_pow is required"))?;
            let (old_email_subject, new_email_subject) =
                send_update_user_email(state, &user_id, &old_email_pow, &new_email_pow).await?;
            Ok(CreateTokenView::Subjects(EmailSubjectsView {
                old_email_subject,
                new_email_subject,
            }))
        }
        TokenPurpose::DeleteUser => {
            let user_id = read_session_user(state, session_token)?;
            let pow = request
                .pow
                .ok_or_else(|| LogicError::bad_request("pow is required"))?;
            let email_subject = send_delete_user_email(state, &user_id, &pow).await?;
            Ok(CreateTokenView::Subject(EmailSubjectView { email_subject }))
        }
    }
}

fn read_session_user(
    state: &AppState,
    session_token: Option<String>,
) -> Result<String, LogicError> {
    let token =
        session_token.ok_or_else(|| LogicError::unauthorized("missing session-token header"))?;
    read_session(state, &token)
}

async fn send_create_user_email(state: &AppState, pow: &Pow) -> Result<String, LogicError> {
    let email = normalize_email(&pow.payload);
    if !validate_email(&email, &state.config.email.allowed_domains) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }
    verify_issued_pow(state, pow)?;

    let token = Uuid::now_v7().to_string();
    let email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &email, &email_subject, &token).await?;

    let email_address_hash = nail_common::hash::email(&email);
    let key = token_key(&token).map_err(|error| {
        LogicError::internal(format!("failed to hash create-user token: {error}"))
    })?;
    state.caches.create_user.insert(
        &key,
        CreateUserTokenEntry {
            email_address_hash: email_address_hash.clone(),
        },
    );
    Ok(email_subject)
}

pub async fn send_update_user_email(
    state: &AppState,
    user_id: &str,
    old_email_pow: &Pow,
    new_email_pow: &Pow,
) -> Result<(String, String), LogicError> {
    verify_issued_pow(state, old_email_pow)?;
    verify_issued_pow(state, new_email_pow)?;

    let old_email = normalize_email(&old_email_pow.payload);
    let new_email = normalize_email(&new_email_pow.payload);
    if old_email == new_email {
        return Err(LogicError::bad_request(
            "new email must be different from old email",
        ));
    }

    let user_entry = read_user(&state.graph, user_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    let old_email_hash = nail_common::hash::email(&old_email);
    if user_entry.email_address_hash != old_email_hash {
        return Err(LogicError::bad_request(
            "old email does not match your current email",
        ));
    }

    let allowed_domains = &state.config.email.allowed_domains;
    if !validate_email(&old_email, allowed_domains) || !validate_email(&new_email, allowed_domains)
    {
        return Err(LogicError::bad_request("email domain not allowed"));
    }

    let new_email_hash = nail_common::hash::email(&new_email);
    if let Some(existing_user_id) = read_user_by_email_address_hash(&state.graph, &new_email_hash)
        .await
        .map_err(database_error)?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();

    let old_email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &old_email, &old_email_subject, &old_token).await?;
    let new_email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &new_email, &new_email_subject, &new_token).await?;

    let token_hash_from_old_email = token_key(&old_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let token_hash_from_new_email = token_key(&new_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    state.caches.email_update.insert(
        user_id,
        EmailUpdateTokenEntry {
            old_email_hash: old_email_hash.clone(),
            new_email_hash: new_email_hash.clone(),
            token_hash_from_old_email,
            token_hash_from_new_email,
        },
    );
    Ok((old_email_subject, new_email_subject))
}

pub async fn update_user_email(
    state: &AppState,
    user_id: &str,
    pow: &Pow,
    raw_old_email_token: &str,
    raw_new_email_token: &str,
) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;

    let raw_old = raw_old_email_token.trim();
    let raw_new = raw_new_email_token.trim();
    let old_email_token = normalize_token(raw_old)
        .ok_or_else(|| LogicError::bad_request("invalid old email token"))?;
    let new_email_token = normalize_token(raw_new)
        .ok_or_else(|| LogicError::bad_request("invalid new email token"))?;

    let expected_canonical = format!("{old_email_token}\n{new_email_token}");
    let expected_raw = format!("{raw_old}\n{raw_new}");
    if pow.payload != expected_canonical && pow.payload != expected_raw {
        return Err(LogicError::bad_request("PoW payload does not match token"));
    }
    if old_email_token == new_email_token {
        return Err(LogicError::bad_request(
            "old token and new token must be different",
        ));
    }

    let entry = state
        .caches
        .email_update
        .read(user_id)
        .ok_or_else(|| LogicError::bad_request("invalid or expired email update request"))?;

    let old_token_hash = token_key(&old_email_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    let new_token_hash = token_key(&new_email_token)
        .map_err(|error| LogicError::internal(format!("failed to hash email token: {error}")))?;
    if entry.token_hash_from_old_email != old_token_hash
        || entry.token_hash_from_new_email != new_token_hash
    {
        return Err(LogicError::bad_request("token mismatch"));
    }

    let old_email_hash = entry.old_email_hash;
    let new_email_hash = entry.new_email_hash;
    if let Some(existing_user_id) = read_user_by_email_address_hash(&state.graph, &new_email_hash)
        .await
        .map_err(database_error)?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    write_user_email(&state.graph, user_id, &old_email_hash, &new_email_hash)
        .await
        .map_err(|error| match error {
            UserWriteError::AlreadyTaken => {
                LogicError::bad_request("new email is already used by another account")
            }
            UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
            UserWriteError::EmailMismatch => {
                LogicError::bad_request("email has already been changed")
            }
            UserWriteError::Db(error) => {
                LogicError::internal(format!("failed to update email: {error}"))
            }
        })?;

    state.caches.email_update.consume_if(user_id, |current| {
        current.token_hash_from_old_email == old_token_hash
            && current.token_hash_from_new_email == new_token_hash
    });
    state.caches.session.delete_by_reverse_key(user_id);
    state
        .caches
        .create_user
        .delete_by_reverse_key(&old_email_hash);
    state.caches.delete_user.delete_by_reverse_key(user_id);

    let new_session_token = create_session(state, user_id)?;
    Ok(new_session_token)
}

pub async fn send_delete_user_email(
    state: &AppState,
    user_id: &str,
    pow: &Pow,
) -> Result<String, LogicError> {
    verify_issued_pow(state, pow)?;

    let email = normalize_email(&pow.payload);
    let user_entry = read_user(&state.graph, user_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    if user_entry.email_address_hash != nail_common::hash::email(&email) {
        return Err(LogicError::bad_request("email does not match your account"));
    }

    let token = Uuid::now_v7().to_string();
    let email_subject = Uuid::now_v7().to_string();
    send_confirmation_email(state, &email, &email_subject, &token).await?;

    let key = token_key(&token)
        .map_err(|error| LogicError::internal(format!("failed to hash delete token: {error}")))?;
    state.caches.delete_user.insert(
        &key,
        DeleteUserTokenEntry {
            user_id: user_id.to_string(),
            email_address_hash: user_entry.email_address_hash,
        },
    );
    Ok(email_subject)
}

async fn send_confirmation_email(
    state: &AppState,
    email: &str,
    subject: &str,
    token: &str,
) -> Result<(), LogicError> {
    match state.email.send_email(email, subject, token).await {
        Ok(()) => Ok(()),
        Err(SendEmailError::RateLimited) => Err(LogicError::bad_request(
            "email already sent recently, check your inbox",
        )),
        Err(SendEmailError::Transport(error)) => {
            tracing::warn!(target: "email", error = %error, "failed to send email");
            Err(LogicError::internal("failed to send email"))
        }
    }
}
