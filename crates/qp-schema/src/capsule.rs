//! Capsule types (R3): meaning of a fact, the capsule story, the ledger statistic,
//! sensitivity and the meaning glyph's parameters. Concepts in `docs/CONCEPTS.md`
//! ("Kapsül (R3)"); framework in `docs/CAPSULE.md`.
//!
//! Every new field on an existing type is optional and skipped when absent, so documents
//! and transcripts written before R3 serialize byte-identically.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::TimestampMs;

/// The six meaning kinds (K30). The glyph's shape encodes this.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MeaningKind {
    /// Devam: the move goes on (trend, MA alignment, flag).
    Continuation,
    /// Dönüş: the move may be turning (divergence, double top/bottom).
    Reversal,
    /// Kırılım: a boundary was crossed (range, channel, support/resistance).
    Breakout,
    /// Sıkışma: the move is narrowing; direction unknown (band squeeze, low volatility).
    Squeeze,
    /// Aşırılık: unusual stretch (overbought/oversold, outside the band).
    Extreme,
    /// Akış: money in/out independent of price (volume, settlement, institutional flow).
    Flow,
}

impl MeaningKind {
    /// All kinds, in canonical order.
    pub const ALL: [Self; 6] = [
        Self::Continuation,
        Self::Reversal,
        Self::Breakout,
        Self::Squeeze,
        Self::Extreme,
        Self::Flow,
    ];

    /// Snake-case name (`breakout`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Continuation => "continuation",
            Self::Reversal => "reversal",
            Self::Breakout => "breakout",
            Self::Squeeze => "squeeze",
            Self::Extreme => "extreme",
            Self::Flow => "flow",
        }
    }
}

/// Direction of a meaning.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Upwards.
    Up,
    /// Downwards.
    Down,
    /// No direction (e.g. a squeeze).
    Neutral,
}

/// Evidence family (K30). Several facts from one family count as one confirmation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFamily {
    /// Price and indicators computed from price.
    Price,
    /// Traded volume.
    Volume,
    /// Money flow: settlement, institutional (broker) flow.
    Flow,
    /// Derivatives (futures, options).
    Derivative,
    /// News and fundamentals.
    News,
}

/// License class of the data a fact rests on (K33).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct License {
    /// `public`: the user's own data entitlement for the bar series is enough to see the
    /// values (the replica can recompute it). `licensed`: rests on data only the server
    /// may read; computed only on the server (server fact) and shown to an unentitled
    /// user as a comment without numbers.
    pub class: LicenseClass,
    /// The data entitlement needed to see the evidence values (`flow/THYAO`, `flow/*`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entitlement: Option<String>,
}

/// See [`License::class`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LicenseClass {
    /// Recomputable from the user's own bar data.
    Public,
    /// Server-only data; numbers never reach an unentitled user.
    Licensed,
}

/// The three time kinds of a capsule (K27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimeKind {
    /// Olay: happened at a bar; past tense with its time; freshness fades.
    Event,
    /// Durum: holds while the condition holds; present tense; ends when it breaks.
    State,
    /// Beklenti: conditional; lives until the condition is met or voided.
    Expectation,
}

/// Evidence geometry: what the chart depth draws for a fact. Values are never stored
/// here; they are keys into [`crate::Fact::evidence`], so redacting the evidence removes
/// the geometry's numbers too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "shape", rename_all = "snake_case", deny_unknown_fields)]
pub enum Geometry {
    /// A horizontal level from one bar to another (a broken resistance, a threshold).
    Level {
        /// Evidence key of the level value.
        key: String,
        /// First bar of the segment.
        from_bar: u64,
        /// Last bar of the segment.
        to_bar: u64,
    },
    /// A band between two evidence values over a bar range (a squeeze's band).
    Band {
        /// Evidence key of the upper value.
        upper: String,
        /// Evidence key of the lower value.
        lower: String,
        /// First bar.
        from_bar: u64,
        /// Last bar.
        to_bar: u64,
    },
    /// Marked points (swing peaks of a divergence): one bar and one evidence key each.
    Points {
        /// `(bar, evidence key)` pairs, in bar order.
        points: Vec<GeoPoint>,
    },
}

