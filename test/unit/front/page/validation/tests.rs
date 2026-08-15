use crate::page::validation::{
    looks_like_pdf, validate_comment_content, validate_name, validate_note, validate_pdf_selection,
    validate_summary, validate_tags, validate_title,
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
fn tags_mirror_the_backend_parser() {
    assert_eq!(
        validate_tags("a b", 8),
        Ok(vec!["a".to_string(), "b".to_string()])
    );
    assert!(validate_tags("no-hash", 8).is_ok());
    assert!(validate_tags("", 8).is_err());
    let nine = "1 2 3 4 5 6 7 8 9";
    assert!(validate_tags(nine, 8).is_err());
    assert_eq!(validate_tags("ab", 8), Ok(vec!["ab".to_string()]));
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
