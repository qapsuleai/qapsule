//! Narration and comments carried by operations: targets, narrative steps and
//! timelines, and validated agent comments (R5B, §25.4).

use super::*;

/// A step of a narration or an attention directive points at something.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// View id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Panel id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<u32>,
    /// Layer id (R2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Series id (e.g. the series whose value matters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Output or field of the series, e.g. `"v"` (volume of a bar series), `"signal"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Document item id (finding, fact, capsule...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// First bar index (0-based) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive) of the region; equal to `from_bar` for one bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
}

/// One step of a narration timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeStep {
    /// Start of the step, milliseconds from the start of the narration.
    pub at_ms: u32,
    /// What to show.
    pub target: Target,
    /// What to say (one or two sentences).
    pub say: String,
}

/// A narration timeline about a finding: ordered steps, each pointing at something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Narrative {
    /// Unique id within the document.
    pub id: String,
    /// The item it narrates (usually a finding id).
    pub about: String,
    /// Steps, ascending by `at_ms`.
    pub steps: Vec<NarrativeStep>,
    /// Who made it and when.
    pub provenance: crate::types::Provenance,
}

/// (R5B, §25.4) What a comment is about: exactly one of a capsule (or its finding) and
/// an instrument symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentAbout {
    /// Capsule id (or its finding's id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
    /// Instrument symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
}

/// (R5B, §25.4) A validated comment, as stored in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Comment {
    /// Unique id within the document.
    pub id: String,
    /// What it is about (a capsule is stored by its capsule id).
    pub about: CommentAbout,
    /// Language of the text.
    pub lang: Language,
    /// The text.
    pub text: String,
    /// Ids it rests on.
    pub cites: Vec<String>,
    /// External quotes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external: Vec<crate::comment::ExternalQuote>,
    /// The strictness it was validated with.
    pub strictness: crate::comment::Strictness,
    /// Who wrote it (an agent: `author.kind = "agent"`) and when.
    pub provenance: crate::types::Provenance,
    /// (R5B) The person the writing agent works for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf: Option<String>,
}
