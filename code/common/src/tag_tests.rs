use crate::response::NamedRef;
use crate::tag::MAX_TAG_NAME_CHAR_COUNT;
use crate::tag::TagNameError;
use crate::tag::TagNamesError;
use crate::tag::parse_tags;
use crate::tag::validate_tag_name;

#[test]
fn accepts_plain_tag_name() {
    let result = validate_tag_name("rust");
    assert_eq!(result, Ok("rust".to_string()));
    assert_eq!(validate_tag_name("web-api"), Ok("web-api".to_string()));
    assert_eq!(validate_tag_name("my_tag"), Ok("my_tag".to_string()));
}

#[test]
fn trims_whitespace_around_tag_name() {
    assert_eq!(validate_tag_name("  rust  "), Ok("rust".to_string()));
}

#[test]
fn rejects_empty_tag_name() {
    for raw in ["", "   ", "	"] {
        let result = validate_tag_name(raw);
        assert!(matches!(result, Err(TagNameError::Empty)), "raw: {raw:?}");
    }
}

#[test]
fn rejects_tag_name_with_forbidden_characters() {
    for raw in ["a!b", "a b", "a$c", "名", "a#b"] {
        let result = validate_tag_name(raw);
        assert!(
            matches!(result, Err(TagNameError::ContainsForbiddenChar(_))),
            "raw: {raw:?}"
        );
    }
}

#[test]
fn rejects_tag_name_longer_than_maximum_at_boundary() {
    let accepted = "a".repeat(MAX_TAG_NAME_CHAR_COUNT);
    assert_eq!(validate_tag_name(&accepted), Ok(accepted.clone()));
    let rejected = "a".repeat(MAX_TAG_NAME_CHAR_COUNT + 1);
    assert!(matches!(
        validate_tag_name(&rejected),
        Err(TagNameError::TooLong)
    ));
}

#[test]
fn parses_empty_input_as_no_tags() {
    let result = parse_tags("", 8);
    assert_eq!(result, Ok(Vec::new()));
}

#[test]
fn parses_whitespace_separated_tags() {
    let result = parse_tags("rust  web\napi", 8);
    assert_eq!(
        result,
        Ok(vec![
            "rust".to_string(),
            "web".to_string(),
            "api".to_string()
        ])
    );
}

#[test]
fn deduplicates_repeated_tag_names() {
    let result = parse_tags("a a b a", 8);
    assert_eq!(result, Ok(vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn rejects_tag_with_forbidden_character() {
    let result = parse_tags("a!", 8);
    assert!(matches!(
        result,
        Err(TagNamesError::Name(TagNameError::ContainsForbiddenChar(
            '!'
        )))
    ));
}

#[test]
fn rejects_more_tags_than_max_count() {
    let raw = ["1", "2", "3", "4", "5", "6", "7", "8", "9"].join(" ");
    let result = parse_tags(&raw, 8);
    assert!(matches!(
        result,
        Err(TagNamesError::TooManyTags { max_count: 8 })
    ));
}

#[test]
fn deduplication_does_not_consume_the_count() {
    let result = parse_tags("a a", 1);
    assert_eq!(result, Ok(vec!["a".to_string()]));
}

#[test]
fn named_ref_round_trips_on_the_wire() {
    let tag_ref = NamedRef {
        id: "0197c0b0-0000-7000-8000-000000000000".to_string(),
        name: "rust".to_string(),
    };
    let json = serde_json::to_string(&tag_ref).expect("serialize named ref");
    assert_eq!(
        json,
        r#"{"id":"0197c0b0-0000-7000-8000-000000000000","name":"rust"}"#
    );
    let decoded: NamedRef = serde_json::from_str(&json).expect("deserialize named ref");
    assert_eq!(decoded, tag_ref);
}

#[test]
fn error_messages_are_english_and_descriptive() {
    assert_eq!(TagNameError::Empty.to_string(), "tag name cannot be empty");
    assert_eq!(
        TagNameError::TooLong.to_string(),
        "tag name too long (max 32 chars)"
    );
    assert_eq!(
        TagNameError::ContainsForbiddenChar('!').to_string(),
        "tag name cannot contain '!'"
    );
    assert_eq!(
        TagNamesError::TooManyTags { max_count: 3 }.to_string(),
        "too many tags (max 3)"
    );
}
