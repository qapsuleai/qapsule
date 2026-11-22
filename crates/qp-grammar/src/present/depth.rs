//! Depth rules and surface reduction (`docs/GRAMMAR.md` §13, G4): what the presenter shows
//! for one finding at `sentence`, `capsule` and `chart` depth. Pure functions of the
//! finding, its facts, the data and the surface profile; the authority applies them.
//!
//! R3 (`docs/GRAMMAR.md` §18): a finding presented by a capsule with a story gets the
//! capsule card at `capsule` depth (glyph, sentence, ledger line, mini price line), and a
//! finding whose facts carry a meaning gets its evidence geometry drawn at `chart` depth.
//! Findings written before R3 keep the R2 rules exactly.

use std::borrow::Cow;
use std::collections::BTreeMap;

use qp_schema::capsule::{Geometry, GlyphPhase, GlyphSpec, LedgerStat, Story, Trust};
use qp_schema::grammar::{
    ColorSpec, Emphasis, EmphasisKind, Layer, Mark, Origin, Panel, PanelPos, Position, Scale,
    ScaleKind, SizeSpec, View,
};
use qp_schema::protocol::{HitItem, OpenDepth, Profile, SharedElement, Target};
use qp_schema::scene::{DataSource, PixelRect, SceneDesc};
use qp_schema::{Bar, Depth, Fact, Finding};

use crate::detmath::num;
use crate::error::{ErrorKind, GrammarError};
use crate::look::Look;
use crate::provider::{DataProvider, SeriesKind};

mod card;
mod opening;

use card::chart_view_r3;
pub use card::{
    capsule_card_view, capsule_card_view_in, evidence_layers, evidence_note, geometry_field,
    ledger_text, story_glyph, CARD_GLYPH_PX, CHART_GLYPH_PX, FRESHNESS_HALF_BARS,
    GLYPH_MIN_LEDGER_N,
};
pub(crate) use card::{has_story, named_panel, panel_at, uniq};
pub use opening::{
    deeper, presenter_depth, shallower, shared_elements, tap_opens, transition_anchors,
};

/// Bars shown before the fact bar in the chart view.
pub const CHART_LOOKBACK: u64 = qp_schema::params::system::DEPTH_CHART_LOOKBACK_BARS.default_u64();

/// What the presenter shows for a finding at one depth.
#[derive(Debug, Clone, PartialEq)]
pub struct Presented {
    /// Depth actually used (requested depth capped by the profile; `workspace` → `chart`).
    pub depth: Depth,
    /// One sentence: the capsule headline, or the finding's claim.
    pub text: String,
    /// Confidence (depth ≥ capsule).
    pub confidence: Option<f64>,
    /// The view: capsule view at `capsule` (only if the profile allows series data),
    /// presenter view at `chart`; none at `sentence`.
    pub view: Option<View>,
}

fn rank(d: Depth) -> u8 {
    match d {
        Depth::Sentence => 0,
        Depth::Capsule => 1,
        Depth::Chart => 2,
        Depth::Workspace => 3,
    }
}

/// The depth used: `requested` capped by `profile.depth`, and never deeper than `chart`.
#[must_use]
pub fn effective_depth(requested: Depth, profile: &Profile) -> Depth {
    let d = if rank(profile.depth) < rank(requested) {
        profile.depth
    } else {
        requested
    };
    if rank(d) > rank(Depth::Chart) {
        Depth::Chart
    } else {
        d
    }
}

fn not_found(path: &str, why: String) -> GrammarError {
    GrammarError::new(ErrorKind::NotFound, path, why)
}

pub(crate) fn finding_and_fact<'p>(
    id: &str,
    p: &'p dyn DataProvider,
) -> Result<(Cow<'p, Finding>, Cow<'p, Fact>), GrammarError> {
    let finding = p
        .finding(id)
        .ok_or_else(|| not_found("/finding", format!("finding `{id}` does not exist")))?;
    let fid = finding.facts.first().ok_or_else(|| {
        not_found(
            "/finding",
            format!("finding `{id}` has no facts to present"),
        )
    })?;
    let fact = p
        .fact(fid)
        .ok_or_else(|| not_found("/finding", format!("fact `{fid}` does not exist")))?;
    Ok((finding, fact))
}

/// The bar series a fact's series sits on, and its bars (borrowed from the provider;
/// only the first/last visible bar times are read).
pub(crate) fn bar_series_bars<'p>(
    fact: &Fact,
    p: &'p dyn DataProvider,
) -> Result<(String, Cow<'p, [Bar]>), GrammarError> {
    let bs = match p.series_kind(&fact.series) {
        Some(SeriesKind::Bars) => Some(fact.series.clone()),
        Some(SeriesKind::Values) => p.input_bars(&fact.series),
        _ => None,
    }
    .ok_or_else(|| not_found("/finding", format!("series `{}` has no bars", fact.series)))?;
    match p.bars(&bs) {
        Some(bars) if !bars.is_empty() => Ok((bs, bars)),
        _ => Err(not_found("/finding", format!("series `{bs}` has no bars"))),
    }
}

