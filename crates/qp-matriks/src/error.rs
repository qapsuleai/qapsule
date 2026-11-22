//! [`DecodeError`]: the decoding error for a Matriks payload (out of `lib.rs`).

use crate::wire;

/// Decoding error for a Matriks payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The payload is not valid protobuf wire format.
    Wire(wire::WireError),
    /// prost could not decode the payload into the expected message.
    Proto(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Wire(e) => write!(f, "wire format: {e}"),
            DecodeError::Proto(e) => write!(f, "protobuf decode: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<wire::WireError> for DecodeError {
    fn from(e: wire::WireError) -> Self {
        DecodeError::Wire(e)
    }
}

impl From<prost::DecodeError> for DecodeError {
    fn from(e: prost::DecodeError) -> Self {
        DecodeError::Proto(e.to_string())
    }
}
