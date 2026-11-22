//! System defaults: one `ParamDef` per defined key, and [`REGISTRY`] listing them all.
//!
//! Each key's default is also a named `*_DEFAULT` constant in its owner's exact type;
//! the `ParamDef` is built from it and owner crates bind their constants to it
//! (`pub const LEDGER_MIN_N: u32 = SCORECARD_MIN_N_DEFAULT;`). A test next to each owner
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

/// Default of [`SCORECARD_MIN_N`].
pub const SCORECARD_MIN_N_DEFAULT: u32 = 20;
/// `scorecard.min_n`: ledger sample size from which scorecard counts are stated plainly.
pub const SCORECARD_MIN_N: ParamDef = int(
    "scorecard.min_n",
    "count",
    1.0,
    SCORECARD_MIN_N_DEFAULT as i64,
    "Ledger sample size from which the scorecard sentence states counts plainly.",
);

/// Default of [`GLYPH_MIN_LEDGER_N`].
pub const GLYPH_MIN_LEDGER_N_DEFAULT: u32 = 20;
/// `glyph.min_ledger_n`: closed ledger rows before a glyph shows a confidence fill.
pub const GLYPH_MIN_LEDGER_N: ParamDef = int(
    "glyph.min_ledger_n",
    "count",
    1.0,
    GLYPH_MIN_LEDGER_N_DEFAULT as i64,
    "Closed ledger rows a story needs at birth before its glyph shows a confidence fill.",
);

/// Default of [`GLYPH_FRESHNESS_HALF_BARS`].
pub const GLYPH_FRESHNESS_HALF_BARS_DEFAULT: f64 = 5.0;
/// `glyph.freshness_half_bars`: bars after the last transition at which freshness is one half.
pub const GLYPH_FRESHNESS_HALF_BARS: ParamDef = float(
    "glyph.freshness_half_bars",
    "bars",
    0.0,
    GLYPH_FRESHNESS_HALF_BARS_DEFAULT,
    "Bars after the last transition at which a glyph's freshness is one half.",
);

/// Default of [`STORY_THEME_GAP_BARS`].
pub const STORY_THEME_GAP_BARS_DEFAULT: u64 = 10;
/// `story.theme.gap_bars`: join tolerance for same kind and direction.
pub const STORY_THEME_GAP_BARS: ParamDef = int(
    "story.theme.gap_bars",
    "bars",
    0.0,
    STORY_THEME_GAP_BARS_DEFAULT as i64,
    "Join tolerance for facts of the same kind and direction.",
);

/// Default of [`STORY_THEME_FLOW_CONFIRM_BARS`].
pub const STORY_THEME_FLOW_CONFIRM_BARS_DEFAULT: u64 = 1;
/// `story.theme.flow_confirm_bars`: window in which a flow fact confirms a price story.
pub const STORY_THEME_FLOW_CONFIRM_BARS: ParamDef = int(
    "story.theme.flow_confirm_bars",
    "bars",
    0.0,
    STORY_THEME_FLOW_CONFIRM_BARS_DEFAULT as i64,
    "Window in which a flow fact confirms a price-meaning story.",
);

/// Default of [`OUTCOME_BARRIER_C_DIR`].
pub const OUTCOME_BARRIER_C_DIR_DEFAULT: f64 = 0.3;
/// `outcome.barrier.c_dir`: directional barrier constant (`k = c · √h`).
pub const OUTCOME_BARRIER_C_DIR: ParamDef = float(
    "outcome.barrier.c_dir",
    "ratio",
    0.0,
    OUTCOME_BARRIER_C_DIR_DEFAULT,
    "Directional barrier constant: k = c_dir · √h, in ATR(fast) units.",
);

/// Default of [`OUTCOME_BARRIER_C_NEUTRAL`].
pub const OUTCOME_BARRIER_C_NEUTRAL_DEFAULT: f64 = 0.65;
/// `outcome.barrier.c_neutral`: neutral barrier constant (`k_n = c · √h`).
pub const OUTCOME_BARRIER_C_NEUTRAL: ParamDef = float(
    "outcome.barrier.c_neutral",
    "ratio",
    0.0,
    OUTCOME_BARRIER_C_NEUTRAL_DEFAULT,
    "Neutral barrier constant: k_n = c_neutral · √h, in ATR(slow) units.",
);

/// Default of [`OUTCOME_ATR_FAST`].
pub const OUTCOME_ATR_FAST_DEFAULT: usize = 14;
/// `outcome.atr_fast`: ATR period for directional stories.
pub const OUTCOME_ATR_FAST: ParamDef = int(
    "outcome.atr_fast",
    "bars",
    1.0,
    OUTCOME_ATR_FAST_DEFAULT as i64,
    "ATR period for directional stories.",
);

