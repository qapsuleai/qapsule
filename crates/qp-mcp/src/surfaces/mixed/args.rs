//! The mixed surface's arguments: one input type per tool (the input schemas are
//! generated from them).

use super::*;

/// How a capsule is given (M29).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text card: ids, sentence, counts, evidence values (default).
    #[default]
    Card,
    /// The full protocol object(s).
    Json,
    /// A JSON summary; with `detail: true` also the story, evidence and comments.
    Layered,
}

/// `overview` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverviewArgs {
    /// Only what moved at or after this time (epoch ms; pass the `now` of your previous
    /// overview). Default: everything the person may see.
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Capsules to give (default 8, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Capsule format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: the full capsule items instead of summaries.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// `find` arguments (filters combine with AND).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// `true`: open stories only; `false`: closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Only stories now at these stages.
    #[serde(default)]
    pub stages: Vec<Stage>,
    /// Only stories that moved at or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Only stories born at or after this time (epoch ms).
    #[serde(default)]
    pub born_since_t: Option<i64>,
    /// Only the most recently born match.
    #[serde(default)]
    pub latest: bool,
    /// Apply the person's audience (watch list, positions, sensitivity).
    #[serde(default)]
    pub audience: bool,
    /// At most this many (default 8, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Capsule format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: the full capsule items instead of summaries.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// `capsule` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleArgs {
    /// Capsule id, its finding id, or a selector, e.g.
    /// `{"instrument":"THYAO","meaning":"breakout","latest":true}`.
    #[schemars(with = "CapsuleRef")]
    pub capsule: Value,
    /// Format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: add the story, the evidence values and the comments.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// What `watch` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatchAction {
    /// Leave a watch (`text`, `match`; optional `expires_t`, `max_matches`).
    Add,
    /// Remove a watch (`intent`).
    Remove,
    /// The person's watches.
    List,
    /// Be notified of capsule transitions (optional `instrument`).
    Follow,
    /// Stop a follow (`sub`).
    Unfollow,
}

/// `watch` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchArgs {
    /// What to do.
    pub action: WatchAction,
    /// `add`: the request in words.
    #[serde(default)]
    pub text: Option<String>,
    /// `add`: which story transitions match (every given field must hold).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// `add`: expiry time (epoch ms).
    #[serde(default)]
    pub expires_t: Option<i64>,
    /// `add`: the watch ends after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// `remove`: the watch's intent id (`watch-…`).
    #[serde(default)]
    pub intent: Option<String>,
    /// `follow`: only this instrument.
    #[serde(default)]
    pub instrument: Option<String>,
    /// `unfollow`: the subscription id `follow` returned.
    #[serde(default)]
    pub sub: Option<String>,
}

/// What `write` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteKind {
    /// A comment, validated by the engine before it is published.
    Comment,
    /// A proposal of view operations; a person accepts or rejects it.
    Proposal,
}

/// `write` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteArgs {
    /// `comment` or `proposal`.
    pub kind: WriteKind,
    /// comment: what it is about, `{"capsule": id}` or `{"instrument": symbol}`.
    #[serde(default)]
    pub about: Option<CommentAbout>,
    /// comment: language of `text`.
    #[serde(default)]
    pub lang: Option<Language>,
    /// comment: the text; `[n]` marks the n-th entry of `cites`.
    #[serde(default)]
    pub text: Option<String>,
    /// comment: ids of the capsules, findings or facts it rests on (at least one).
    #[serde(default)]
    pub cites: Vec<String>,
    /// comment: external sources quoted, with the numbers taken from them.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
    /// proposal: the operations (applied together if a person accepts).
    #[serde(default)]
    #[schemars(with = "Vec<Op>")]
    pub ops: Vec<Value>,
    /// proposal: why, in a sentence.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `query` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryArgs {
    /// A qp/0 query with its `kind`, e.g. `{"kind":"stories","instrument":"THYAO"}`,
    /// `{"kind":"ledger","meaning":"breakout"}`, `{"kind":"describe"}`.
    #[schemars(with = "Query")]
    pub query: Value,
}
