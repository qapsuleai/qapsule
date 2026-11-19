//! Matriks MQTT message layer for Qapsule (R6).
//!
//! Pure: no network, no clock, no randomness. Bytes come in through the
//! caller (live connector or raw-log replay), typed values and events go out.
//!
//! - [`messages`]: prost types generated at build time from the byte-exact
//!   Matriks protos in `proto/` (K62; provenance in `proto/PROVENANCE.md`).
//! - [`fields`]: per-message field number → proto name table (generated).
//! - [`wire`]: which field numbers a payload carries (field presence from the
//!   wire, also for proto3 messages).
//! - [`topic`]: Matriks topic parsing.
//! - [`symbol`]: price-card (`SymbolMessage`) book with protobuf merge
//!   semantics: a field that did not arrive did not change.
//! - [`clock`]: `TimeMessage` decoding and the [`clock::MarketClock`].
//! - [`notice`]: `MessengerMessage` classification (multi-connect = stop).
//! - [`status`]: `stockStatus` codes → session phase ("seans evresi").
//! - [`rawlog`]: binary, appendable raw message log for record and replay.
//! - [`feed`]: [`LiveFeed`], the one pipeline from raw records to §18.4 data events,
//!   shared by the live connector and replay.

#![deny(missing_docs)]

/// Prost types generated from `proto/*.proto` (package `messages`).
///
/// Proto2 `optional` scalars are `Option<T>`: `None` means the field was not
/// on the wire. Proto2 `required` fields are plain values. Proto3 scalars
/// (`TimeMessage`, `DepthStatsMessage`, `QStatus`, `ViopAgentListMessage`)
/// carry no presence: absent and default look the same.
#[allow(missing_docs, clippy::all, clippy::pedantic, rustdoc::all)]
pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

pub mod clock;
pub mod feed;
pub mod fields;
pub mod notice;
pub mod rawlog;
pub mod status;
pub mod symbol;
pub mod topic;
pub mod wire;

pub use clock::{decode_time, ClockEvent, MarketClock, TimeTick};
pub use feed::{
    parse_update_date, ConnChange, FeedOut, FeedStats, LiveFeed, LiveFeedConfig, TimeSource,
    VolumeField,
};
pub use fields::FieldSet;
pub use notice::{classify_notice, BrokerNotice, NoticeInfo};
pub use rawlog::{RawError, RawReader, RawRecord, RawWriter, RecordKind};
pub use status::{session_phase, SessionPhase, SESSION_PHASES};
pub use symbol::{InitialPolicy, SymbolBook, SymbolUpdate, UpdateKind};
pub use topic::{parse_topic, Topic};

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
