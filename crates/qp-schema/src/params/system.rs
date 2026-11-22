//! System defaults: one `ParamDef` per defined key, and [`REGISTRY`] listing them all.
//!
//! Owner crates bind their named constants to these definitions with the typed
//! accessors (`pub const X: u32 = SCORECARD_MIN_N.default_u32();`); a test next to each
//! constant checks it still equals the old literal. Inventory: `docs/PARAMETERS.md`.

use super::{ChangedBy, ParamDef, ParamKind, ParamScope, ParamValue};

const SYSTEM: &[ParamScope] = &[ParamScope::System];
const SYSTEM_PLAN: &[ParamScope] = &[ParamScope::System, ParamScope::Plan];
const SINCE: &str = "RG1";

/// A system-scope integer parameter changed only by a release.
const fn int(
    key: &'static str,
    unit: &'static str,
    min: f64,
    default: i64,
    description: &'static str,
) -> ParamDef {
    ParamDef {
        key,
        kind: ParamKind::Int,
        unit,
        min: Some(min),
        max: None,
        default: ParamValue::Int(default),
        scopes: SYSTEM,
        changed_by: ChangedBy::ReleaseOnly,
        entitlement: false,
        description,
        since: SINCE,
    }
}

/// A system-scope float parameter changed only by a release.
const fn float(
    key: &'static str,
    unit: &'static str,
    min: f64,
    default: f64,
    description: &'static str,
) -> ParamDef {
    ParamDef {
        key,
        kind: ParamKind::Float,
        unit,
        min: Some(min),
        max: None,
        default: ParamValue::Float(default),
        scopes: SYSTEM,
        changed_by: ChangedBy::ReleaseOnly,
        entitlement: false,
        description,
        since: SINCE,
    }
}

/// `scorecard.min_n`: ledger sample size from which scorecard counts are stated plainly.
pub const SCORECARD_MIN_N: ParamDef = int(
    "scorecard.min_n",
    "count",
    1.0,
    20,
    "Ledger sample size from which the scorecard sentence states counts plainly.",
);

/// `glyph.min_ledger_n`: closed ledger rows before a glyph shows a confidence fill.
pub const GLYPH_MIN_LEDGER_N: ParamDef = int(
    "glyph.min_ledger_n",
    "count",
    1.0,
    20,
    "Closed ledger rows a story needs at birth before its glyph shows a confidence fill.",
);

/// `glyph.freshness_half_bars`: bars after the last transition at which freshness is one half.
pub const GLYPH_FRESHNESS_HALF_BARS: ParamDef = float(
    "glyph.freshness_half_bars",
    "bars",
    0.0,
    5.0,
    "Bars after the last transition at which a glyph's freshness is one half.",
);

/// `story.theme.gap_bars`: join tolerance for same kind and direction.
pub const STORY_THEME_GAP_BARS: ParamDef = int(
    "story.theme.gap_bars",
    "bars",
    0.0,
    10,
    "Join tolerance for facts of the same kind and direction.",
);

/// `story.theme.flow_confirm_bars`: window in which a flow fact confirms a price story.
pub const STORY_THEME_FLOW_CONFIRM_BARS: ParamDef = int(
    "story.theme.flow_confirm_bars",
    "bars",
    0.0,
    1,
    "Window in which a flow fact confirms a price-meaning story.",
);

/// `outcome.barrier.c_dir`: directional barrier constant (`k = c · √h`).
pub const OUTCOME_BARRIER_C_DIR: ParamDef = float(
    "outcome.barrier.c_dir",
    "ratio",
    0.0,
    0.3,
    "Directional barrier constant: k = c_dir · √h, in ATR(fast) units.",
);

/// `outcome.barrier.c_neutral`: neutral barrier constant (`k_n = c · √h`).
pub const OUTCOME_BARRIER_C_NEUTRAL: ParamDef = float(
    "outcome.barrier.c_neutral",
    "ratio",
    0.0,
    0.65,
    "Neutral barrier constant: k_n = c_neutral · √h, in ATR(slow) units.",
);

