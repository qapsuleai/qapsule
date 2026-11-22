//! The task surface's arguments: one input type per tool (the input schemas are
//! generated from them).

use super::*;

/// How a capsule-giving tool answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text: ids, numbers, the sentence.
    #[default]
    Card,
    /// The protocol object, whole.
    Json,
    /// Summary first; the rest with `detail: true`.
    Layered,
}

/// `whats_new` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WhatsNewArgs {
    /// Look back to this time (epoch ms) instead of your last look.
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Only the person's feed (watch list, sensitivity; holdings and watches always).
    #[serde(default)]
    pub feed: bool,
    /// Most capsules (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `find` arguments (all filters optional, AND-ed).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Only these current stages.
    #[serde(default)]
    pub stages: Vec<Stage>,
    /// `true`: open stories only; `false`: closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Moved at or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Born at or after this time (epoch ms).
    #[serde(default)]
    pub born_since_t: Option<i64>,
    /// Only the latest-born match.
    #[serde(default)]
    pub latest: bool,
    /// Only the person's feed.
    #[serde(default)]
    pub feed: bool,
    /// Rows (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Next page: the previous answer's `next`.
    #[serde(default)]
    pub after: Option<String>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `explain` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainArgs {
    /// Capsule id, finding id, or a selector such as
    /// `{"instrument":"THYAO","meaning":"breakout","latest":true}`.
    pub capsule: CapsuleRef,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include story, evidence and comments.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `instrument` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstrumentArgs {
    /// Instrument symbol; absent: list the instruments.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Only stories alive in the last this many bars.
    #[serde(default)]
    pub last: Option<u32>,
    /// Rows (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Next page: the previous answer's `next`.
    #[serde(default)]
    pub after: Option<String>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
}

/// `track_record` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackRecordArgs {
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// One ledger key, e.g. `breakout/up/breakout.range@1`.
    #[serde(default)]
    pub key: Option<String>,
    /// Sum per meaning kind instead of per key.
    #[serde(default)]
    pub by_kind: bool,
}

/// What `watch` does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatchAction {
    /// Leave a watch.
    #[default]
    Add,
    /// Remove a watch (`intent`).
    Remove,
    /// List your watches.
    List,
}

/// `watch` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchArgs {
    /// `add` (default), `remove` or `list`.
    #[serde(default)]
    pub action: WatchAction,
    /// `add`: the request in words.
    #[serde(default)]
    pub text: Option<String>,
    /// `add`: what story transitions match (every given field must hold).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// `add`: expiry (epoch ms).
    #[serde(default)]
    pub expires_t: Option<i64>,
    /// `add`: end after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// `remove`: the watch id (`watch-…`).
    #[serde(default)]
    pub intent: Option<String>,
}

/// `comment` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentArgs {
    /// The capsule it is about (id or selector); or give `instrument`.
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// The instrument it is about (symbol).
    #[serde(default)]
    pub instrument: Option<String>,
    /// Language of `text`.
    pub lang: Language,
    /// Plain text; `[n]` cites `cites[n-1]`. Only numbers the cited items give.
    pub text: String,
    /// Ids it rests on (capsule, finding or fact; at least one).
    pub cites: Vec<String>,
    /// Named external quotes with the numbers they give.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
}

/// `propose` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposeArgs {
    /// `qp/0` view/drawing ops (applied together if accepted).
    pub ops: Vec<Value>,
    /// Why, for the person deciding.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `glossary` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlossaryArgs {
    /// Only entries whose name or text contains this (case-insensitive).
    #[serde(default)]
    pub term: Option<String>,
}
