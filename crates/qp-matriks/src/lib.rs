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
//! - [`trade`]: `TradeMessage` → [`Trade`] (level 1 only; buyer / seller never kept),
//!   configurable time unit, de-duplication key, the [`TradeHistory`] repair interface.
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
pub mod messages;

pub mod clock;
mod error;
pub mod feed;
pub mod fields;
pub mod notice;
pub mod rawlog;
pub mod status;
pub mod symbol;
pub mod topic;
pub mod trade;
pub mod wire;

pub use clock::{decode_time, ClockEvent, MarketClock, TimeTick};
pub use error::DecodeError;
pub use feed::{
    parse_update_date, ConnChange, FeedOut, FeedSource, FeedStats, LiveFeed, LiveFeedConfig,
    TimeSource, TradeFeedConfig, TradeFeedStats, TradeGap, VolumeField,
};
pub use fields::FieldSet;
pub use notice::{classify_notice, BrokerNotice, NoticeInfo};
pub use rawlog::{RawError, RawReader, RawRecord, RawWriter, RecordKind};
pub use status::{session_phase, SessionPhase, SESSION_PHASES};
pub use symbol::{InitialPolicy, SymbolBook, SymbolUpdate, UpdateKind};
pub use topic::{
    parse_topic, trade_topic, trade_topic_at, trade_topics, Topic, TopicError, TradeLevel,
    TradeStream,
};
pub use trade::{
    decode_trade, decode_trade_raw, level2_presence, sanitize_trade_payload, Aggressor, Trade,
    TradeDecodeConfig, TradeError, TradeHistory, TradeKey, TradeRawView, TradeTimeUnit,
    TRADE_FIELDS,
};