/// One marked point of [`Geometry::Points`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoPoint {
    /// Bar index.
    pub bar: u64,
    /// Evidence key of the value at that bar.
    pub key: String,
}

/// The meaning of a fact (R3): what a detector found, in the six-kind vocabulary.
/// Optional on [`crate::Fact`]; facts written before R3 have none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactMeaning {
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Evidence family.
    pub family: EvidenceFamily,
    /// Time kind.
    pub time: TimeKind,
    /// Edge: a point event, or a state's start or end.
    #[serde(default, skip_serializing_if = "Edge::is_point")]
    pub edge: Edge,
    /// Detector id, e.g. `breakout.donchian`.
    pub detector: String,
    /// Detector version; part of the ledger key.
    pub detector_version: u32,
    /// Instrument symbol.
    pub instrument: String,
    /// Strength in `[0, 1]` (detector-defined, deterministic).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub strength: f64,
    /// License class of the data.
    pub license: License,
    /// Geometry drawn at chart depth.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometry: Vec<Geometry>,
    /// Machine-checkable condition of an expectation (`time = expectation`) or of a
    /// state's continuation (`time = state`): a sentence slot, not free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<MeaningCondition>,
    /// `true` when produced on the server from licensed data (server fact, K33).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub server: bool,
}

/// Whether a fact is a point event, or the start or end of a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    /// A point event (a crossing, a breakout).
    #[default]
    Point,
    /// A state started (RSI went above 70 and stays there while the state holds).
    Enter,
    /// A state ended.
    Exit,
}

impl Edge {
    /// `true` for [`Edge::Point`] (serde skip helper).
    #[must_use]
    pub fn is_point(&self) -> bool {
        matches!(self, Self::Point)
    }
}

/// A machine-checkable condition attached to a meaning (sentence slot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeaningCondition {
    /// What is compared: `close`, `rsi`, `bandwidth`, … (detector-defined name).
    pub subject: String,
    /// Comparison.
    pub op: crate::CompareOp,
    /// Evidence key of the threshold value.
    pub key: String,
    /// Within how many bars (expectation), if bounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_bars: Option<u32>,
}

/// Lifecycle stage of a capsule story (K28, K37):
/// `candidate → born → strengthened | weakened → confirmed → realized | invalidated |
/// expired | retracted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Aday: seen on an open bar; only on the watching replica; never in the authority's
    /// log or the ledger.
    Candidate,
    /// Doğdu: finalized on bar close; the authority gave it an id.
    Born,
    /// Güçlendi: a new supporting fact of the same family.
    Strengthened,
    /// Zayıfladı: a fact against it, or its state weakened.
    Weakened,
    /// Teyit edildi: a second (or further) independent evidence family supports it.
    Confirmed,
    /// Gerçekleşti: the outcome rule says it came true. Closes; ledger hit.
    Realized,
    /// Geçersiz: its invalidation happened. Closes; ledger miss.
    Invalidated,
    /// Süresi doldu: its natural life (in bars) ended without an outcome. Closes; ledger miss.
    Expired,
    /// Geri çekildi: the data it rested on proved wrong (K37). Closes; NOT in the ledger.
    Retracted,
}

impl Stage {
    /// Whether the stage closes the story.
    #[must_use]
    pub fn is_closed(self) -> bool {
        matches!(
            self,
            Self::Realized | Self::Invalidated | Self::Expired | Self::Retracted
        )
    }
}

/// One transition in a capsule's story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    /// Stage entered.
    pub stage: Stage,
    /// Bar index (of the story's bar series) at which it happened.
    pub bar: u64,
    /// Bar time.
    pub t: TimestampMs,
    /// Facts that caused it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// Machine-readable reason (`new_family`, `same_family`, `opposite_fact`,
    /// `state_ended`, `outcome_hit`, `outcome_miss`, `horizon`, `data_corrected`,
    /// `dispute_upheld`, …).
    pub reason: String,
}

/// Ledger statistic for one key (meaning kind + detector), as seen at some time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerStat {
    /// Closed capsules counted (realized + invalidated + expired; never retracted).
    pub n: u32,
    /// Of which realized.
    pub realized: u32,
}

