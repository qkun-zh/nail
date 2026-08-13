
use crate::repo::types::{
    AuthenticateTokenEntry, DeregisterTokenEntry, DownloadTokenEntry, EmailUpdateTokenEntry,
    SessionTokenEntry, UserEntry, VersionEntry, alias_of,
};

#[test]
fn user_entry_carries_email_hash_and_name_fields() {
    let entry = UserEntry {
        email_address_hash: "hash-1".to_string(),
        name: "Alice-01".to_string(),
    };
    assert_eq!(entry.email_address_hash, "hash-1");
    assert_eq!(entry.name, "Alice-01");
}

#[test]
fn version_entry_carries_version_number_content_hash_and_note_fields() {
    let entry = VersionEntry {
        version_number: "1.2.3".to_string(),
        content_hash: "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string(),
        note: "n".to_string(),
    };
    assert_eq!(entry.version_number, "1.2.3");
    assert_eq!(entry.content_hash, "a1b2c3d4e5f60718293a4b5c6d7e8f90");
    assert_eq!(entry.note, "n");
}

#[test]
fn authenticate_token_entry_carries_email_and_subject_fields() {
    let entry = AuthenticateTokenEntry {
        email_address_hash: "h".to_string(),
        email_subject: "subject".to_string(),
    };
    assert_eq!(entry.email_address_hash, "h");
    assert_eq!(entry.email_subject, "subject");
}

#[test]
fn session_token_entry_carries_user_identifier() {
    let entry = SessionTokenEntry {
        user_id: "u1".to_string(),
    };
    assert_eq!(entry.user_id, "u1");
}

#[test]
fn email_update_token_entry_carries_pair_and_token_hash_fields() {
    let entry = EmailUpdateTokenEntry {
        old_email_address_hash: "old-hash".to_string(),
        new_email_address_hash: "new-hash".to_string(),
        token_from_old_email_hash: "token-old-hash".to_string(),
        token_from_new_email_hash: "token-new-hash".to_string(),
    };
    assert_eq!(entry.old_email_address_hash, "old-hash");
    assert_eq!(entry.new_email_address_hash, "new-hash");
    assert_eq!(entry.token_from_old_email_hash, "token-old-hash");
    assert_eq!(entry.token_from_new_email_hash, "token-new-hash");
}

#[test]
fn deregister_token_entry_carries_user_and_email_hash_fields() {
    let entry = DeregisterTokenEntry {
        user_id: "u1".to_string(),
        email_address_hash: "h".to_string(),
    };
    assert_eq!(entry.user_id, "u1");
    assert_eq!(entry.email_address_hash, "h");
}

#[test]
fn download_token_entry_carries_ownership_fields() {
    let entry = DownloadTokenEntry {
        version_id: "v1".to_string(),
        user_id: "u1".to_string(),
    };
    assert_eq!(entry.version_id, "v1");
    assert_eq!(entry.user_id, "u1");
}

#[test]
fn entity_type_constants_are_complete() {
    assert_eq!(crate::repo::types::ENTITY_TYPE_USER, "user");
    assert_eq!(crate::repo::types::ENTITY_TYPE_ARTICLE, "article");
    assert_eq!(crate::repo::types::ENTITY_TYPE_VERSION, "version");
    assert_eq!(crate::repo::types::ENTITY_TYPE_COMMENT, "comment");
    assert_eq!(crate::repo::types::ENTITY_TYPE_TAG, "tag");
    assert_eq!(crate::repo::types::ENTITY_TYPE_ROLE, "role");
    assert_eq!(crate::repo::types::ENTITY_TYPE_PERMISSION, "permission");
}

#[test]
fn edge_type_constants_are_complete() {
    assert_eq!(crate::repo::types::EDGE_USER_TO_ARTICLE, "user_to_article");
    assert_eq!(
        crate::repo::types::EDGE_ARTICLE_TO_VERSION,
        "article_to_version"
    );
    assert_eq!(crate::repo::types::EDGE_USER_TO_COMMENT, "user_to_comment");
    assert_eq!(
        crate::repo::types::EDGE_COMMENT_TO_VERSION,
        "comment_to_version"
    );
    assert_eq!(
        crate::repo::types::EDGE_COMMENT_TO_COMMENT,
        "comment_to_comment"
    );
    assert_eq!(crate::repo::types::EDGE_ARTICLE_TO_TAG, "article_to_tag");
    assert_eq!(crate::repo::types::EDGE_USER_HOLD_ROLE, "user_hold_role");
    assert_eq!(
        crate::repo::types::EDGE_ROLE_GRANT_PERMISSION,
        "role_grant_permission"
    );
    assert_eq!(crate::repo::types::EDGE_ROLE_APPLY_TAG, "role_apply_tag");
}

#[test]
fn key_constants_are_complete() {
    assert_eq!(crate::repo::types::KEY_TYPE, "type");
    assert_eq!(crate::repo::types::KEY_ID, "id");
    assert_eq!(
        crate::repo::types::KEY_EMAIL_ADDRESS_HASH,
        "email_address_hash"
    );
    assert_eq!(crate::repo::types::KEY_USER_NAME, "name");
    assert_eq!(crate::repo::types::KEY_TITLE, "title");
    assert_eq!(crate::repo::types::KEY_SUMMARY, "summary");
    assert_eq!(crate::repo::types::KEY_CONTENT_HASH, "content_hash");
    assert_eq!(crate::repo::types::KEY_TAG_NAME, "tag_name");
    assert_eq!(crate::repo::types::KEY_ROLE_NAME, "role_name");
    assert_eq!(crate::repo::types::KEY_PERMISSION_NAME, "permission_name");
    assert_eq!(
        crate::repo::types::KEY_LATEST_VERSION_ID,
        "latest_version_id"
    );
}

#[test]
fn visibility_constants_are_complete() {
    assert_eq!(crate::repo::types::VISIBILITY_PUBLIC, "public");
    assert_eq!(crate::repo::types::VISIBILITY_PRIVATE, "private");
}

#[test]
fn alias_of_joins_kind_and_business_identifier() {
    assert_eq!(alias_of("user", "u-1"), "user:u-1");
    assert_eq!(
        alias_of("permission", "Article::Read"),
        "permission:Article::Read"
    );
}
