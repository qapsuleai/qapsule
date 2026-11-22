//! What a [`LiveFeed`](super::LiveFeed) reports: connection changes, the per-record
//! output and the counters (split out of `feed.rs` for size).

use super::*;

/// A connection change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnChange {
    /// A connection opened; `session` counts them from 1.
    Opened {
        /// 1 for the first connection, 2 after the first reconnect, …
        session: u32,
    },
    /// The connection was lost or closed: data is stale until the next one.
    Lost,
}

/// What one record caused.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedOut {
    /// §18.4 data events, per feed with a gapless local `seq`.
    pub events: Vec<DataEvent>,
    /// The new exchange clock (UTC ms) if it moved.
    pub clock_ms: Option<i64>,
    /// A connection change.
    pub connection: Option<ConnChange>,
    /// `(symbol, stockStatus)` for symbols whose phase arrived or changed.
    pub phases: Vec<(String, i32)>,
    /// Bars that became final: feed and bar open time.
    pub finalized: Vec<(FeedKey, i64)>,
    /// A trade-endpoint connection change (rule 9).
    pub trade_connection: Option<ConnChange>,
}

impl FeedOut {
    /// Whether nothing happened.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
            && self.clock_ms.is_none()
            && self.connection.is_none()
            && self.phases.is_empty()
            && self.finalized.is_empty()
            && self.trade_connection.is_none()
    }
}

/// Counters (no market values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedStats {
    /// Records pushed.
    pub records: u64,
    /// Price-card messages decoded.
    pub symbol_messages: u64,
    /// Of those, initials.
    pub initials: u64,
    /// Payloads that failed to decode (state unchanged).
    pub decode_errors: u64,
    /// `mx/timestamp` messages.
    pub time_messages: u64,
    /// Other topics (messenger, …): ignored.
    pub other_messages: u64,
    /// Card messages without any bar field (not pushed to the builders).
    pub no_bar_fields: u64,
    /// Pushed ticks timed by their own `updateDate` (or arrival in `Arrival` mode).
    pub time_direct: u64,
    /// Pushed ticks timed by the anchor rule (no / unreadable `updateDate`).
    pub time_derived: u64,
    /// Ticks dropped: no time known yet.
    pub time_unknown: u64,
    /// Ticks whose time was before the clock when they arrived (out of order).
    pub behind_clock: u64,
    /// Largest such lag (ms); the finalize grace should cover most of it.
    pub max_behind_ms: i64,
    /// Unreadable `updateDate` values.
    pub bad_update_date: u64,
    /// `updateDate` values too far ahead of arrival to move the clock.
    pub clock_lead_rejected: u64,
    /// Ticks pushed to builders (per symbol message, not per interval).
    pub ticks: u64,
    /// Connections opened.
    pub connections_opened: u64,
    /// Connections lost.
    pub connections_lost: u64,
    /// `Subscribed` records taken as re-subscriptions.
    pub resubscribes: u64,
    /// `Subscribed` records that needed nothing (first subscription or already fresh).
    pub subscribed_ignored: u64,
    /// `SubscribeRejected` records.
    pub subscribe_rejected: u64,
    /// Note records.
    pub notes: u64,
    /// Configured intervals that could not be parsed.
    pub bad_intervals: u64,
    /// Feeds (symbol × interval).
    pub feeds: u64,
    /// Bars reported final.
    pub finalized: u64,
    /// Sum over feeds: ticks outside every trading window.
    pub off_session: u64,
    /// Sum over feeds: ticks for final bars.
    pub late: u64,
    /// Sum over feeds: trade evidence without a known price.
    pub no_price: u64,
    /// Sum over feeds: invalid field values dropped.
    pub invalid_fields: u64,
    /// Sum over feeds: counter went backwards.
    pub volume_regressions: u64,
    /// Sum over feeds: snapshot counter taken as the day's base.
    pub volume_rebases: u64,
    /// Sum over feeds: day extremes reflected into intraday bars (`Mixed`).
    pub extremes_reflected: u64,
    /// Sum over feeds: bars opened.
    pub bars_opened: u64,
    /// Sum over feeds: data events.
    pub events: u64,
    /// Trade stream counters (rule 9).
    pub trades: TradeFeedStats,
}

/// Trade stream counters (no market values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeFeedStats {
    /// Trade messages received.
    pub messages: u64,
    /// Payloads that are not a `TradeMessage`.
    pub decode_errors: u64,
    /// Trades without a valid price.
    pub no_price: u64,
    /// Trades without a usable timestamp.
    pub no_time: u64,
    /// Trades whose topic symbol and payload symbol differ (the topic wins).
    pub symbol_mismatch: u64,
    /// Decoded trades, by the time unit actually used (`Auto` shows which one won).
    pub unit_ns_from_midnight: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_ms: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_us: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_ns: u64,
    /// Live trades whose time is more than 60 s from their arrival (a wrong unit shows
    /// here first).
    pub far_from_arrival: u64,
    /// Special trades seen (`isTradeEx` or `mx/tradeex`).
    pub special: u64,
    /// Trades dropped as duplicates (same trade number).
    pub duplicates: u64,
    /// Live trades held while their symbol had a gap.
    pub buffered: u64,
    /// History trades before the gap start (already covered): dropped.
    pub history_before_gap: u64,
    /// History trades received inside gaps (before de-duplication).
    pub history_applied: u64,
    /// Gaps filled.
    pub repairs: u64,
    /// History requests that failed (the gap stays).
    pub repair_errors: u64,
    /// Trade-endpoint connections opened.
    pub connections_opened: u64,
    /// Trade-endpoint connections lost.
    pub connections_lost: u64,
}
