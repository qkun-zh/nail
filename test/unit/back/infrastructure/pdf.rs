use crate::infrastructure::pdf::{
    PdfGuardError, PdfStreamGuard, content_hash_rel_path, sanitize_attachment_filename,
    valid_content_hash,
};

fn feed(guard: &mut PdfStreamGuard, chunks: &[&[u8]]) -> Result<(), PdfGuardError> {
    for chunk in chunks {
        guard.update(chunk)?;
    }
    guard.finish()
}

fn minimal_pdf() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"%PDF-1.7\n");
    for _ in 0..64 {
        bytes.push(b'x');
    }
    bytes.extend_from_slice(b"\n%%EOF");
    bytes
}

#[test]
fn valid_pdf_is_accepted() {
    let pdf = minimal_pdf();
    let mut guard = PdfStreamGuard::new(1_000_000);
    assert!(feed(&mut guard, &[&pdf]).is_ok());
}

#[test]
fn pdf_smaller_than_minimum_is_rejected() {
    let mut guard = PdfStreamGuard::new(1_000_000);
    let error = feed(&mut guard, &[b"%PDF-1.7"]).expect_err("too small");
    assert!(matches!(error, PdfGuardError::TooSmall { .. }));
}

#[test]
fn pdf_over_the_size_limit_is_rejected() {
    let mut guard = PdfStreamGuard::new(8);
    let error = feed(&mut guard, &[b"%PDF-1.7\n%%EOF"]).expect_err("too large");
    assert!(matches!(error, PdfGuardError::TooLarge { .. }));
}

#[test]
fn pdf_without_header_is_rejected() {
    let mut guard = PdfStreamGuard::new(1_000_000);
    let bytes = b"not a pdf at all\n%%EOF";
    let error = feed(&mut guard, &[bytes]).expect_err("bad header");
    assert!(matches!(error, PdfGuardError::BadHeader));
}

#[test]
fn pdf_with_unsupported_version_is_rejected() {
    let mut guard = PdfStreamGuard::new(1_000_000);
    let bytes = b"%PDF-3.0\npadding\n%%EOF";
    let error = feed(&mut guard, &[bytes]).expect_err("bad version");
    assert!(matches!(error, PdfGuardError::BadVersion));
}

#[test]
fn pdf_without_footer_is_rejected() {
    let mut guard = PdfStreamGuard::new(1_000_000);
    let bytes = b"%PDF-1.7\npadding without a closing marker";
    let error = feed(&mut guard, &[bytes]).expect_err("bad footer");
    assert!(matches!(error, PdfGuardError::BadFooter));
}

#[test]
fn pdf_footer_within_trailing_whitespace_is_accepted() {
    let pdf = minimal_pdf();
    let mut with_whitespace = pdf.clone();
    with_whitespace.extend_from_slice(b"\n\r\n");
    let mut guard = PdfStreamGuard::new(1_000_000);
    assert!(feed(&mut guard, &[&with_whitespace]).is_ok());
}

#[test]
fn content_hash_rel_path_builds_the_two_plus_two_layout() {
    let hash = "abcdef1234567890abcdef1234567890";
    assert_eq!(
        content_hash_rel_path(hash).as_deref(),
        Some("ab/cd/abcdef1234567890abcdef1234567890.pdf")
    );
}

#[test]
fn content_hash_rel_path_rejects_an_invalid_hash() {
    assert_eq!(content_hash_rel_path("short"), None);
    assert_eq!(content_hash_rel_path("ABCDEF1234567890ABCDEF1234567890"), None);
    assert_eq!(content_hash_rel_path("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"), None);
    assert!(valid_content_hash("abcdef1234567890abcdef1234567890"));
    assert!(!valid_content_hash("ABCDEF1234567890abcdef1234567890"));
}

#[test]
fn sanitize_attachment_filename_keeps_hash_derived_names() {
    assert_eq!(
        sanitize_attachment_filename("abcdef1234567890abcdef1234567890.pdf"),
        "abcdef1234567890abcdef1234567890.pdf"
    );
}

#[test]
fn sanitize_attachment_filename_strips_unsafe_characters() {
    assert_eq!(
        sanitize_attachment_filename("my file (1).pdf"),
        "myfile1.pdf"
    );
    assert_eq!(
        sanitize_attachment_filename("../../etc/passwd.pdf"),
        "....etcpasswd.pdf"
    );
}

#[test]
fn sanitize_attachment_filename_falls_back_when_empty() {
    assert_eq!(sanitize_attachment_filename(""), "article.pdf");
    assert_eq!(sanitize_attachment_filename("()/\\"), "article.pdf");
}

#[tokio::test]
async fn upload_places_the_pdf_and_drops_an_unkept_placed_file() {
    let directory = std::env::temp_dir().join(format!("nail_pdf_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).expect("create dir");
    let temp_path = directory.join("tmp.pdf");
    std::fs::write(&temp_path, minimal_pdf()).expect("write temp");

    let final_path = directory.join("final.pdf");
    {
        let upload = crate::infrastructure::pdf::PdfUpload::received(
            "abcdef1234567890abcdef1234567890".to_string(),
            crate::infrastructure::pdf::TempPdf::new(temp_path.clone()),
        );
        let placed = upload.place(final_path.clone()).await.expect("place");
        assert!(final_path.exists());
        drop(placed);
    }
    assert!(!final_path.exists(), "an unkept placed file must be removed");
    assert!(!temp_path.exists(), "the temp file must be moved away");

    let _ = std::fs::remove_dir_all(&directory);
}
