//! The `query` surface's arguments: `ask` and `act` input types (the input schemas are
//! generated from them).

use super::*;

// ---------------------------------------------------------------------------------------
// Arguments (the input schemas are generated from these types).

/// What `ask` finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Find {
    /// Engine stories (capsules) you may see, most important first.
    Capsules,
    /// One instrument's story list, in birth order.
    Stories,
    /// Engine facts.
    Facts,
    /// An instrument: series, data state, last bar, capsule counts, session phase.
    Instrument,
    /// The track record (closed and realized per key).
    Ledger,
    /// Market status (open, phase, delayed, stale).
    Market,
    /// Terms used in capsule texts and what they mean.
    Glossary,
    /// What changed: log entries since a position or time.
    Changes,
    /// Validated comments.
    Comments,
    /// Your person's watches.
    Watches,
    /// Proposals.
    Proposals,
}

/// The answer shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text card per item, with ids and numbers.
    #[default]
    Card,
    /// The full protocol objects.
    Json,
    /// Compact summary per item; evidence only with `expand`.
    Layered,
}

/// A related part of a capsule or story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Expand {
    /// Transitions (bar, stage, reason, facts).
    Story,
    /// Per fact evidence (values if you are entitled, else what it requires).
    Evidence,
    /// The ledger of the capsule's key.
    Ledger,
    /// Validated comments on it.
    Comments,
}

/// One value or a list.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// One.
    One(T),
    /// Several.
    Many(Vec<T>),
}

impl<T: Clone> OneOrMany<T> {
    pub(super) fn to_vec(&self) -> Vec<T> {
        match self {
            Self::One(t) => vec![t.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

/// `where`: filters, AND-ed. Which keys apply depends on `find`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Where {
    /// An item id (capsule or finding, fact, comment, watch, proposal).
    #[serde(default)]
    pub id: Option<String>,
    /// A capsule by id or by meaning (selector).
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Story stage(s).
    #[serde(default)]
    pub stage: Option<OneOrMany<Stage>>,
    /// `true` open stories only, `false` closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Only what concerns your person (a position or an active watch).
    #[serde(default)]
    pub holdings: Option<bool>,
    /// Only the most recently born match.
    #[serde(default)]
    pub latest: Option<bool>,
    /// Your person's feed filter (watch list, sensitivity).
    #[serde(default)]
    pub audience: Option<bool>,
    /// Moved (capsules, changes) or happened (facts) at or after this bar.
    #[serde(default)]
    pub since_bar: Option<u64>,
    /// At or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<u64>,
    /// Log entries after this `seq` (an answer's `seq`).
    #[serde(default)]
    pub after_seq: Option<u64>,
    /// Log entries of the last this-many ms.
    #[serde(default)]
    pub within_ms: Option<u64>,
    /// Log entries by this principal.
    #[serde(default)]
    pub author: Option<String>,
    /// Proposal state (`pending` …) or watch state (`active`, `matched`, `expired`,
    /// `removed`).
    #[serde(default)]
    pub state: Option<String>,
    /// Exact fact kind, e.g. `breakout.range.up`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Ledger key.
    #[serde(default)]
    pub key: Option<String>,
    /// Ledger detector.
    #[serde(default)]
    pub detector: Option<String>,
    /// Ledger rows per kind.
    #[serde(default)]
    pub by_kind: Option<bool>,
    /// Bars of the instrument a story's life must meet.
    #[serde(default)]
    pub range: Option<Range>,
    /// The page after this capsule id (a stories answer's `next`).
    #[serde(default)]
    pub after: Option<String>,
}

/// `ask` arguments.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskArgs {
    /// What to find.
    pub find: Find,
    /// Filters (AND); keys per `find` in the tool description.
    #[serde(default, rename = "where")]
    pub where_: Where,
    /// Keep only these fields of each item (`id` is always kept).
    #[serde(default)]
    pub select: Vec<String>,
    /// Parts to attach to each capsule or story.
    #[serde(default)]
    pub expand: Vec<Expand>,
    /// At most this many items (default 10; facts, changes, comments 20; max 100).
    #[serde(default)]
    pub limit: Option<u32>,
    /// `card` (default), `json` or `layered`.
    #[serde(default)]
    pub format: Option<Format>,
}

/// What `act` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Do {
    /// Write a validated comment on a capsule or an instrument.
    Comment,
    /// Leave a watch for your person.
    Watch,
    /// Remove a watch.
    Unwatch,
    /// Propose view or drawing operations (a person decides).
    Propose,
    /// What a pending proposal would change (nothing is written).
    Preview,
}

/// `act` arguments.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActArgs {
    /// What to do.
    #[serde(rename = "do")]
    pub do_: Do,
    /// comment: the capsule it is about (id or selector).
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// comment: the instrument it is about (instead of `capsule`).
    #[serde(default)]
    pub instrument: Option<String>,
    /// comment, watch: the text.
    #[serde(default)]
    pub text: Option<String>,
    /// comment: language of `text` (default `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
    /// comment: ids (or capsule selectors) it rests on; `[n]` in the text cites the n-th.
    #[serde(default)]
    pub cites: Vec<CapsuleRef>,
    /// comment: named external quotes and the numbers taken from them.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
    /// watch: the person it is for (default yours).
    #[serde(default, rename = "for")]
    pub for_: Option<String>,
    /// watch: which story transitions match (every given key holds).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// watch: expiry (epoch ms).
    #[serde(default)]
    pub expires_t: Option<u64>,
    /// watch: ends after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// unwatch: the watch id.
    #[serde(default)]
    pub watch: Option<String>,
    /// propose: protocol operations on views and drawings.
    #[serde(default)]
    pub ops: Vec<Value>,
    /// propose: why (shown to the person).
    #[serde(default)]
    pub reason: Option<String>,
    /// preview: the proposal (id or selector).
    #[serde(default)]
    pub proposal: Option<ProposalRef>,
    /// preview: scene width (default 1200).
    #[serde(default)]
    pub width: Option<u32>,
    /// preview: scene height (default 600).
    #[serde(default)]
    pub height: Option<u32>,
}