pub(crate) fn main_field(series: &str, p: &dyn DataProvider) -> Option<String> {
    match p.series_kind(series)? {
        SeriesKind::Bars => Some("c".to_owned()),
        SeriesKind::Points => Some("v".to_owned()),
        SeriesKind::Values => p.main_field(series),
    }
}

pub(crate) fn layer(id: &str, mark: Mark, panel: u32) -> Layer {
    let mut l = Layer::new(mark, panel);
    l.id = Some(id.to_owned());
    l
}

/// A rule at the fact's `level`, its origin the fact (`docs/GRAMMAR.md` §10).
fn level_rule(fact: &Fact, panel: u32) -> Option<Layer> {
    let level = fact.evidence.get("level")?;
    let mut rule = layer("level", Mark::Rule, panel);
    // `qp/0` writes constants as numeric strings ("70"); kept for RD1 transcript 16.
    rule.encoding.y = Some(Position::Name(num(*level)));
    rule.origin = BTreeMap::from([(
        "/encoding/y".to_owned(),
        Origin::Fact {
            fact: fact.id.clone(),
            field: "level".to_owned(),
        },
    )]);
    Some(rule)
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn time_scale(t0: i64, t1: i64) -> Scale {
    Scale {
        kind: ScaleKind::Time,
        domain: Some([t0 as f64, t1 as f64]),
        id: Some("x".to_owned()),
    }
}

fn linear_panel(id: u32) -> Panel {
    Panel {
        id: Some(id),
        name: None,
        weight: None,
        y: Scale {
            kind: ScaleKind::Linear,
            domain: None,
            id: Some(format!("y{id}")),
        },
    }
}

/// Depth 0: the capsule headline if there is one, else the finding's claim.
///
/// # Errors
/// The finding does not exist.
pub fn sentence(finding: &str, p: &dyn DataProvider) -> Result<String, GrammarError> {
    let f = p
        .finding(finding)
        .ok_or_else(|| not_found("/finding", format!("finding `{finding}` does not exist")))?;
    Ok(p.headline(finding).unwrap_or_else(|| f.claim.clone()))
}

/// Depth 1 view. For an R3 capsule (a capsule with a story presents the finding): the
/// capsule card ([`capsule_card_view`]). Otherwise (R2): one panel; the main output of
/// the fact's series over the last `max_points` bars as a line; a rule at the fact's level
/// (origin: the fact) if it has one; a point at the fact bar (data: the fact) if that bar
/// is shown. Title: the sentence.
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn capsule_view(
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    capsule_view_in(&crate::look::R4, finding, max_points, p, id)
}

/// (R5/T3) [`capsule_view`] with the sizes of `look` (the card's glyph, sentence and
/// ledger; `r4`: [`capsule_view`]).
///
/// # Errors
/// As [`capsule_view`].
pub fn capsule_view_in(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    if has_story(finding, p) {
        return capsule_card_view_in(look, finding, max_points, p, id);
    }
    let (_, fact) = finding_and_fact(finding, p)?;
    let (_, bars) = bar_series_bars(&fact, p)?;
    let n = bars.len() as u64;
    let from = n.saturating_sub(u64::from(max_points.max(1)));
    let main = main_field(&fact.series, p).ok_or_else(|| {
        not_found(
            "/finding",
            format!("series `{}` has no main field", fact.series),
        )
    })?;
    let mut line = layer("line", Mark::Line, 0);
    line.series = Some(fact.series.clone());
    line.encoding.y = Some(Position::Name(main));
    let mut layers = vec![line];
    if let Some(rule) = level_rule(&fact, 0) {
        layers.push(rule);
    }
    if fact.bar >= from && fact.bar < n {
        let mut point = layer("fact", Mark::Point, 0);
        point.fact = Some(fact.id.clone());
        point.encoding.y = Some(Position::Name("value".to_owned()));
        layers.push(point);
    }
    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    Ok(View {
        id: id.to_owned(),
        title: Some(sentence(finding, p)?),
        display_tz: None,
        theme: None,
        x: time_scale(bars[i(from)].t, bars[i(n - 1)].t),
        y: Vec::new(),
        panels: vec![linear_panel(0)],
        layers,
        emphasis: Vec::new(),
    })
}

/// Depth 2 view (the presenter's view): panel 0 candles of the fact's bar series; if the
/// fact is on a study series, panel 1 its line and a rule at the fact's level (origin:
/// the fact); `x.domain = [t(max(0, fact bar − 30)), t(last bar)]`; `highlight` emphasis
/// on the fact. When the finding's first fact has a meaning (R3), its evidence geometry
/// and the capsule's glyph are added ([`evidence_layers`], `docs/GRAMMAR.md` §18).
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn chart_view(finding: &str, p: &dyn DataProvider, id: &str) -> Result<View, GrammarError> {
    chart_view_in(&crate::look::R4, finding, p, id)
}

/// (R5/T3) [`chart_view`] with the sizes of `look` (the glyph over the chart, the note;
/// `r4`: [`chart_view`]).
///
/// # Errors
/// As [`chart_view`].
pub fn chart_view_in(
    look: &Look,
    finding: &str,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let (f, fact) = finding_and_fact(finding, p)?;
    if fact.meaning.is_some() {
        return chart_view_r3(look, &f, &fact, p, id);
    }
    let (bs, bars) = bar_series_bars(&fact, p)?;
    let n = bars.len() as u64;
    let from = fact.bar.saturating_sub(CHART_LOOKBACK).min(n - 1);
    let mut candles = layer("candles", Mark::Candle, 0);
    candles.series = Some(bs.clone());
    let mut layers = vec![candles];
    let mut panels = vec![linear_panel(0)];
    if fact.series != bs {
        panels.push(linear_panel(1));
        let mut line = layer("line", Mark::Line, 1);
        line.series = Some(fact.series.clone());
        layers.push(line);
        if let Some(rule) = level_rule(&fact, 1) {
            layers.push(rule);
        }
    }
    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        theme: None,
        x: time_scale(bars[i(from)].t, bars[i(n - 1)].t),
        y: Vec::new(),
        panels,
        layers,
        emphasis: vec![Emphasis {
            target: Target {
                item: Some(fact.id.clone()),
                ..Target::default()
            },
            kind: EmphasisKind::Highlight,
        }],
    })
}

