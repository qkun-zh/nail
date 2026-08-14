use crate::name::MAX_NAME_CHAR_COUNT;
use crate::name::NameError;
use crate::name::validate_name;

#[test]
fn rejects_blank_or_whitespace_only_input() {
    for raw in ["", "   ", "\t\n "] {
        let result = validate_name(raw);
        assert!(matches!(result, Err(NameError::Empty)));
    }
}

#[test]
fn rejects_names_with_forbidden_characters() {
    for raw in ["a b", "alice!", "a.b", "名", "a,b", "a@b"] {
        let result = validate_name(raw);
        assert!(matches!(result, Err(NameError::ContainsForbiddenChar(_))));
    }
}

#[test]
fn accepts_ascii_alphanumerics_dash_and_underscore() {
    for raw in ["alice", "Alice_01", "a-b_c", "0", "A"] {
        let result = validate_name(raw);
        assert_eq!(result, Ok(raw.to_string()));
    }
}

#[test]
fn accepts_every_allowed_single_character() {
    let mut allowed = Vec::new();
    for byte in b'a'..=b'z' {
        allowed.push(byte as char);
    }
    for byte in b'A'..=b'Z' {
        allowed.push(byte as char);
    }
    for byte in b'0'..=b'9' {
        allowed.push(byte as char);
    }
    allowed.push('-');
    allowed.push('_');
    for ch in allowed {
        let raw = ch.to_string();
        assert_eq!(validate_name(&raw), Ok(raw), "char {ch:?}");
    }
}

#[test]
fn rejects_name_longer_than_maximum_at_boundary() {
    let accepted = "a".repeat(MAX_NAME_CHAR_COUNT);
    assert_eq!(validate_name(&accepted), Ok(accepted.clone()));
    let rejected = "a".repeat(MAX_NAME_CHAR_COUNT + 1);
    assert!(matches!(validate_name(&rejected), Err(NameError::TooLong)));
}

#[test]
fn returns_trimmed_name_on_success() {
    let result = validate_name("  alice_01  ");
    assert_eq!(result, Ok("alice_01".to_string()));
}

#[test]
fn error_messages_are_english_and_descriptive() {
    assert_eq!(NameError::Empty.to_string(), "name cannot be empty");
    assert_eq!(
        NameError::TooLong.to_string(),
        "name too long (max 32 unicode chars)"
    );
    assert_eq!(
        NameError::ContainsForbiddenChar('!').to_string(),
        "name cannot contain '!'"
    );
}
