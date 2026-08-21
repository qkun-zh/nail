use crate::text::TextError;
use crate::text::validate_ascii_text;

#[test]
fn rejects_blank_or_whitespace_only_input() {
    for raw in ["", "   ", "\t\n "] {
        let result = validate_ascii_text(raw, 100, false);
        assert!(matches!(result, Err(TextError::Empty)));
    }
}

#[test]
fn rejects_non_ascii_characters() {
    for raw in ["café", "你好", "naïve"] {
        let result = validate_ascii_text(raw, 100, false);
        assert!(matches!(result, Err(TextError::ContainsForbiddenChar(_))));
    }
}

#[test]
fn rejects_control_characters() {
    for raw in ["a\u{0}b", "a\u{7f}b", "tab\there"] {
        let result = validate_ascii_text(raw, 100, false);
        assert!(matches!(result, Err(TextError::ContainsForbiddenChar(_))));
    }
}

#[test]
fn rejects_newline_unless_allowed() {
    let rejected = validate_ascii_text("line1\nline2", 100, false);
    assert!(matches!(
        rejected,
        Err(TextError::ContainsForbiddenChar('\n'))
    ));
    let accepted = validate_ascii_text("line1\nline2", 100, true);
    assert_eq!(accepted, Ok("line1\nline2".to_string()));
}

#[test]
fn rejects_oversized_input_at_boundary() {
    let accepted = validate_ascii_text("12345", 5, false);
    assert_eq!(accepted, Ok("12345".to_string()));
    let rejected = validate_ascii_text("123456", 5, false);
    assert!(matches!(rejected, Err(TextError::TooLong { max_chars: 5 })));
}

#[test]
fn measures_length_after_trimming() {
    let accepted = validate_ascii_text("  abc  ", 3, false);
    assert_eq!(accepted, Ok("abc".to_string()));
    let rejected = validate_ascii_text("  abcd  ", 3, false);
    assert!(matches!(rejected, Err(TextError::TooLong { max_chars: 3 })));
}

#[test]
fn returns_trimmed_text_on_success() {
    let result = validate_ascii_text("  Hello, World! 123  ", 100, false);
    assert_eq!(result, Ok("Hello, World! 123".to_string()));
}

#[test]
fn error_messages_are_english_and_descriptive() {
    assert_eq!(TextError::Empty.to_string(), "text cannot be empty");
    assert_eq!(
        TextError::TooLong { max_chars: 5 }.to_string(),
        "text too long (max 5 ascii chars)"
    );
    assert_eq!(
        TextError::ContainsForbiddenChar('é').to_string(),
        "text can only contain printable ASCII; forbidden: 'é'"
    );
}

#[test]
fn accepts_every_printable_ascii_character() {
    for byte in 0x21..=0x7e {
        let ch = char::from(byte);
        let result = validate_ascii_text(&ch.to_string(), 1, false);
        assert_eq!(result, Ok(ch.to_string()), "byte {byte:#04x}");
    }
}

#[test]
fn space_alone_trims_to_empty() {
    let result = validate_ascii_text(" ", 1, false);
    assert!(matches!(result, Err(TextError::Empty)));
}

#[test]
fn rejects_every_non_printable_ascii_byte() {
    for byte in (0x00..=0x1f).chain(0x7f..=0x7f) {
        let ch = char::from(byte);
        let result = validate_ascii_text(&ch.to_string(), 1, false);
        if ch.is_whitespace() {
            assert!(matches!(result, Err(TextError::Empty)), "byte {byte:#04x}");
        } else {
            assert!(
                matches!(
                    result,
                    Err(TextError::ContainsForbiddenChar(c)) if c == ch
                ),
                "byte {byte:#04x}"
            );
        }
    }
}