/// `outcome.atr_fast`: ATR period for directional stories.
pub const OUTCOME_ATR_FAST: ParamDef = int(
    "outcome.atr_fast",
    "bars",
    1.0,
    14,
    "ATR period for directional stories.",
);

/// `outcome.atr_slow`: ATR period for neutral stories.
pub const OUTCOME_ATR_SLOW: ParamDef = int(
    "outcome.atr_slow",
    "bars",
    1.0,
    100,
    "ATR period for neutral stories (normal volatility).",
);

/// `outcome.intraday_max_step_ms`: bar step from which a series is judged as daily.
pub const OUTCOME_INTRADAY_MAX_STEP_MS: ParamDef = int(
    "outcome.intraday_max_step_ms",
    "ms",
    1.0,
    43_200_000,
    "Bars at least this far apart are not intraday and are judged as daily.",
);

/// `watch.max_per_person`: active watches per person (an entitlement, set per plan).
pub const WATCH_MAX_PER_PERSON: ParamDef = ParamDef {
    key: "watch.max_per_person",
    kind: ParamKind::Int,
    unit: "count",
    min: Some(0.0),
    max: None,
    default: ParamValue::Int(32),
    scopes: SYSTEM_PLAN,
    changed_by: ChangedBy::Admin,
    entitlement: true,
    description: "Active watches per person at most.",
    since: SINCE,
};

/// `dispute.max_open_per_conn`: open disputes per connection.
pub const DISPUTE_MAX_OPEN_PER_CONN: ParamDef = int(
    "dispute.max_open_per_conn",
    "count",
    1.0,
    16,
    "Open disputes per connection at most.",
);

/// `dispute.max_per_conn`: disputes accepted per connection over its life.
pub const DISPUTE_MAX_PER_CONN: ParamDef = int(
    "dispute.max_per_conn",
    "count",
    1.0,
    256,
    "Disputes accepted per connection over its life at most.",
);

/// `dispute.max_digests`: different digests per connection, claim and time.
pub const DISPUTE_MAX_DIGESTS: ParamDef = int(
    "dispute.max_digests",
    "count",
    1.0,
    4,
    "Different digests per (connection, claim, t) at most.",
);

/// `gesture.story_zoom_min_bars`: fewest bars a story cluster's tap shows.
pub const GESTURE_STORY_ZOOM_MIN_BARS: ParamDef = int(
    "gesture.story_zoom_min_bars",
    "bars",
    1.0,
    20,
    "Fewest bars a tap on a story cluster shows.",
);

/// `depth.chart.lookback_bars`: bars shown before the fact bar in the chart depth.
pub const DEPTH_CHART_LOOKBACK_BARS: ParamDef = int(
    "depth.chart.lookback_bars",
    "bars",
    0.0,
    30,
    "Bars shown before the fact bar in the chart depth view.",
);

/// Every defined parameter, in document order.
pub const REGISTRY: &[&ParamDef] = &[
    &SCORECARD_MIN_N,
    &GLYPH_MIN_LEDGER_N,
    &GLYPH_FRESHNESS_HALF_BARS,
    &STORY_THEME_GAP_BARS,
    &STORY_THEME_FLOW_CONFIRM_BARS,
    &OUTCOME_BARRIER_C_DIR,
    &OUTCOME_BARRIER_C_NEUTRAL,
    &OUTCOME_ATR_FAST,
    &OUTCOME_ATR_SLOW,
    &OUTCOME_INTRADAY_MAX_STEP_MS,
    &WATCH_MAX_PER_PERSON,
    &DISPUTE_MAX_OPEN_PER_CONN,
    &DISPUTE_MAX_PER_CONN,
    &DISPUTE_MAX_DIGESTS,
    &GESTURE_STORY_ZOOM_MIN_BARS,
    &DEPTH_CHART_LOOKBACK_BARS,
];

/// The definition with this key, if any.
pub fn by_key(key: &str) -> Option<&'static ParamDef> {
    REGISTRY.iter().copied().find(|d| d.key == key)
}
