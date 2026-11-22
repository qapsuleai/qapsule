//! Replay controls and replay state (R4, §21).

use super::*;

/// (R4, §21) A replay control. A **step** is one distinct virtual time of the replay
/// (every feed's events at that time); step boundaries do not depend on the speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayControl {
    /// Where the replay is (no change).
    Status,
    /// Stop advancing with the wall clock (explicit `step` and `seek` still work).
    Pause,
    /// Advance with the wall clock again, at the current speed.
    Resume,
    /// Set the speed: steps per wall-clock second (`0 < steps_per_s <= 1000`).
    Speed {
        /// Steps per second.
        steps_per_s: f64,
    },
    /// Take `n` steps now (default 1, at most 100000); the notifications they cause
    /// follow the `result`.
    Step {
        /// Number of steps.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        n: Option<u32>,
    },
    /// Go to a position: exactly one of `t` (every step at or before `t` taken), `date`
    /// (`YYYY-MM-DD`: the start of that day in the exchange time zone, nothing of that
    /// day taken) or `step` (that many steps taken). Forward: the steps in between are
    /// taken and their notifications follow the `result`. Backward: the authority is
    /// rebuilt from its init and advanced silently to the position (a new authority,
    /// `epoch + 1`, §8.4): every connection is closed and must send `hello` again.
    Seek {
        /// Virtual time, epoch ms.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t: Option<TimestampMs>,
        /// A date in the exchange time zone.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        /// A step count.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<u64>,
    },
}

/// (R4, §21) Where the replay data comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplaySourceKind {
    /// The deterministic synthetic market (`qp_source::synthetic_market`).
    #[default]
    Synthetic,
    /// Real bars fetched to the host's machine (`fixtures/local/`, K23).
    Local,
}

/// (R4, §21) Replayed days by how a bar was built inside the day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayDays {
    /// From finer bars (e.g. hourly bars of a daily feed).
    pub fine: u64,
    /// On the labelled synthetic path open → extreme → extreme → close (no fine data).
    pub ohlc: u64,
    /// As one whole bar.
    pub bar: u64,
}

/// (R4, §21) The state of a replay (answer to every `replay` control).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayState {
    /// Virtual time (the authority's clock): the time of the last step taken.
    pub t: TimestampMs,
    /// Steps taken.
    pub step: u64,
    /// Steps of the whole replay.
    pub steps: u64,
    /// Data events delivered so far.
    pub events: u64,
    /// Time of the first step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// Time of the last step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
    /// Not advancing with the wall clock.
    pub paused: bool,
    /// Every step taken.
    pub done: bool,
    /// Steps per wall-clock second.
    pub speed: f64,
    /// True when the data is synthetic (a page must say so on screen).
    pub synthetic: bool,
    /// Where the data comes from.
    pub source: ReplaySourceKind,
    /// Why the source differs from the one asked for (e.g. no local data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Number of instruments (feeds).
    pub instruments: u32,
    /// Bar interval of the feeds.
    pub interval: String,
    /// Interval of the finer bars that move a bar inside its day, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine: Option<String>,
    /// Replayed days by how they were built.
    pub days: ReplayDays,
    /// Authority epoch (grows with every backward seek).
    pub epoch: u64,
    /// True when this control rebuilt the authority (a backward seek): every connection
    /// was closed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub reset: bool,
}
