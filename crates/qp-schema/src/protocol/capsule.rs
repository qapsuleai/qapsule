//! Capsule answers for a reader (R3, §20) and evidence re-checks.

use super::*;

/// Both sentences of a capsule for one reader (R3, §20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleSentence {
    /// The sentence in the reader's language.
    pub text: String,
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
}

/// One capsule as listed for a reader (R3, §20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleItem {
    /// Capsule id.
    pub id: String,
    /// Finding id.
    pub finding: String,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series.
    pub series: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Time kind.
    pub time: TimeKind,
    /// Story stage.
    pub stage: Stage,
    /// Capsule state.
    pub state: CapsuleState,
    /// Importance tier (1 most important).
    pub tier: u8,
    /// Importance score.
    pub importance: f64,
    /// Independent evidence families supporting it.
    pub confirmations: u8,
    /// Trust path.
    pub trust: Trust,
    /// Birth bar.
    pub born_bar: u64,
    /// Bar of the latest transition.
    pub last_bar: u64,
    /// Time of the latest transition.
    pub t: TimestampMs,
    /// Ledger of its key at birth (no look-ahead).
    pub ledger_at_birth: LedgerStat,
    /// The sentence as of the series' last closed bar, for this reader (K26).
    pub sentence: CapsuleSentence,
    /// (R4, K42/A28) Whether the claim still holds as of the same bar: `valid`,
    /// `no_longer` (the state condition broke, the story is still open) or `closed`.
    /// The sentence and the glyph say the same.
    pub validity: Validity,
    /// About the reader's position or active intent. (R4, K42/A22) Its **negative
    /// transitions** (`weakened`, `invalidated`, `retracted`) pass any sensitivity; its
    /// birth and other transitions go through the sensitivity filter like any story.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub relevant: bool,
    /// The sentence carries no digits for this reader (K33).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub redacted: bool,
    /// (R5/M19, K53) Its ledger at birth is near a coin flip by the host's feed rule: in
    /// the feed only at the `all` sensitivity, shown with this mark (`docs/PROTOCOL.md`
    /// §23.2). Never set without a host rule.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub coin_flip: bool,
    /// (R5/m17-int, `docs/LANGUAGE.md`) The capsule's four surfaces in the voice the
    /// connection chose (`style` `voice`), in the reader's language and by the same
    /// audience rule (redacted: no digits, K33). Absent under the default voice `r4`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<CapsuleVoice>,
}

/// (R5/m17-int, `docs/LANGUAGE.md`) A capsule's surfaces in one voice and language: what
/// the face, the open capsule and the glossary show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleVoice {
    /// Voice name (`marla`, `selvi`).
    pub voice: String,
    /// Başlık: what changed, with the threshold, when and whether it still holds.
    pub headline: String,
    /// Ayrıntı: why, and where things stand now (built from the face's pieces, A33).
    pub detail: String,
    /// Karne: the expectation and how often it played out before.
    pub ledger: String,
    /// Sözlük: the terms on this capsule's screens, each opened on touch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glossary: Vec<GlossEntry>,
    /// The text face of a reader without a face scene (A32): headline, ledger and detail,
    /// one per line.
    pub face: String,
}

/// (R5/m17-int) One glossary entry: a term and its short explanation (no numerals).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlossEntry {
    /// The term as the screen writes it.
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// One fact's evidence as a reader may open it (R3, §20; K34).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAccess {
    /// Fact id.
    pub fact: String,
    /// Evidence family.
    pub family: EvidenceFamily,
    /// License class of the data.
    pub license: LicenseClass,
    /// The reader may see the values.
    pub open: bool,
    /// The evidence values (only when `open`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f64>,
    /// The frozen evidence (only when `open`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<EvidenceRef>,
    /// The data entitlement the values require (when not `open`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,
    /// What the evidence rests on, in words (always).
    pub rests_on: String,
}

/// A capsule opened to its evidence (R3, §20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleDetail {
    /// The capsule as listed.
    pub item: CapsuleItem,
    /// The story with every transition (bar, time, stage, reason, facts).
    pub story: Story,
    /// The facts as the reader may receive them.
    pub facts: Vec<Fact>,
    /// The ledger of its key now (closes before the series' last closed bar).
    pub ledger: LedgerStat,
    /// The glyph's parameters (engine `glyph_spec` at the last closed bar).
    pub glyph: GlyphSpec,
    /// Per fact: the evidence values or what they require (K34).
    pub evidence: Vec<EvidenceAccess>,
    /// (R4/S1, §20.2) Only when `capsule` was a selector, the reader may not read the
    /// licensed flow of the instrument and the authority already told this reader of
    /// licensed activity newer than the resolved capsule's latest transition: the
    /// entitlement those stories require and the server comments that told it (the same
    /// comments the `capsules` query gives). Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withheld: Option<Withheld>,
    /// (R5B, §25.4) Validated comments about this capsule the reader may receive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
}

/// (R4/S1, §20.2) What a capsule selector may also have matched that the reader cannot
/// see: newer licensed stories, told only by the server comments the reader already gets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Withheld {
    /// Entitlements they require (the comments' `entitlement`, e.g. `flow/*`).
    pub requires: Vec<String>,
    /// The server comments newer than the resolved capsule's latest transition.
    pub comments: Vec<ServerComment>,
}

/// One ledger row (R3, §20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerRow {
    /// Ledger key (`kind/direction/detector@version`), or the kind when grouped by kind.
    pub key: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// (R4, K42/A23) Direction (absent when grouped by kind without a direction filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// Detector (absent when grouped by kind).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector: Option<String>,
    /// Closed capsules (realized + invalidated + expired; never retracted).
    pub n: u32,
    /// Of which realized.
    pub realized: u32,
}

/// A server-verified, digit-free comment on licensed data for a reader without the
/// entitlement (R3, §20; the chosen trust path mühür, M6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServerComment {
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series it belongs to.
    pub series: String,
    /// Bar index.
    pub bar: u64,
    /// Bar time.
    pub t: TimestampMs,
    /// Machine codes of what is said.
    pub codes: Vec<String>,
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
    /// Always `server` (K38).
    pub trust: Trust,
    /// What it requires and rests on, the rule and the receipt (no digits).
    pub attachments: BTreeMap<String, String>,
}

/// Outcome of re-checking one fact's frozen evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    /// Same bars (digest equal) and same evidence values.
    Verified,
    /// The data or the recomputed evidence values differ.
    Changed,
    /// The fact has no evidence reference (written before RP2, or on a points series).
    NoRef,
    /// The referenced data is not available now (binding not ready, range outside).
    Unavailable,
    /// The asker may not read the referenced feed.
    NoEntitlement,
}

/// An evidence value then and now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueCheck {
    /// Value stored in the fact.
    pub then: f64,
    /// Value recomputed today; `null` if not computable.
    pub now: Option<f64>,
}

/// One fact's evidence check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactCheck {
    /// Fact id.
    pub fact: String,
    /// Outcome.
    pub status: EvidenceStatus,
    /// What happened, in words (first difference when changed).
    pub message: String,
    /// The stored reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored: Option<EvidenceRef>,
    /// The same range over today's data (bars and digest), when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<EvidenceRef>,
    /// Evidence values (`value`, `prev`…) stored vs recomputed, for a fact on a study.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, ValueCheck>,
}

/// Answer to `verify_evidence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCheck {
    /// The fact or finding checked.
    pub item: String,
    /// True iff every fact is `verified`.
    pub verified: bool,
    /// One check per fact, in the finding's fact order.
    pub facts: Vec<FactCheck>,
}
