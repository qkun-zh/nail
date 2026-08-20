// Probe 003: unified hash() salt=value construction. Source: pinned
// ascon-xof128-0.2.1 (src/cxof.rs:31-68 try_new_customized; OutputSizeUser
// U32, CollisionResistance U16) — AsconCxof128 is the only variant accepting
// a customization; salt=value keeps digests deterministic so email lookup
// keeps working. Acceptance question: is hash() deterministic, 128-bit
// (32 hex chars), distinct across distinct inputs, and error-free for
// normal inputs?

#[test]
fn probe_003_salt_equals_value_deterministic() {
    let value = b"alice@example.com";
    let first = crate::hash::hash(value).expect("hash must succeed");
    let second = crate::hash::hash(value).expect("hash must succeed");
    assert_eq!(first, second, "salt=value must be deterministic");
    assert_eq!(first.len(), 32, "128-bit digest must be 32 hex chars");
    assert!(
        first.chars().all(|ch| ch.is_ascii_hexdigit()),
        "digest must be hex"
    );
    let empty = crate::hash::hash(b"").expect("empty input must hash");
    assert_eq!(empty.len(), 32, "empty input still yields 32 hex chars");
    let padded = crate::hash::hash(b"alice@example.com ").expect("hash must succeed");
    assert_ne!(first, padded, "distinct inputs must differ");
    let upper = crate::hash::hash(b"Alice@example.com").expect("hash must succeed");
    assert_ne!(first, upper, "distinct inputs must differ");
}
