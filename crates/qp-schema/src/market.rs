//! (R6/T3) Market status: the exchange's time, whether the market is open, each
//! instrument's session phase and whether the live data is stale (`docs/PROTOCOL.md` §24,
//! `docs/CONCEPTS.md` "Canlı veri (R6)").
//!
//! The host builds it (from the Matriks exchange clock `mx/timestamp` when it comes, else
//! from the price cards' times, the exchange calendar and each card's `stockStatus`) and
//! gives it to the authority with `Session::set_market_status`. It is an input next to
//! the clock (`host_clock`), not the clock: the authority's time still comes from
//! `host_clock`.
//!
//! The coarse class of a session phase ([`PhaseClass`]) is derived from the phase's
//! machine **key** (`qp_matriks::status::SessionPhase::key`, the snake case of the proto's
//! fixed name, K62), never from its numeric code: the schema holds no copy of the
//! decoder's code table and does not depend on the decoder, and an unknown key is
//! [`PhaseClass::Other`].

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::TimestampMs;

/// Where the market status came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarketSource {
    /// The exchange clock message (`mx/timestamp`, source `MX`): its open/holiday flags.
    Clock,
    /// No exchange clock: the exchange time is the price cards' update time and
    /// open/holiday come from the exchange calendar (`qp-calendar`, XIST).
    Calendar,
}

/// One instrument's session phase as the feed gives it (Matriks `stockStatus`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhaseCode {
    /// The feed's code (`stockStatus`, e.g. 2). Never shown to the reader; the authority
    /// does not pass it to readers (0, not sent).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub code: i32,
    /// The phase's machine key (`continuous`, `closing_session`, …;
    /// `qp_matriks::status::SessionPhase::key`).
    pub key: String,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(v: &i32) -> bool {
    *v == 0
}

impl PhaseCode {
    /// Its coarse class (from the key).
    #[must_use]
    pub fn class(&self) -> PhaseClass {
        PhaseClass::of_key(&self.key)
    }
}

/// The coarse class of a session phase: what the reader is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhaseClass {
    /// No trading: closed, outside the session, session closed.
    Closed,
    /// Opening auction (order collection before the open).
    PreOpen,
    /// Continuous trading (also the primary, odd-lot and after-hours continuous phases).
    Continuous,
    /// Closing auction (order collection for the close).
    ClosingAuction,
    /// Trading at the closing (last) price.
    ClosingPriceTrades,
    /// Halted: suspended, or order collection after a circuit breaker.
    Halted,
    /// Anything else (uncrossing, break, one-sided auctions, dissemination, unknown).
    Other,
}

impl PhaseClass {
    /// The class of phase key `key` (`qp_matriks::status::SessionPhase::key`); unknown
    /// keys are [`PhaseClass::Other`].
    #[must_use]
    pub fn of_key(key: &str) -> Self {
        match key {
            "continuous"
            | "primary_continuous"
            | "odd_continuous"
            | "continuous_after_hours_morning_session"
            | "continuous_after_hours_evening_session"
            | "continuous_after_cutoff" => Self::Continuous,
            "opening_session" | "opening_session_wo_eq" => Self::PreOpen,
            "closing_session" | "closing_session_wo_eq" => Self::ClosingAuction,
            "continuous_last_trd_pr" | "odd_cont_last_trd_pr" => Self::ClosingPriceTrades,
            "suspend"
            | "order_collection_period_after_cb"
            | "order_collection_period_after_cb_wo_eq" => Self::Halted,
            "closed"
            | "closed_after_hours_morning_session"
            | "closed_after_hours_evening_session"
            | "non_tradable_period"
            | "non_trading_period_after_hours_morning_session"
            | "non_trading_period_after_hours_evening_session"
            | "pmd_session_closed"
            | "fixed_income_session_closed"
            | "derivatives_session_closed" => Self::Closed,
            _ => Self::Other,
        }
    }
}

/// (R6/T3, §24) What the host knows about the market now. An input of the authority
/// (`Session::set_market_status`), next to — not instead of — the clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketStatus {
    /// The exchange's time (UTC ms): the clock message's, else the newest price card's
    /// update time (15 minutes behind in delayed mode). While stale: the last data's.
    pub exchange_time: TimestampMs,
    /// Whether the equity market is open (`None`: unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    /// Whether today is an exchange holiday (`None`: unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holiday: Option<bool>,
    /// Where `open`/`holiday` came from.
    pub source: MarketSource,
    /// The feed is the delayed one (15 minutes).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub delayed: bool,
    /// The live connection is down (or stopped): the data is stale (`BindingState::Stale`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    /// Session phase by instrument symbol (as the price cards last gave it).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub phases: BTreeMap<String, PhaseCode>,
}

impl MarketStatus {
    /// The session phase class of instrument `symbol`, if the feed gave one.
    #[must_use]
    pub fn phase_of(&self, symbol: &str) -> Option<PhaseClass> {
        self.phases.get(symbol).map(PhaseCode::class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_of_keys() {
        assert_eq!(PhaseClass::of_key("continuous"), PhaseClass::Continuous);
        assert_eq!(PhaseClass::of_key("opening_session"), PhaseClass::PreOpen);
        assert_eq!(
            PhaseClass::of_key("closing_session_wo_eq"),
            PhaseClass::ClosingAuction
        );
        assert_eq!(
            PhaseClass::of_key("continuous_last_trd_pr"),
            PhaseClass::ClosingPriceTrades
        );
        assert_eq!(PhaseClass::of_key("suspend"), PhaseClass::Halted);
        assert_eq!(PhaseClass::of_key("closed"), PhaseClass::Closed);
        assert_eq!(PhaseClass::of_key("uncrossing"), PhaseClass::Other);
        assert_eq!(PhaseClass::of_key("no_such_key"), PhaseClass::Other);
    }

    #[test]
    fn json_round_trip_and_defaults() {
        let s = MarketStatus {
            exchange_time: 1_700_000_000_000,
            open: Some(true),
            holiday: None,
            source: MarketSource::Calendar,
            delayed: true,
            stale: false,
            phases: BTreeMap::from([(
                "THYAO".to_owned(),
                PhaseCode {
                    code: 2,
                    key: "continuous".to_owned(),
                },
            )]),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(
            j,
            r#"{"exchange_time":1700000000000,"open":true,"source":"calendar","delayed":true,"phases":{"THYAO":{"code":2,"key":"continuous"}}}"#
        );
        assert_eq!(serde_json::from_str::<MarketStatus>(&j).unwrap(), s);
        assert_eq!(s.phase_of("THYAO"), Some(PhaseClass::Continuous));
        assert_eq!(s.phase_of("GARAN"), None);
    }
}
