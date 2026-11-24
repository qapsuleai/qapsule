//! (RF1 S3.3, moved from `qp-authority` `replica.rs`) Digests of the replica check
//! (§19.6): FNV-1a 64 and the document digest. The authority's `stories` topic keys and the
//! replica, the viewer and the tape compare with them.

use qp_schema::Document;

/// FNV-1a 64 over `bytes` (standard offset basis and prime).
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Document digest (§19.6): FNV-1a 64 of the document's JSON bytes (`serde_json`, the
/// canonical field order of the schema types), 16 lowercase hex digits.
#[must_use]
pub fn document_digest(doc: &Document) -> String {
    let bytes = serde_json::to_vec(doc).unwrap_or_default();
    format!("{:016x}", fnv1a64(&bytes))
}
