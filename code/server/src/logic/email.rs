use cache::{Hash, OldAndNewEmailAddressAndTokenHashes, UserId, UserIdAndEmailAddressHash};
use common::request::{CreateTokenRequest, TokenPurpose};
use common::response::email::{EmailSubjectView, EmailSubjectsView};
use email_address::{EmailAddress, Options};
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::logic::session::{create_session, hash_canonical_token, normalize_token, read_session};
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
            let email = request
                .email
                .ok_or_else(|| LogicError::bad_request("email is required"))?;
            let email_subject = send_create_user_email(state, &email).await?;
            Ok(CreateTokenView::Subject(EmailSubjectView { email_subject }))
        }
        TokenPurpose::UpdateUserEmail => {
            let user_id = read_session_user(state, session_token)?;
            let old_email = request
                .old_email
                .ok_or_else(|| LogicError::bad_request("old_email is required"))?;
            let new_email = request
                .new_email
                .ok_or_else(|| LogicError::bad_request("new_email is required"))?;
            let (old_email_subject, new_email_subject) =
                send_update_user_email(state, &user_id, &old_email, &new_email).await?;
            Ok(CreateTokenView::Subjects(EmailSubjectsView {
                old_email_subject,
                new_email_subject,
            }))
        }
        TokenPurpose::DeleteUser => {
            let user_id = read_session_user(state, session_token)?;
            let email = request
                .email
                .ok_or_else(|| LogicError::bad_request("email is required"))?;
            let email_subject = send_delete_user_email(state, &user_id, &email).await?;
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

async fn send_create_user_email(state: &AppState, raw_email: &str) -> Result<String, LogicError> {
    let email = normalize_email(raw_email);
    if !validate_email(&email, state.configurator.email_allowed_domains()) {
        return Err(LogicError::bad_request("email domain not allowed"));
    }

    let token = Uuid::now_v7().to_string();
    let email_id = send_confirmation_email(state, &email, &token).await?;

    let email_address_hash = common::hash::hash(email.as_bytes())
        .map_err(|error| LogicError::internal(format!("failed to hash email: {error}")))?;
    let key = hash_canonical_token(&token)?;
    state.cache.user_creation.insert(
        &key,
        Hash::new(email_address_hash.clone())
            .map_err(|error| LogicError::internal(format!("invalid email hash: {error}")))?,
    );
    Ok(email_id)
}

pub async fn send_update_user_email(
    state: &AppState,
    user_id: &str,
    raw_old_email: &str,
    raw_new_email: &str,
) -> Result<(String, String), LogicError> {
    let old_email = normalize_email(raw_old_email);
    let new_email = normalize_email(raw_new_email);
    if old_email == new_email {
        return Err(LogicError::bad_request(
            "new email must be different from old email",
        ));
    }

    let user_entry = read_user(&state.database, user_id)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    let old_email_hash = common::hash::hash(old_email.as_bytes())
        .map_err(|error| LogicError::internal(format!("failed to hash email: {error}")))?;
    if user_entry.email_address_hash != old_email_hash {
        return Err(LogicError::bad_request(
            "old email does not match your current email",
        ));
    }

    let allowed_domains = state.configurator.email_allowed_domains();
    if !validate_email(&old_email, allowed_domains) || !validate_email(&new_email, allowed_domains)
    {
        return Err(LogicError::bad_request("email domain not allowed"));
    }

    let new_email_hash = common::hash::hash(new_email.as_bytes())
        .map_err(|error| LogicError::internal(format!("failed to hash email: {error}")))?;
    if let Some(existing_user_id) =
        read_user_by_email_address_hash(&state.database, &new_email_hash)?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();

    let old_email_id = send_confirmation_email(state, &old_email, &old_token).await?;
    let new_email_id = send_confirmation_email(state, &new_email, &new_token).await?;

    let token_hash_from_old_email = hash_canonical_token(&old_token)?;
    let token_hash_from_new_email = hash_canonical_token(&new_token)?;
    state.cache.email_update.insert(
        user_id,
        OldAndNewEmailAddressAndTokenHashes {
            old_email_address_hash: Hash::new(old_email_hash.clone())
                .map_err(|error| LogicError::internal(format!("invalid email hash: {error}")))?,
            new_email_address_hash: Hash::new(new_email_hash.clone())
                .map_err(|error| LogicError::internal(format!("invalid email hash: {error}")))?,
            old_email_token_hash: Hash::new(token_hash_from_old_email).map_err(|error| {
                LogicError::internal(format!("invalid email token hash: {error}"))
            })?,
            new_email_token_hash: Hash::new(token_hash_from_new_email).map_err(|error| {
                LogicError::internal(format!("invalid email token hash: {error}"))
            })?,
        },
    );
    Ok((old_email_id, new_email_id))
}

pub fn update_user_email(
    state: &AppState,
    user_id: &str,
    raw_old_email_token: &str,
    raw_new_email_token: &str,
) -> Result<String, LogicError> {
    let raw_old = raw_old_email_token.trim();
    let raw_new = raw_new_email_token.trim();
    let old_email_token = normalize_token(raw_old)
        .ok_or_else(|| LogicError::bad_request("invalid old email token"))?;
    let new_email_token = normalize_token(raw_new)
        .ok_or_else(|| LogicError::bad_request("invalid new email token"))?;

    if old_email_token == new_email_token {
        return Err(LogicError::bad_request(
            "old token and new token must be different",
        ));
    }

    let entry = state
        .cache
        .email_update
        .read(user_id)
        .ok_or_else(|| LogicError::bad_request("invalid or expired email update request"))?;

    let old_token_hash = hash_canonical_token(&old_email_token)?;
    let new_token_hash = hash_canonical_token(&new_email_token)?;
    if entry.old_email_token_hash.as_str() != old_token_hash
        || entry.new_email_token_hash.as_str() != new_token_hash
    {
        return Err(LogicError::bad_request("token mismatch"));
    }

    let old_email_hash = entry.old_email_address_hash.as_str();
    let new_email_hash = entry.new_email_address_hash.as_str();
    if let Some(existing_user_id) =
        read_user_by_email_address_hash(&state.database, new_email_hash)?
        && existing_user_id != user_id
    {
        return Err(LogicError::bad_request(
            "new email is already used by another account",
        ));
    }

    write_user_email(&state.database, user_id, old_email_hash, new_email_hash).map_err(
        |error| match error {
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
        },
    )?;

    let _ = state.cache.email_update.delete_if(user_id, |current| {
        current.old_email_token_hash.as_str() == old_token_hash
            && current.new_email_token_hash.as_str() == new_token_hash
    });
    let _ = state.cache.session.delete_by_reverse_key(user_id);
    let _ = state
        .cache
        .user_creation
        .delete_by_reverse_key(old_email_hash);
    let _ = state.cache.user_deletion.delete_by_reverse_key(user_id);

    let new_session_token = create_session(state, user_id)?;
    Ok(new_session_token)
}

pub async fn send_delete_user_email(
    state: &AppState,
    user_id: &str,
    raw_email: &str,
) -> Result<String, LogicError> {
    let email = normalize_email(raw_email);
    let user_entry = read_user(&state.database, user_id)?
        .ok_or_else(|| LogicError::unauthorized("user not found"))?;
    let email_hash = common::hash::hash(email.as_bytes())
        .map_err(|error| LogicError::internal(format!("failed to hash email: {error}")))?;
    if user_entry.email_address_hash != email_hash {
        return Err(LogicError::bad_request("email does not match your account"));
    }

    let token = Uuid::now_v7().to_string();
    let email_id = send_confirmation_email(state, &email, &token).await?;

    let key = hash_canonical_token(&token)?;
    state.cache.user_deletion.insert(
        &key,
        UserIdAndEmailAddressHash {
            user_id: UserId::new(user_id.to_string())
                .map_err(|error| LogicError::internal(format!("invalid user id: {error}")))?,
            email_address_hash: Hash::new(user_entry.email_address_hash)
                .map_err(|error| LogicError::internal(format!("invalid email hash: {error}")))?,
        },
    );
    Ok(email_id)
}

async fn send_confirmation_email(
    state: &AppState,
    email: &str,
    token: &str,
) -> Result<String, LogicError> {
    match state.emailer.send(email, token).await {
        Ok(email_id) => Ok(email_id),
        Err(emailer::SendEmailError::RateLimited) => Err(LogicError::bad_request(
            "email already sent recently, check your inbox",
        )),
        Err(emailer::SendEmailError::Validation(error)) => Err(LogicError::bad_request(&error)),
        Err(emailer::SendEmailError::Transport(error)) => {
            tracing::warn!(target: "email", error = %error, "failed to send email");
            Err(LogicError::internal("failed to send email"))
        }
    }
}
