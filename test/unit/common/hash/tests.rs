use crate::hash::email;

#[test]
fn email_hash_is_32_hex_chars() {
    let digest = email("alice@example.com");
    assert_eq!(digest.len(), 32);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn email_hash_is_deterministic() {
    assert_eq!(email("alice@example.com"), email("alice@example.com"));
}

#[test]
fn email_hash_distinguishes_distinct_inputs() {
    let plain = email("alice@example.com");
    let padded = email("alice@example.com ");
    let upper = email("Alice@example.com");
    assert_ne!(plain, padded);
    assert_ne!(plain, upper);
}

#[test]
fn token_hash_is_result_based_and_64_hex_chars() {
    let digest = crate::hash::token("0197c0b0-0000-7000-8000-000000000000");
    let digest = digest.expect("token hash must succeed");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn token_hash_is_deterministic() {
    let token_string = "0197c0b0-0000-7000-8000-000000000000";
    let first = crate::hash::token(token_string).expect("token hash must succeed");
    let second = crate::hash::token(token_string).expect("token hash must succeed");
    assert_eq!(first, second);
}

#[test]
fn token_hash_differs_from_email_hash() {
    let token_string = "alice@example.com";
    let token_digest = crate::hash::token(token_string).expect("token hash must succeed");
    let email_digest = email(token_string);
    assert_ne!(token_digest, email_digest);
}

#[test]
fn pdf_hash_is_32_hex_chars_and_deterministic() {
    let data = b"%PDF-1.7 hello world %%EOF";
    let first = crate::hash::pdf(data);
    let second = crate::hash::pdf(data);
    assert_eq!(first.len(), 32);
    assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(first, second);
}

#[test]
fn pdf_hash_is_independent_of_chunk_boundaries() {
    use crate::hash::PdfHasher;
    let data: Vec<u8> = (0..5000u32).map(|value| (value % 251) as u8).collect();
    let one_shot = crate::hash::pdf(&data);
    let mut hasher = PdfHasher::new();
    for chunk in data.chunks(1) {
        hasher.update(chunk);
    }
    assert_eq!(hasher.finalize(), one_shot);
    let mut hasher = PdfHasher::new();
    for chunk in data.chunks(7) {
        hasher.update(chunk);
    }
    assert_eq!(hasher.finalize(), one_shot);
}