/// Default of [`OUTCOME_ATR_SLOW`].
pub const OUTCOME_ATR_SLOW_DEFAULT: usize = 100;
/// `outcome.atr_slow`: ATR period for neutral stories.
pub const OUTCOME_ATR_SLOW: ParamDef = int(
    "outcome.atr_slow",
    "bars",
    1.0,
    OUTCOME_ATR_SLOW_DEFAULT as i64,
    "ATR period for neutral stories (normal volatility).",
);

/// Default of [`OUTCOME_INTRADAY_MAX_STEP_MS`].
pub const OUTCOME_INTRADAY_MAX_STEP_MS_DEFAULT: i64 = 43_200_000;
/// `outcome.intraday_max_step_ms`: bar step from which a series is judged as daily.
pub const OUTCOME_INTRADAY_MAX_STEP_MS: ParamDef = int(
    "outcome.intraday_max_step_ms",
    "ms",
    1.0,
    OUTCOME_INTRADAY_MAX_STEP_MS_DEFAULT,
    "Bars at least this far apart are not intraday and are judged as daily.",
);

/// Default of [`WATCH_MAX_PER_PERSON`].
pub const WATCH_MAX_PER_PERSON_DEFAULT: u32 = 32;
/// `watch.max_per_person`: active watches per person (an entitlement, set per plan).
pub const WATCH_MAX_PER_PERSON: ParamDef = ParamDef {
    key: "watch.max_per_person",
    kind: ParamKind::Int,
    unit: "count",
    min: Some(0.0),
    max: None,
    default: ParamValue::Int(WATCH_MAX_PER_PERSON_DEFAULT as i64),
    scopes: SYSTEM_PLAN,
    changed_by: ChangedBy::Admin,
    entitlement: true,
    description: "Active watches per person at most.",
    since: SINCE,
};

/// Default of [`DISPUTE_MAX_OPEN_PER_CONN`].
pub const DISPUTE_MAX_OPEN_PER_CONN_DEFAULT: usize = 16;
/// `dispute.max_open_per_conn`: open disputes per connection.
pub const DISPUTE_MAX_OPEN_PER_CONN: ParamDef = int(
    "dispute.max_open_per_conn",
    "count",
    1.0,
    DISPUTE_MAX_OPEN_PER_CONN_DEFAULT as i64,
    "Open disputes per connection at most.",
);

/// Default of [`DISPUTE_MAX_PER_CONN`].
pub const DISPUTE_MAX_PER_CONN_DEFAULT: usize = 256;
/// `dispute.max_per_conn`: disputes accepted per connection over its life.
pub const DISPUTE_MAX_PER_CONN: ParamDef = int(
    "dispute.max_per_conn",
    "count",
    1.0,
    DISPUTE_MAX_PER_CONN_DEFAULT as i64,
    "Disputes accepted per connection over its life at most.",
);

/// Default of [`DISPUTE_MAX_DIGESTS`].
pub const DISPUTE_MAX_DIGESTS_DEFAULT: usize = 4;
/// `dispute.max_digests`: different digests per connection, claim and time.
pub const DISPUTE_MAX_DIGESTS: ParamDef = int(
    "dispute.max_digests",
    "count",
    1.0,
    DISPUTE_MAX_DIGESTS_DEFAULT as i64,
    "Different digests per (connection, claim, t) at most.",
);

/// Default of [`GESTURE_STORY_ZOOM_MIN_BARS`].
pub const GESTURE_STORY_ZOOM_MIN_BARS_DEFAULT: u64 = 20;
/// `gesture.story_zoom_min_bars`: fewest bars a story cluster's tap shows.
pub const GESTURE_STORY_ZOOM_MIN_BARS: ParamDef = int(
    "gesture.story_zoom_min_bars",
    "bars",
    1.0,
    GESTURE_STORY_ZOOM_MIN_BARS_DEFAULT as i64,
    "Fewest bars a tap on a story cluster shows.",
);

/// Default of [`DEPTH_CHART_LOOKBACK_BARS`].
pub const DEPTH_CHART_LOOKBACK_BARS_DEFAULT: u64 = 30;
/// `depth.chart.lookback_bars`: bars shown before the fact bar in the chart depth.
pub const DEPTH_CHART_LOOKBACK_BARS: ParamDef = int(
    "depth.chart.lookback_bars",
    "bars",
    0.0,
    DEPTH_CHART_LOOKBACK_BARS_DEFAULT as i64,
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
