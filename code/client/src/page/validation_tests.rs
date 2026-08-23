use crate::page::validation::{
    looks_like_pdf, validate_comment_content, validate_name, validate_note, validate_pdf_selection,
    validate_summary, validate_title, validate_uuid,
};

#[test]
fn mirrors_the_name_rules() {
    assert!(validate_name("").is_err());
    assert_eq!(validate_name("ok_name-1"), Ok("ok_name-1".to_string()));
    assert!(validate_name("bad name").is_err());
    assert!(validate_name("x".repeat(33).as_str()).is_err());
    assert!(validate_name("名字").is_err());
}

#[test]
fn title_rejects_newlines_and_overlong_input() {
    assert!(validate_title("", 200).is_err());
    assert_eq!(
        validate_title("A fine title", 200),
        Ok("A fine title".to_string())
    );
    assert!(validate_title("line\nbreak", 200).is_err());
    assert!(validate_title("x".repeat(201).as_str(), 200).is_err());
}

#[test]
fn summary_allows_newlines_within_its_limit() {
    assert_eq!(
        validate_summary("line one\nline two", 2000),
        Ok("line one\nline two".to_string())
    );
    assert!(validate_summary("x".repeat(2001).as_str(), 2000).is_err());
}

#[test]
fn note_obeys_its_ascii_limit() {
    assert_eq!(validate_note("a note", 1024), Ok("a note".to_string()));
    assert!(validate_note("x".repeat(1025).as_str(), 1024).is_err());
    assert!(validate_note("emoji 中", 1024).is_err());
}

#[test]
fn comment_content_obeys_its_ascii_limit() {
    assert_eq!(validate_comment_content("hi", 1024), Ok("hi".to_string()));
    assert!(validate_comment_content("", 1024).is_err());
    assert!(validate_comment_content("x".repeat(1025).as_str(), 1024).is_err());
}

#[test]
fn uuid_rejects_bad_format_and_accepts_canonical() {
    let canonical = "01a018c7-f177-7da1-a821-5f1000648383";
    assert_eq!(validate_uuid(canonical), Ok(canonical.to_string()));
    assert_eq!(
        validate_uuid(" 01a018c7-f177-7da1-a821-5f1000648383 "),
        Ok(canonical.to_string())
    );
    assert!(validate_uuid("01a00100-d22d-73c3-a5c3-c54e9b8f6f3").is_err());
    assert!(validate_uuid("01a00100-d22d-73c3-a5c3-c54e9b8f6f32z").is_err());
    assert_eq!(
        validate_uuid("01a00100d22d73c3a5c3c54e9b8f6f32"),
        Ok("01a00100-d22d-73c3-a5c3-c54e9b8f6f32".to_string())
    );
    assert!(validate_uuid("01a00100-d22d-73c3-a5c3-c54e9b8f6f32-").is_err());
    assert!(validate_uuid("not-a-uuid").is_err());
    assert!(validate_uuid("").is_err());
}

#[test]
fn sniffs_pdf_by_mime_type_or_name() {
    assert!(looks_like_pdf("application/pdf", "file.bin"));
    assert!(looks_like_pdf("", "file"));
    assert!(looks_like_pdf("application/octet-stream", "file"));
    assert!(looks_like_pdf("text/plain", "file.pdf"));
    assert!(!looks_like_pdf("text/plain", "file.txt"));
}

#[test]
fn pdf_selection_enforces_type_and_size() {
    assert!(validate_pdf_selection("application/pdf", "a.pdf", 10, 100).is_ok());
    assert!(validate_pdf_selection("text/plain", "a.txt", 10, 100).is_err());
    assert!(validate_pdf_selection("application/pdf", "a.pdf", 101, 100).is_err());
    assert!(validate_pdf_selection("application/pdf", "a.pdf", 100, 100).is_ok());
}
