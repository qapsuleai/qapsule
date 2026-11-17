//! (R5/M18, K50, K54, K53, K48, K33, K23; `docs/PROTOCOL.md` §23) The instrument's
//! story list: every capsule (story) of one instrument whose life meets a range, as the
//! reader may receive it — what the chart depth (the story chart) draws. It carries no
//! bars (K23): bar indexes and times only; the device draws them on the bars of its own
//! data subscription.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::capsule::{Direction, MeaningKind, Stage, Validity};
use crate::types::TimestampMs;

/// Rows of a `stories` answer without `limit`.
pub const STORIES_DEFAULT_LIMIT: u32 = 200;
/// Largest `limit` of a `stories` query.
pub const STORIES_MAX_LIMIT: u32 = 500;

/// How a closed story ended (the closing stage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoryOutcome {
    /// The expected outcome happened.
    Realized,
    /// The claim broke.
    Invalidated,
    /// Its life ran out without an outcome.
    Expired,
    /// Withdrawn after a data correction or an upheld dispute (never in the ledger).
    Retracted,
}

impl StoryOutcome {
    /// The outcome of a closing stage (`None`: the stage is not closing).
    #[must_use]
    pub fn of(stage: Stage) -> Option<Self> {
        match stage {
            Stage::Realized => Some(Self::Realized),
            Stage::Invalidated => Some(Self::Invalidated),
            Stage::Expired => Some(Self::Expired),
            Stage::Retracted => Some(Self::Retracted),
            _ => None,
        }
    }
}

/// Why a story is not in the reader's feed (`in_feed: false`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedHidden {
    /// The instrument is not in the reader's watch list (nor a position or an active
    /// intent's instrument).
    Watchlist,
    /// (K53) Its ledger is near a coin flip ("yazı tura") and the reader's sensitivity is
    /// not `all`. The rule is the host's (chosen by M19); without one, never.
    CoinFlip,
    /// Its importance tier is below the reader's sensitivity level.
    Sensitivity,
}

/// The face summary of a story for the reader: the headline pieces (`HeadlineParts` of
/// the grammar) in the reader's language. A face design builds its headline from them.
/// Written as of the series' last closed bar when the row was made; for a reader who may
/// not see digits (`redacted`), every text is digit-free and `ledger_n` is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryFace {
    /// What happened, a short noun phrase ("yukarı kırılım" / "upside breakout").
    pub event: String,
    /// The direction as one word.
    pub direction_word: String,
    /// When it was born, as a phrase by the exchange calendar ("dün", "3 işlem günü önce").
    pub when: String,
    /// When it last moved, the same kind of phrase.
    pub moved: String,
    /// Whether it still holds, or how it ended, in words.
    pub validity_phrase: String,
    /// The ledger at birth, shortened ("7'de 3 tuttu (az örnek)"; redacted: qualitative).
    pub ledger_short: String,
    /// Few cases: the ledger at birth has fewer closed rows than the product's threshold.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub few_cases: bool,
    /// Closed ledger rows at birth (absent for a redacted reader).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_n: Option<u32>,
}

/// One story of the instrument as the reader may receive it (a row of the story chart).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryRow {
    /// Capsule id.
    pub id: String,
    /// Finding id.
    pub finding: String,
    /// Bar series the bar indexes below belong to.
    pub series: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Importance tier (1 most important), as in the reader's world.
    pub tier: u8,
    /// Birth bar.
    pub born_bar: u64,
    /// Birth time (the birth bar's time).
    pub born_t: TimestampMs,
    /// Bar of the latest transition.
    pub last_bar: u64,
    /// Time of the latest transition.
    pub last_t: TimestampMs,
    /// Closing bar (absent: the story is open, its life runs to the last closed bar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_bar: Option<u64>,
    /// Closing time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_t: Option<TimestampMs>,
    /// Stage now.
    pub stage: Stage,
    /// How it ended (closed stories only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<StoryOutcome>,
    /// Whether the claim still holds as of the series' last closed bar (K42/A28; the same
    /// value as the capsule item's and the glyph's).
    pub validity: Validity,
    /// Whether the story passes the reader's feed filter (watch list, sensitivity, K53),
    /// whatever the feed's length.
    pub in_feed: bool,
    /// Why not (only when `in_feed` is false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<FeedHidden>,
    /// (K53) Its ledger is near a coin flip: the mark the chart (and the `all` sensitivity
    /// feed) shows it with.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub coin_flip: bool,
    /// The face summary (headline pieces) for the reader.
    pub face: StoryFace,
    /// The reader gets no digits for this story (K33): the face is digit-free.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub redacted: bool,
}
