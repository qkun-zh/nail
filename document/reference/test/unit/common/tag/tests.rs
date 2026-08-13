
use super::*;

#[test]
fn parse_tags_adjacent_or_space_separated() {
    assert_eq!(
        parse_hashtag_tags("#hello#big#ball", 8).unwrap(),
        vec!["#hello", "#big", "#ball"]
    );
    assert_eq!(
        parse_hashtag_tags("#hello #big  #ball", 8).unwrap(),
        vec!["#hello", "#big", "#ball"]
    );
    assert_eq!(
        parse_hashtag_tags("  #a  #b  ", 8).unwrap(),
        vec!["#a", "#b"]
    );
    assert_eq!(parse_hashtag_tags("", 8).unwrap(), Vec::<String>::new());
    assert_eq!(parse_hashtag_tags("   ", 8).unwrap(), Vec::<String>::new());
}

#[test]
fn parse_tags_rejects_empty_and_isolated_hash() {
    assert!(matches!(
        parse_hashtag_tags("#", 8),
        Err(TagNamesError::Name(TagNameError::Empty))
    ));
    assert!(matches!(
        parse_hashtag_tags("#a#", 8),
        Err(TagNamesError::Name(TagNameError::Empty))
    ));
    assert!(matches!(
        parse_hashtag_tags("#a# #b", 8),
        Err(TagNamesError::Name(TagNameError::Empty))
    ));
    assert!(matches!(
        parse_hashtag_tags("##a", 8),
        Err(TagNamesError::Name(TagNameError::Empty))
    ));
}

#[test]
fn parse_tags_rejects_bare_text() {
    assert!(matches!(
        parse_hashtag_tags("lebron", 8),
        Err(TagNamesError::Name(TagNameError::MissingHash))
    ));
    assert!(matches!(
        parse_hashtag_tags("#a lebron", 8),
        Err(TagNamesError::Name(TagNameError::MissingHash))
    ));
}

#[test]
fn parse_tags_is_case_sensitive_and_dedupes_exact() {
    assert_eq!(
        parse_hashtag_tags("#Hello#hello#Hello", 8).unwrap(),
        vec!["#Hello", "#hello"]
    );
    assert_eq!(
        parse_hashtag_tags("#Rust#rust#Go", 8).unwrap(),
        vec!["#Rust", "#rust", "#Go"]
    );
}

#[test]
fn parse_tags_accepts_charset_and_length() {
    assert_eq!(
        parse_hashtag_tags("#a-b_c1#0x#A9", 8).unwrap(),
        vec!["#a-b_c1", "#0x", "#A9"]
    );
    assert_eq!(
        parse_hashtag_tags(&format!("#{}", "a".repeat(31)), 8).unwrap(),
        vec![format!("#{}", "a".repeat(31))]
    );
    assert!(matches!(
        parse_hashtag_tags(&format!("#{}", "a".repeat(32)), 8),
        Err(TagNamesError::Name(TagNameError::TooLong))
    ));
}

#[test]
fn parse_tags_rejects_forbidden_chars() {
    assert!(matches!(
        parse_hashtag_tags("#hello world", 8),
        Err(TagNamesError::Name(TagNameError::MissingHash))
    ));
    assert!(matches!(
        parse_hashtag_tags("#你好", 8),
        Err(TagNamesError::Name(TagNameError::ContainsForbiddenChar(_)))
    ));
    assert!(matches!(
        parse_hashtag_tags("#a.b", 8),
        Err(TagNamesError::Name(TagNameError::ContainsForbiddenChar(
            '.'
        )))
    ));
    assert!(matches!(
        validate_tag_name("#a#b"),
        Err(TagNameError::ContainsForbiddenChar('#'))
    ));
}

#[test]
fn parse_tags_caps_count() {
    assert!(matches!(
        parse_hashtag_tags("#a#b#c#d#e#f#g#h#i", 8),
        Err(TagNamesError::TooManyTags { max_count: 8 })
    ));
}
