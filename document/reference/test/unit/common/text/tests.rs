
use super::*;

#[test]
fn accepts_printable_ascii() {
    assert_eq!(
        validate_ascii_text("hello world 123", 100, false).unwrap(),
        "hello world 123"
    );
    assert_eq!(
        validate_ascii_text("line1\nline2", 100, true).unwrap(),
        "line1\nline2"
    );
}

#[test]
fn trims_whitespace() {
    assert_eq!(
        validate_ascii_text("  padded  ", 100, false).unwrap(),
        "padded"
    );
}

#[test]
fn rejects_empty_after_trim() {
    for raw in ["", "   ", "\n\t "] {
        assert_eq!(validate_ascii_text(raw, 100, true), Err(TextError::Empty));
    }
}

#[test]
fn rejects_non_ascii_before_length() {
    for raw in ["中文", "emoji 👍", "caf\u{e9}"] {
        assert!(matches!(
            validate_ascii_text(raw, 100, true),
            Err(TextError::ContainsForbiddenChar(ch)) if !ch.is_ascii()
        ));
    }
}

#[test]
fn rejects_control_characters() {
    for raw in ["a\u{0}b", "a\u{1}b", "a\u{7f}b", "a\u{1b}[31m"] {
        assert!(matches!(
            validate_ascii_text(raw, 100, true),
            Err(TextError::ContainsForbiddenChar(_))
        ));
    }
}

#[test]
fn newline_policy_is_per_field() {
    assert!(validate_ascii_text("a\nb", 100, true).is_ok());
    assert!(matches!(
        validate_ascii_text("a\nb", 100, false),
        Err(TextError::ContainsForbiddenChar('\n'))
    ));
    assert!(matches!(
        validate_ascii_text("a\r\nb", 100, true),
        Err(TextError::ContainsForbiddenChar('\r'))
    ));
}

#[test]
fn rejects_too_long_ascii_only() {
    let raw = "a".repeat(101);
    assert_eq!(
        validate_ascii_text(&raw, 100, false),
        Err(TextError::TooLong { max_chars: 100 })
    );
    assert!(validate_ascii_text(&"a".repeat(100), 100, false).is_ok());
}

#[test]
fn error_messages_are_escaped() {
    let msg = TextError::ContainsForbiddenChar('\u{1b}').to_string();
    assert!(msg.contains("\\u{1b}") || msg.contains("\\x1b"));
    assert!(!msg.contains('\u{1b}'));
}
