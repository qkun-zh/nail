use crate::validate::AlphanumericDashUnderscore;
use crate::validate::CharPolicy;
use crate::validate::PrintableAscii;
use crate::validate::ValidationError;
use crate::validate::validate_with_policy;

#[derive(Debug, PartialEq, Eq)]
enum TestError {
    Empty,
    TooLong(usize),
    Forbidden(char),
}

impl ValidationError for TestError {
    fn empty() -> Self {
        TestError::Empty
    }
    fn too_long(max_chars: usize) -> Self {
        TestError::TooLong(max_chars)
    }
    fn forbidden(ch: char) -> Self {
        TestError::Forbidden(ch)
    }
}

#[test]
fn trims_and_returns_valid_input() {
    let result =
        validate_with_policy::<TestError, _>("  alice_01  ", 8, &AlphanumericDashUnderscore);
    assert_eq!(result, Ok("alice_01".to_string()));
}

#[test]
fn rejects_blank_or_whitespace_only_input() {
    for raw in ["", "   ", "\t\n "] {
        let result = validate_with_policy::<TestError, _>(raw, 8, &AlphanumericDashUnderscore);
        assert!(matches!(result, Err(TestError::Empty)), "raw: {raw:?}");
    }
}

#[test]
fn rejects_characters_not_allowed_by_the_policy() {
    for raw in ["a b", "alice!", "a.b", "名", "a,b"] {
        let result = validate_with_policy::<TestError, _>(raw, 8, &AlphanumericDashUnderscore);
        assert!(
            matches!(result, Err(TestError::Forbidden(_))),
            "raw: {raw:?}"
        );
    }
}

#[test]
fn alphanumeric_dash_underscore_policy_accepts_each_allowed_char() {
    for ch in (b'a'..=b'z')
        .chain(b'A'..=b'Z')
        .chain(b'0'..=b'9')
        .map(|byte| byte as char)
        .chain(['-', '_'])
    {
        assert!(
            AlphanumericDashUnderscore.allows(ch),
            "char {ch:?} should be allowed"
        );
    }
    for ch in [' ', '!', '.', '名', '\n'] {
        assert!(
            !AlphanumericDashUnderscore.allows(ch),
            "char {ch:?} should be rejected"
        );
    }
}

#[test]
fn enforces_the_length_cap_after_trimming() {
    let accepted = validate_with_policy::<TestError, _>("12345", 5, &AlphanumericDashUnderscore);
    assert_eq!(accepted, Ok("12345".to_string()));
    let rejected = validate_with_policy::<TestError, _>("123456", 5, &AlphanumericDashUnderscore);
    assert!(matches!(rejected, Err(TestError::TooLong(5))));
    let trimmed = validate_with_policy::<TestError, _>("  abcd  ", 4, &AlphanumericDashUnderscore);
    assert_eq!(trimmed, Ok("abcd".to_string()));
}

#[test]
fn printable_ascii_policy_rejects_newline_unless_allowed() {
    let strict = PrintableAscii {
        allow_newline: false,
    };
    let lax = PrintableAscii {
        allow_newline: true,
    };
    assert!(!strict.allows('\n'));
    assert!(lax.allows('\n'));
    assert!(!strict.allows('\u{0}'));
    assert!(!strict.allows('\u{7f}'));
    assert!(!strict.allows('é'));
    assert!(strict.allows('A'));
    assert!(strict.allows('!'));
}

#[test]
fn printable_ascii_policy_accepts_every_printable_ascii_byte() {
    let strict = PrintableAscii {
        allow_newline: false,
    };
    for byte in 0x21..=0x7e {
        let ch = char::from(byte);
        assert!(strict.allows(ch), "byte {byte:#04x}");
    }
}
