#[test]
fn hash_is_32_hex_chars() {
    let digest = crate::hash::hash(b"alice@example.com").expect("hash must succeed");
    assert_eq!(digest.len(), 32);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn hash_is_deterministic() {
    let first = crate::hash::hash(b"alice@example.com").expect("hash must succeed");
    let second = crate::hash::hash(b"alice@example.com").expect("hash must succeed");
    assert_eq!(first, second);
}

#[test]
fn hash_distinguishes_distinct_inputs() {
    let plain = crate::hash::hash(b"alice@example.com").expect("hash must succeed");
    let padded = crate::hash::hash(b"alice@example.com ").expect("hash must succeed");
    let upper = crate::hash::hash(b"Alice@example.com").expect("hash must succeed");
    assert_ne!(plain, padded);
    assert_ne!(plain, upper);
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
