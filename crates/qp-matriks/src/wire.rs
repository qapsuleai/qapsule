//! Minimal protobuf wire-format scanner: which top-level field numbers does a
//! payload carry?
//!
//! This is the field-presence source of truth for every message, including
//! proto3 ones (where the generated type cannot tell "absent" from "default").
//! A proto3 encoder normally omits default values, so for proto3 a field that
//! is on the wire was sent and a field that is not may be default *or* absent.

/// Wire-format error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The payload ended in the middle of a key or value.
    Truncated {
        /// Byte offset where the incomplete element starts.
        offset: usize,
    },
    /// A varint was longer than 10 bytes.
    VarintTooLong {
        /// Byte offset of the varint.
        offset: usize,
    },
    /// Field number 0 or above the protobuf maximum.
    BadFieldNumber {
        /// Byte offset of the key.
        offset: usize,
    },
    /// Wire type 3/4 (groups) or 6/7 (invalid). Matriks protos use no groups.
    UnsupportedWireType {
        /// The wire type found.
        wire_type: u8,
        /// Byte offset of the key.
        offset: usize,
    },
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Truncated { offset } => write!(f, "truncated element at byte {offset}"),
            WireError::VarintTooLong { offset } => write!(f, "varint too long at byte {offset}"),
            WireError::BadFieldNumber { offset } => write!(f, "bad field number at byte {offset}"),
            WireError::UnsupportedWireType { wire_type, offset } => {
                write!(f, "unsupported wire type {wire_type} at byte {offset}")
            }
        }
    }
}

impl std::error::Error for WireError {}

const MAX_FIELD_NUMBER: u64 = (1 << 29) - 1;

fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64, WireError> {
    let start = *pos;
    let mut value: u64 = 0;
    for i in 0..10 {
        let Some(&b) = buf.get(*pos) else {
            return Err(WireError::Truncated { offset: start });
        };
        *pos += 1;
        value |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(WireError::VarintTooLong { offset: start })
}

fn skip(buf: &[u8], pos: &mut usize, n: u64, start: usize) -> Result<(), WireError> {
    let end = usize::try_from(n)
        .ok()
        .and_then(|n| pos.checked_add(n))
        .filter(|&end| end <= buf.len())
        .ok_or(WireError::Truncated { offset: start })?;
    *pos = end;
    Ok(())
}

/// Top-level field numbers present in `payload`, sorted, without duplicates.
///
/// Validates the whole payload's framing (not the field contents).
pub fn field_numbers(payload: &[u8]) -> Result<Vec<u32>, WireError> {
    let mut pos = 0usize;
    let mut out: Vec<u32> = Vec::new();
    while pos < payload.len() {
        let key_at = pos;
        let key = read_varint(payload, &mut pos)?;
        let number = key >> 3;
        let wire_type = (key & 7) as u8;
        if number == 0 || number > MAX_FIELD_NUMBER {
            return Err(WireError::BadFieldNumber { offset: key_at });
        }
        match wire_type {
            0 => {
                read_varint(payload, &mut pos)?;
            }
            1 => skip(payload, &mut pos, 8, key_at)?,
            2 => {
                let len = read_varint(payload, &mut pos)?;
                skip(payload, &mut pos, len, key_at)?;
            }
            5 => skip(payload, &mut pos, 4, key_at)?,
            other => {
                return Err(WireError::UnsupportedWireType {
                    wire_type: other,
                    offset: key_at,
                })
            }
        }
        out.push(number as u32);
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_all_wire_types() {
        // field 1 varint 150; field 2 fixed64; field 3 len 2; field 4 fixed32; field 1 again
        let mut b = vec![0x08, 0x96, 0x01, 0x11];
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&[0x1a, 0x02, b'h', b'i', 0x25, 0, 0, 0, 0, 0x08, 0x01]);
        assert_eq!(field_numbers(&b).unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn rejects_truncation_and_groups() {
        assert_eq!(
            field_numbers(&[0x1a, 0x05, b'a']),
            Err(WireError::Truncated { offset: 0 })
        );
        assert_eq!(
            field_numbers(&[0x08]),
            Err(WireError::Truncated { offset: 1 })
        );
        assert!(matches!(
            field_numbers(&[0x0b]),
            Err(WireError::UnsupportedWireType { wire_type: 3, .. })
        ));
        assert!(matches!(
            field_numbers(&[0x00]),
            Err(WireError::BadFieldNumber { .. })
        ));
        assert_eq!(field_numbers(&[]).unwrap(), Vec::<u32>::new());
    }
}
