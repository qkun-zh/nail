
use super::*;

const HEX_CHARS: &[u8] = b"0123456789abcdef";

fn assert_hex64(s: &str) {
    assert_eq!(s.len(), 64, "hash must be 32 bytes = 64 hex chars");
    assert!(
        s.bytes().all(|b| HEX_CHARS.contains(&b)),
        "hash must be lowercase hex: {s}"
    );
}

fn assert_hex32(s: &str) {
    assert_eq!(s.len(), 32, "pdf hash must be 16 bytes = 32 hex chars");
    assert!(
        s.bytes().all(|b| HEX_CHARS.contains(&b)),
        "hash must be lowercase hex: {s}"
    );
}

#[test]
fn email_hash_is_deterministic_and_fixed_length() {
    assert_eq!(email("alice@qq.com"), email("alice@qq.com"));
    assert_hex32(&email("alice@qq.com"));
}

#[test]
fn email_hash_is_raw_byte_sensitive_before_normalization() {
    assert_ne!(email("Alice@QQ.com"), email("alice@qq.com"));
    assert_ne!(email(" a@qq.com "), email("a@qq.com"));
}

#[test]
fn email_hash_distinguishes_similar_inputs() {
    assert_ne!(email("a@qq.com"), email("b@qq.com"));
    assert_ne!(email("aa@qq.com"), email("a@qq.com"));
    assert_ne!(email(""), email(" "));
}

#[test]
fn token_hash_is_deterministic_and_fixed_length() {
    assert_eq!(
        token("0196f71a-4c1c-7f00-8000-000000000001"),
        token("0196f71a-4c1c-7f00-8000-000000000001")
    );
    assert_hex64(&token("t"));
}

#[test]
fn email_and_token_hashes_are_domain_separated() {
    for input in ["same-input", "alice@qq.com", ""] {
        assert_ne!(
            email(input),
            token(input),
            "email({input:?}) must differ from token({input:?})"
        );
    }
}

#[test]
fn pdf_hash_is_deterministic_and_fixed_length() {
    assert_eq!(pdf(b"%PDF-1.4 dedup me"), pdf(b"%PDF-1.4 dedup me"));
    assert_hex32(&pdf(b"%PDF-1.4 dedup me"));
}

#[test]
fn pdf_hash_distinguishes_similar_inputs() {
    assert_ne!(pdf(b"%PDF-1.4 a"), pdf(b"%PDF-1.4 b"));
    assert_ne!(pdf(b""), pdf(b" "));
    assert_ne!(pdf(b"%PDF-1.4"), pdf(b"%PDF-1.5"));
}

#[test]
fn pdf_hash_handles_empty_and_large_input() {
    assert_hex32(&pdf(b""));
    let big = vec![0xabu8; 10 * 1024];
    assert_eq!(pdf(&big), pdf(&big));
    assert_hex32(&pdf(&big));
}

#[test]
fn pdf_hasher_chunked_feed_matches_full_input() {
    let data: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
    let full = pdf(&data);

    for chunk_size in [1usize, 7, 64 * 1024] {
        let mut h = PdfHasher::new();
        for chunk in data.chunks(chunk_size) {
            h.update(chunk);
        }
        assert_eq!(h.finalize(), full, "chunk_size={chunk_size}");
    }
}

#[test]
fn pdf_hasher_crosses_64k_boundary_like_pdf() {
    let data: Vec<u8> = (0..(70 * 1024)).map(|i| (i % 257) as u8).collect();
    let full = pdf(&data);

    let mut h = PdfHasher::new();
    for chunk in data.chunks(4096) {
        h.update(chunk);
    }
    assert_eq!(h.finalize(), full);

    let mut h = PdfHasher::new();
    for &b in &data {
        h.update(&[b]);
    }
    assert_eq!(h.finalize(), full);
}

#[test]
fn pdf_hasher_empty_input_is_fixed_length() {
    assert_hex32(&PdfHasher::new().finalize());
    assert_eq!(PdfHasher::new().finalize(), pdf(b""));
}

#[test]
fn pdf_hasher_is_deterministic() {
    let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let a = {
        let mut h = PdfHasher::new();
        h.update(&data);
        h.finalize()
    };
    let b = {
        let mut h = PdfHasher::new();
        h.update(&data);
        h.finalize()
    };
    assert_eq!(a, b);
    assert_hex32(&a);
}
