//! Queries: collections, ranges, reductions and the `Query` message.

use super::*;

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Item collections of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Collection {
    /// Series.
    Series,
    /// Views.
    Views,
    /// Facts.
    Facts,
    /// Findings.
    Findings,
    /// Capsules.
    Capsules,
    /// Intents.
    Intents,
    /// Narratives.
    Narratives,
    /// (R5B) Comments.
    Comments,
    /// Proposals.
    Proposals,
}

/// A bar range. Exactly one form: `last` (the last n bars) or `from_bar`/`to_bar`
/// (0-based, inclusive) or `from_t`/`to_t` (epoch ms, inclusive).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Range {
    /// The last n bars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<u64>,
    /// First bar index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
    /// From time (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// To time (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
}

/// Reduction applied over a range (warmup `null`s are skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reduce {
    /// Smallest value (and its bar).
    Min,
    /// Largest value (and its bar).
    Max,
    /// Arithmetic mean.
    Mean,
    /// First non-null value.
    First,
    /// Last non-null value.
    Last,
    /// Number of non-null values.
    Count,
}

/// A read request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    /// What this protocol endpoint offers: operations, queries, topics, principals,
    /// and the asker's own role and permissions.
    Describe {},
    /// One item by id, from any collection.
    Get {
        /// Item id.
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        id: String,
    },
    /// All items of a collection. For proposals, `state` filters (e.g. `pending`).
    List {
        /// Collection.
        collection: Collection,
        /// Proposal state filter (proposals only).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ProposalState>,
        /// Include bulky data (the bars of bar series). Default false: series are listed
        /// without `data`; read values with the `study` or `at` query, or `get` one item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<bool>,
    },
    /// The whole document and its log position (a snapshot, e.g. for hand-over).
    Document {},
    /// Log entries. Filters combine with AND.
    Log {
        /// Entries with `seq` greater than this.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after_seq: Option<Seq>,
        /// Entries at or after this time (epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_t: Option<TimestampMs>,
        /// Entries at or before this time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until_t: Option<TimestampMs>,
        /// Entries from the last this-many milliseconds (relative to `now`), e.g.
        /// `1800000` for "the last 30 minutes".
        #[serde(default, skip_serializing_if = "Option::is_none")]
        within_ms: Option<u64>,
        /// Only entries by this principal.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        /// Return at most this many (the latest ones).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    /// Values of a series over a range, aligned with its bars (K13/K20), optionally
    /// reduced to one number. For a bar series, outputs are `o`,`h`,`l`,`c`,`v`.
    Study {
        /// Series id (a study series or a bar series).
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
        series: String,
        /// Range; the whole series if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        range: Option<Range>,
        /// Only this output (e.g. `"signal"` of MACD); all if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        /// Reduce to one value per output.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reduce: Option<Reduce>,
    },
    /// Everything at one bar: OHLCV and the value of every study on that bar series.
    At {
        /// Bar series id.
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
        series: String,
        /// Bar index (0-based). Give `bar` or `t`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bar: Option<u64>,
        /// Bar time (epoch ms); the bar with exactly this `t`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t: Option<TimestampMs>,
    },
    /// What a principal's surfaces show right now, semantically (views, panels, series,
    /// visible bar range, capsules). The asker's own if `principal` is absent.
    Screen {
        /// Principal id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
    },
    /// Point query (R2): what is drawn at pixel (`x`, `y`) of a view rendered at
    /// `width` x `height`. Answered from the scene description, without rendering. The
    /// visible range is the asker's viewport of that view if it has one, otherwise the
    /// view's persistent range, otherwise all bars.
    Hit {
        /// View id; the asker's current view if absent.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<ViewRef>")]
        view: Option<String>,
        /// Pixel x (0 = left edge).
        x: f64,
        /// Pixel y (0 = top edge).
        y: f64,
        /// Frame width in pixels (default 1200).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height in pixels (default 600).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// Where a principal is looking (last viewport and focus) and the attention
    /// directives addressed to them.
    Attention {
        /// Principal id.
        principal: String,
    },
    /// A finding presented for a surface. Depth follows the asker's negotiated profile
    /// unless `surface` names another surface kind.
    Present {
        /// Finding id. If absent: the only live finding; with several live findings the
        /// authority does not pick one (`ambiguous`, every live finding as a candidate;
        /// K25). Also takes a selector (§17), e.g. `{"latest":true}`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<FindingRef>")]
        finding: Option<String>,
        /// Present for this surface kind instead of the asker's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        surface: Option<SurfaceKind>,
    },
    /// (RP2, §18.3) Data bindings and their state for the asker: resolved or not, why
    /// not, the anchor (first bar time) and the data position. All bound series if
    /// `series` is absent.
    Binding {
        /// A bound series.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<SeriesRef>")]
        series: Option<String>,
    },
    /// (RP2, §18.5) Re-checks the frozen evidence of a fact, or of every fact of a
    /// finding, against today's data: content digest of the referenced bar range and the
    /// fact's evidence values recomputed.
    VerifyEvidence {
        /// A fact or a finding.
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        item: String,
    },
    /// (R3, §20) The capsules (stories) the asker may see, most important first
    /// (importance descending, then latest transition, then id). Filters combine with
    /// AND. Sentences are written for the asker (its language and data entitlement).
    Capsules {
        /// Only this instrument (symbol).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instrument: Option<String>,
        /// Only these stages.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stages: Vec<Stage>,
        /// `true`: only open stories; `false`: only closed ones.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        open: Option<bool>,
        /// Only this meaning kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meaning: Option<MeaningKind>,
        /// Only stories with a transition at or after this bar.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_bar: Option<u64>,
        /// Only stories with a transition at or after this time (epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_t: Option<TimestampMs>,
        /// (R4) Only stories born at or after this bar (of their own bar series).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        born_since_bar: Option<u64>,
        /// (R4) Only stories born at or after this time (epoch ms; the birth bar's time).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        born_since_t: Option<TimestampMs>,
        /// (R4) Only the most recently born of the matching stories ("the latest
        /// capsule"): the greatest birth time, then the greatest birth bar, then the
        /// greatest id. Applied after every other filter, before `limit`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        latest: bool,
        /// Apply the asker's audience: watch list (and positions, intents) and
        /// sensitivity, with the position/intent override (§9.8). Default false: every
        /// story the asker may see.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        audience: bool,
        /// Return at most this many.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    /// (R3, §20) One capsule opened to its evidence (K34): story history with
    /// transitions and reasons, facts, sentences, ledger, glyph and, per fact, the
    /// evidence values if the asker is entitled, otherwise the entitlement it requires
    /// and what it rests on.
    Capsule {
        /// Capsule id, or its finding's id.
        /// (R4) Also takes a capsule selector (§17, [`CapsuleSel`]).
        #[schemars(with = "CapsuleRef")]
        capsule: String,
    },
    /// (R3, §20) The ledger: closed capsules and realized ones per ledger key
    /// (`kind/direction/detector@version`, R4), or summed per kind. Retracted capsules
    /// never count.
    /// The ledger is license-independent (K33): every principal that may read gets it.
    Ledger {
        /// Only this ledger key.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        /// Only this meaning kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meaning: Option<MeaningKind>,
        /// Only this detector.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detector: Option<String>,
        /// (R4, K42/A23) Only this direction.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direction: Option<Direction>,
        /// Group rows per kind instead of per key (every direction summed; with
        /// `direction`, that direction only).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        by_kind: bool,
    },
    /// (R5/M18, §23, K50) The instrument's story list: every story (capsule) of one
    /// instrument whose life (birth to closing, or to the last closed bar when open)
    /// meets `range`, as the reader may receive it — no bars (K23). Ordered by birth
    /// (time, bar, id), paged with `limit` and `after`.
    Stories {
        /// Instrument symbol.
        instrument: String,
        /// Its bar series (needed only when the instrument has more than one).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        series: Option<String>,
        /// Bars or times of that series the story's life must meet (default: all).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        range: Option<Range>,
        /// Rows per answer (default 200, at most 500).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        /// The page after this capsule id (the previous answer's `next`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
    },
    /// (R5B, §25.6) What a pending proposal would do, without doing it: its operations
    /// expanded on a copy of the document as the asker reads it, the changes, and a scene
    /// summary of every view it would add or change (the document does not change).
    Preview {
        /// Proposal id.
        /// Also takes a selector (§17).
        #[schemars(with = "ProposalRef")]
        proposal: String,
        /// Scene width in pixels (default 1200).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Scene height in pixels (default 600).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
}
