
use super::*;

#[test]
fn error_precedence_whitelist_before_length() {
    let long_non_ascii: String = "名".repeat(33);
    assert!(matches!(
        validate_name(&long_non_ascii),
        Err(NameError::ContainsForbiddenChar(_))
    ));
    assert!(matches!(
        validate_name("中"),
        Err(NameError::ContainsForbiddenChar(_))
    ));
    let long_ascii = "a".repeat(33);
    assert!(matches!(
        validate_name(&long_ascii),
        Err(NameError::TooLong)
    ));
    let boundary = "a".repeat(32);
    assert_eq!(validate_name(&boundary).unwrap(), boundary);
}

#[test]
fn trims_surrounding_whitespace_and_returns_canonical_form() {
    assert_eq!(validate_name("  alice  ").unwrap(), "alice");
    assert_eq!(validate_name("\t alice_2\n").unwrap(), "alice_2");
    assert_eq!(validate_name("  ").unwrap_err(), NameError::Empty);
    assert!(matches!(
        validate_name("a b"),
        Err(NameError::ContainsForbiddenChar(_))
    ));
}

#[test]
fn name_error_implements_std_error() {
    fn assert_error<E: std::error::Error>() {}
    assert_error::<NameError>();
    assert!(std::error::Error::source(&NameError::Empty).is_none());
}