/// How a capsule reached the user; drives the "verified by" mark (K38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    /// The replica can re-check it on its own data (consistency rule, §19).
    Replica,
    /// Server fact: the replica cannot re-check it (licensed data); shown as verified
    /// by the server.
    Server,
}

/// The story a capsule tells (R3). Optional on [`crate::Capsule`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Story {
    /// Instrument symbol.
    pub instrument: String,
    /// Theme key: same instrument + same theme → one story (K28).
    pub theme: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Time kind.
    pub time: TimeKind,
    /// Current stage.
    pub stage: Stage,
    /// Independent evidence families that support it (sorted, unique).
    pub families: Vec<EvidenceFamily>,
    /// Strength in `[0, 1]`.
    pub strength: f64,
    /// Importance score from the ranking rule (higher = more important).
    pub importance: f64,
    /// Importance tier: 1 most important … 3 least (sensitivity filters on it).
    pub tier: u8,
    /// The bar series the story lives on.
    pub series: String,
    /// Bar the story was born on.
    pub born_bar: u64,
    /// Bar of the last transition.
    pub last_bar: u64,
    /// Natural life in bars from birth (expiry by bar count, never by clock).
    pub horizon_bars: u32,
    /// Ledger key (`kind/detector@version`).
    pub ledger_key: String,
    /// Ledger at birth: only capsules closed before the birth bar (no look-ahead).
    pub ledger_at_birth: LedgerStat,
    /// Trust path.
    pub trust: Trust,
    /// Transitions, in order.
    pub history: Vec<Transition>,
}

/// Sensitivity level (K31): the attention budget a user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    /// Yalnız önemliler: tier 1 only.
    Important,
    /// Dengeli: tiers 1–2.
    Balanced,
    /// Hepsi: every capsule.
    All,
}

/// User group, for the default sensitivity (K31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserGroup {
    /// Alışkın kullanıcı: default `all` (list shows everything), push kept low.
    Habituated,
    /// Geleceğin yatırımcısı: default `important`.
    Future,
}

impl UserGroup {
    /// The default sensitivity of the group.
    #[must_use]
    pub fn default_sensitivity(self) -> Sensitivity {
        match self {
            Self::Habituated => Sensitivity::All,
            Self::Future => Sensitivity::Important,
        }
    }
}

/// A person's capsule preferences (simple fields in the document, R3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Audience {
    /// User group.
    pub group: UserGroup,
    /// Chosen sensitivity; absent = the group's default. The system never changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<Sensitivity>,
    /// Watch list (symbols).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watchlist: Vec<String>,
    /// Positions: symbol → signed quantity (sign = long/short).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub positions: BTreeMap<String, f64>,
}

impl Audience {
    /// The sensitivity in effect.
    #[must_use]
    pub fn effective(&self) -> Sensitivity {
        self.sensitivity
            .unwrap_or_else(|| self.group.default_sensitivity())
    }
}

/// Phase of a glyph's line style (K30: dashed candidate, solid final, outcome).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GlyphPhase {
    /// Candidate (open bar).
    Candidate,
    /// Final, still open.
    Final,
    /// Closed: realized.
    Realized,
    /// Closed: not realized (invalidated or expired).
    Failed,
    /// Closed: retracted (data was wrong).
    Retracted,
}

impl From<Stage> for GlyphPhase {
    fn from(s: Stage) -> Self {
        match s {
            Stage::Candidate => Self::Candidate,
            Stage::Born | Stage::Strengthened | Stage::Weakened | Stage::Confirmed => Self::Final,
            Stage::Realized => Self::Realized,
            Stage::Invalidated | Stage::Expired => Self::Failed,
            Stage::Retracted => Self::Retracted,
        }
    }
}

/// Parameters of one meaning glyph (K30 visual channels). All engine-derived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlyphSpec {
    /// Shape: the kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Size/weight: strength in `[0, 1]`.
    pub strength: f64,
    /// Fill: confidence from the ledger in `[0, 1]`; absent when the ledger is too small.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Line style: phase.
    pub phase: GlyphPhase,
    /// Opacity: freshness in `[0, 1]` (1 = this bar).
    pub freshness: f64,
    /// Confirmation marks: number of independent evidence families (1–5).
    pub confirmations: u8,
    /// Server-verified mark (K38).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub server: bool,
}
