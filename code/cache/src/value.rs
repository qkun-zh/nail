use std::fmt;

use uuid::{Uuid, Version};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    InvalidHash,
    InvalidId,
}

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHash => formatter.write_str("invalid hash"),
            Self::InvalidId => formatter.write_str("invalid id"),
        }
    }
}

impl std::error::Error for CacheError {}

pub trait CacheValue: Clone + Send + Sync + 'static {
    fn reverse_key(&self) -> Option<&str> {
        None
    }

    fn validate(&self) -> Result<(), CacheError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash(String);

impl Hash {
    pub fn new(value: String) -> Result<Self, CacheError> {
        if value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(CacheError::InvalidHash)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CacheValue for Hash {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserId(String);

impl UserId {
    pub fn new(value: String) -> Result<Self, CacheError> {
        validate_uuid_v7(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CacheValue for UserId {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionId(String);

impl VersionId {
    pub fn new(value: String) -> Result<Self, CacheError> {
        validate_uuid_v7(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CacheValue for VersionId {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Challenge;

impl CacheValue for Challenge {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldAndNewEmailAddressAndTokenHashes {
    pub old_email_address_hash: Hash,
    pub new_email_address_hash: Hash,
    pub old_email_token_hash: Hash,
    pub new_email_token_hash: Hash,
}

impl CacheValue for OldAndNewEmailAddressAndTokenHashes {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserIdAndEmailAddressHash {
    pub user_id: UserId,
    pub email_address_hash: Hash,
}

impl CacheValue for UserIdAndEmailAddressHash {
    fn reverse_key(&self) -> Option<&str> {
        Some(self.user_id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionIdAndUserId {
    pub version_id: VersionId,
    pub user_id: UserId,
}

impl CacheValue for VersionIdAndUserId {}

fn validate_uuid_v7(value: &str) -> Result<(), CacheError> {
    let uuid = Uuid::parse_str(value).map_err(|_| CacheError::InvalidId)?;
    if uuid.get_version() == Some(Version::SortRand) {
        Ok(())
    } else {
        Err(CacheError::InvalidId)
    }
}