/// The presenter's output for `finding` at `depth` on a surface with `profile`.
/// View ids: `capsule-<finding>` and `present-<finding>`.
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn present(
    finding: &str,
    depth: Depth,
    profile: &Profile,
    p: &dyn DataProvider,
) -> Result<Presented, GrammarError> {
    let depth = effective_depth(depth, profile);
    let f = p
        .finding(finding)
        .ok_or_else(|| not_found("/finding", format!("finding `{finding}` does not exist")))?;
    let text = sentence(finding, p)?;
    let (confidence, view) = match depth {
        Depth::Sentence => (None, None),
        Depth::Capsule => (
            Some(f.confidence),
            if profile.series_data && profile.max_points > 0 {
                Some(capsule_view(
                    finding,
                    profile.max_points,
                    p,
                    &format!("capsule-{finding}"),
                )?)
            } else {
                None
            },
        ),
        Depth::Chart | Depth::Workspace => (
            Some(f.confidence),
            Some(chart_view(finding, p, &format!("present-{finding}"))?),
        ),
    };
    Ok(Presented {
        depth,
        text,
        confidence,
        view,
    })
}

// ---------------------------------------------------------------------------
// R4/T5: capsule face designs (`docs/PROTOCOL.md` §4.6.2)
// ---------------------------------------------------------------------------

/// Whether `name` is a known face design (`crate::faces`, R4/H-M8910: the registry with
/// each design's height; `card` is the earlier name of `r3`).
#[must_use]
pub fn is_face(name: &str) -> bool {
    crate::faces::is_face(name)
}

/// The face of a capsule in the feed (open depth `sentence`) by face design `face`
/// (`None`: the default, `r3` = the §18.4 capsule card [`capsule_view`] at `max_points`).
/// The designs are registered in [`crate::faces::FACES`].
///
/// # Errors
/// An unknown face name (`invalid`, `/face`) or the design's own errors.
pub fn face_view(
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    crate::faces::face_view(face, finding, max_points, p, id)
}

/// (R5/T3) [`face_view`] in `look`.
///
/// # Errors
/// As [`face_view`].
pub fn face_view_in(
    look: &Look,
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    crate::faces::face_view_in(look, face, finding, max_points, p, id)
}

// ---------------------------------------------------------------------------
// R5/M18: the story chart (`crate::stories`)
// ---------------------------------------------------------------------------

/// (R5/M18, K50) The story chart design a connection chose (`ViewState.story`), drawn at
/// `chart`/`evidence` depth around the open capsule's chart; `None` (the default, `r4`):
/// the open capsule's chart alone. The designs are registered in
/// [`crate::stories::STORIES`].
#[must_use]
pub fn story_design(name: Option<&str>) -> Option<&'static crate::stories::StoryDesign> {
    crate::stories::by_name(name)
}

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn chart_lookback_binds_registry() {
        assert_eq!(super::CHART_LOOKBACK, 30);
    }
}
