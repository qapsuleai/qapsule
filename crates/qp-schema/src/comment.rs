//! (R5B) Agent comments: the draft an agent submits and the machine-readable reasons a
//! draft is rejected (`rounds/R5B/PLAN.md` §2.1, GOAL principle 2).
//!
//! The validation logic lives in `qp_capsule::comment`; these are only the data types so
//! the protocol (`comment_rejected` error, `Comment`) can carry them.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::protocol::Language;

/// How strict the comment validator is (M30 a/b/c).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strictness {
    /// (a) Citations exist and are visible; advice and licence checks.
    Cites,
    /// (b) (a) + every number in the text is in the cited evidence or a named external
    /// source.
    CitesNumbers,
    /// (c) (b) + every sentence that carries a number carries a citation marker `[n]`.
    PerSentence,
}

/// A quotation from an external (named) source, e.g. a news item or a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalQuote {
    /// Who said it (e.g. `"matriks:news/123"`); must not be empty.
    pub source: String,
    /// The quoted text.
    pub quote: String,
    /// The numbers taken from the quote; each must appear in `quote`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<f64>,
}

/// A comment as an agent submits it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentDraft {
    /// Language of `text` (decides how ambiguous numbers such as `1.234` are read).
    pub lang: Language,
    /// The comment. `[n]` marks a citation of `cites[n-1]`.
    pub text: String,
    /// Cited fact / capsule / finding ids.
    pub cites: Vec<String>,
    /// External source quotations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external: Vec<ExternalQuote>,
}

/// Which check rejected a draft.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// Citations (missing, unknown id, bad marker, external without source).
    Cites,
    /// A number not found in the cited evidence or a named external source.
    Number,
    /// Advice language.
    Advice,
    /// The reader's licence does not cover an id or a number.
    License,
    /// A sentence with a number but no citation marker (`PerSentence`).
    Sentence,
    /// The text states only the direction opposite to every cited directional item.
    Direction,
}

/// One machine-readable reason a draft was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reason {
    /// The check.
    pub check: Check,
    /// Stable code (e.g. `unknown_id`, `unsupported_number`, `advice_word`,
    /// `licensed_value`, `uncited_sentence`).
    pub code: String,
    /// The offending token (id, number as written, word), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 0-based index of the sentence it occurs in, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sentence: Option<usize>,
    /// A short English sentence telling the agent how to fix it.
    pub hint: String,
}
