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

/// Bars shown before the fact bar in the chart view.
pub const CHART_LOOKBACK: u64 = 30;

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

fn finding_and_fact<'p>(
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
fn bar_series_bars<'p>(
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

fn main_field(series: &str, p: &dyn DataProvider) -> Option<String> {
    match p.series_kind(series)? {
        SeriesKind::Bars => Some("c".to_owned()),
        SeriesKind::Points => Some("v".to_owned()),
        SeriesKind::Values => p.main_field(series),
    }
}

fn layer(id: &str, mark: Mark, panel: u32) -> Layer {
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
fn time_scale(t0: i64, t1: i64) -> Scale {
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
// R3: capsule card, evidence geometry, glyph parameters (`docs/GRAMMAR.md` §18)
// ---------------------------------------------------------------------------

/// Closed ledger rows a story needs at birth before its glyph shows a confidence fill
/// (the M2 template's ledger-sentence threshold).
pub const GLYPH_MIN_LEDGER_N: u32 = 20;

/// Bars after the last transition at which a glyph's freshness is one half.
pub const FRESHNESS_HALF_BARS: f64 = 5.0;

/// Glyph size in the capsule card (the `r4` look's `glyph.card_px`).
pub const CARD_GLYPH_PX: f64 = crate::look::R4.glyph.card_px;
/// Glyph size over the chart at `chart` depth (the `r4` look's `glyph.chart_px`).
pub const CHART_GLYPH_PX: f64 = crate::look::R4.glyph.chart_px;

/// The glyph parameters of a story at bar `now_bar` (K30 channels). The same rule as the
/// engine's `qp_capsule::glyph_spec(story, now_bar, GLYPH_MIN_LEDGER_N)` (cross-checked in
/// `tests/capsule_depth.rs`): confidence `realized / n` of the ledger at birth when
/// `n >= GLYPH_MIN_LEDGER_N`; freshness `h / (h + age)`, `h =` [`FRESHNESS_HALF_BARS`],
/// `age = now_bar − last_bar`; phase from the stage; confirmations = families (1–5);
/// server mark from the trust path. A provider whose authority runs the engine may
/// supply the engine's own value instead ([`DataProvider::glyph`]).
#[must_use]
pub fn story_glyph(story: &Story, now_bar: u64) -> GlyphSpec {
    let lab = story.ledger_at_birth;
    let confidence =
        (lab.n >= GLYPH_MIN_LEDGER_N.max(1)).then(|| f64::from(lab.realized) / f64::from(lab.n));
    #[allow(clippy::cast_precision_loss)]
    let age = now_bar.saturating_sub(story.last_bar) as f64;
    GlyphSpec {
        kind: story.kind,
        direction: story.direction,
        strength: story.strength,
        confidence,
        phase: GlyphPhase::from(story.stage),
        freshness: FRESHNESS_HALF_BARS / (FRESHNESS_HALF_BARS + age),
        confirmations: u8::try_from(story.families.len().clamp(1, 5)).unwrap_or(5),
        server: story.trust == Trust::Server,
        validity: None,
    }
}

/// The capsule card's ledger line: `Karne <realized>/<n>` (closed capsules of the same
/// ledger key before the birth bar), or `henüz karne yok` when none closed yet.
#[must_use]
pub fn ledger_text(st: &LedgerStat) -> String {
    if st.n == 0 {
        "henüz karne yok".to_owned()
    } else {
        format!("Karne {}/{}", st.realized, st.n)
    }
}

/// A bar of a fact's evidence geometry, addressed as an origin field (R3):
/// `geometry.<i>.from_bar`, `geometry.<i>.to_bar` (level, band) or
/// `geometry.<i>.points.<j>.bar`. `None` for anything else.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn geometry_field(fact: &Fact, field: &str) -> Option<f64> {
    let rest = field.strip_prefix("geometry.")?;
    let (i, rest) = rest.split_once('.')?;
    let g = fact
        .meaning
        .as_ref()?
        .geometry
        .get(i.parse::<usize>().ok()?)?;
    let bar = match (g, rest) {
        (Geometry::Level { from_bar, .. } | Geometry::Band { from_bar, .. }, "from_bar") => {
            *from_bar
        }
        (Geometry::Level { to_bar, .. } | Geometry::Band { to_bar, .. }, "to_bar") => *to_bar,
        (Geometry::Points { points }, r) => {
            let j = r.strip_prefix("points.")?.strip_suffix(".bar")?;
            points.get(j.parse::<usize>().ok()?)?.bar
        }
        _ => return None,
    };
    Some(bar as f64)
}

/// Evidence keys a geometry draws.
fn geometry_keys(g: &Geometry) -> Vec<&str> {
    match g {
        Geometry::Level { key, .. } => vec![key.as_str()],
        Geometry::Band { upper, lower, .. } => vec![upper.as_str(), lower.as_str()],
        Geometry::Points { points } => points.iter().map(|p| p.key.as_str()).collect(),
    }
}

/// When a fact's evidence geometry cannot be drawn because its values are absent (the
/// authority redacted them for a reader without the data entitlement, K33): the note the
/// chart shows instead. `None` when every geometry value is present or there is none.
#[must_use]
pub fn evidence_note(fact: &Fact) -> Option<String> {
    let m = fact.meaning.as_ref()?;
    let missing = m
        .geometry
        .iter()
        .flat_map(geometry_keys)
        .any(|k| !fact.evidence.contains_key(k));
    missing.then(|| match &m.license.entitlement {
        Some(e) => format!("Kanıt değerleri veri yetkisi gerektirir: {e}"),
        None => "Kanıt değerleri bu okuyucuya açık değil".to_owned(),
    })
}

/// Whether an R3 capsule with a story presents `finding`.
fn has_story(finding: &str, p: &dyn DataProvider) -> bool {
    p.capsule(finding).is_some_and(|c| c.story.is_some())
}

fn panel_at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

fn named_panel(id: u32, name: &str, weight: f64) -> Panel {
    let mut p = linear_panel(id);
    p.name = Some(name.to_owned());
    p.weight = Some(weight);
    p
}

fn fact_origin(fact: &Fact, field: &str) -> Origin {
    Origin::Fact {
        fact: fact.id.clone(),
        field: field.to_owned(),
    }
}

/// The capsule card view (R3, `capsule` depth): panel 0 `card` holds the glyph
/// (`glyph`), the sentence (`sentence`, the capsule headline) and the ledger line
/// (`ledger`), all panel-positioned; panel 1 `price` holds the mini price line (`line`:
/// close of the fact's bar series over the last `max_points` bars), a rule at the fact's
/// `level` if it has one (`level`, origin: the fact) and a point at the fact bar if shown
/// (`fact`). Title: the instrument (derived).
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn capsule_card_view(
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    capsule_card_view_in(&crate::look::R4, finding, max_points, p, id)
}

/// (R5/T3) [`capsule_card_view`] with the sizes of `look`: glyph `glyph.card_px`, the
/// sentence in the `body` role beside it (`space.glyph_gap`), the ledger line in the
/// `label` role.
///
/// # Errors
/// As [`capsule_card_view`].
pub fn capsule_card_view_in(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let (_, fact) = finding_and_fact(finding, p)?;
    let (bs, bars) = bar_series_bars(&fact, p)?;
    let n = bars.len() as u64;
    let from = n.saturating_sub(u64::from(max_points.max(1)));
    let mut glyph = layer("glyph", Mark::Glyph, 0);
    glyph.finding = Some(finding.to_owned());
    glyph.encoding.x = Some(panel_at(0.0, 0.0));
    glyph.encoding.y = Some(panel_at(0.0, 0.0));
    let card_px = look.glyph.card_px;
    glyph.encoding.size = Some(SizeSpec::Constant(card_px));
    let mut sentence = layer("sentence", Mark::Text, 0);
    sentence.finding = Some(finding.to_owned());
    sentence.encoding.text = Some("headline".to_owned());
    sentence.encoding.x = Some(panel_at(0.0, card_px + look.space.glyph_gap));
    sentence.encoding.y = Some(panel_at(0.0, 0.0));
    sentence.encoding.size = Some(SizeSpec::Constant(look.types.body.px));
    let mut ledger = layer("ledger", Mark::Text, 0);
    ledger.finding = Some(finding.to_owned());
    ledger.encoding.text = Some("ledger".to_owned());
    ledger.encoding.x = Some(panel_at(0.0, 0.0));
    // The ledger line's top: its size plus 2 px above the panel's bottom.
    ledger.encoding.y = Some(panel_at(1.0, -(look.types.label.px + 2.0)));
    ledger.encoding.size = Some(SizeSpec::Constant(look.types.label.px));
    ledger.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));
    let mut line = layer("line", Mark::Line, 1);
    line.series = Some(bs);
    line.encoding.y = Some(Position::Name("c".to_owned()));
    line.label = Some("C".to_owned());
    let mut layers = vec![glyph, sentence, ledger, line];
    if fact.series_is_bars(p) {
        if let Some(rule) = level_rule(&fact, 1) {
            layers.push(rule);
        }
        if fact.bar >= from && fact.bar < n {
            let mut point = layer("fact", Mark::Point, 1);
            point.fact = Some(fact.id.clone());
            point.encoding.y = Some(Position::Name("value".to_owned()));
            layers.push(point);
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
        panels: vec![named_panel(0, "card", 3.0), named_panel(1, "price", 2.0)],
        layers,
        emphasis: Vec::new(),
    })
}

trait FactExt {
    fn series_is_bars(&self, p: &dyn DataProvider) -> bool;
}

impl FactExt for Fact {
    fn series_is_bars(&self, p: &dyn DataProvider) -> bool {
        p.series_kind(&self.series) == Some(SeriesKind::Bars)
    }
}

/// Unique layer ids: `base`, then `base-2`, `base-3`… (K25: the presenter's own names).
fn uniq(taken: &mut Vec<String>, base: &str) -> String {
    let mut id = base.to_owned();
    let mut k = 2;
    while taken.contains(&id) {
        id = format!("{base}-{k}");
        k += 1;
    }
    taken.push(id.clone());
    id
}

/// Binds the channel at `ptr` to `field` of `fact` by origin only: the view never carries
/// the value (K33). The compiler resolves it from the reader's provider at compile time,
/// so a reader whose provider redacts the evidence gets no number from the view.
fn bind(l: &mut Layer, ptr: &str, fact: &Fact, field: &str) {
    l.origin.insert(ptr.to_owned(), fact_origin(fact, field));
}

/// The evidence geometry of `fact` as layers in `panel` (R3, `docs/GRAMMAR.md` §18):
/// `Level` → a rule segment (`level`), `Band` → a band over its bars (`band`), `Points`
/// → a point per swing (`swing-point`) joined by segments (`swing`). Every position — the
/// values and the bars — is bound to the fact **by origin only** (evidence key,
/// `geometry.<i>.…`): the layer carries no number; the compiler resolves each one from
/// the reader's provider (K33: a view never carries evidence values to a reader). A
/// geometry whose values are absent from the evidence (redacted) gets no layer at all;
/// the chart says what the evidence needs instead ([`evidence_note`]).
#[must_use]
pub fn evidence_layers(fact: &Fact, panel: u32, taken: &mut Vec<String>) -> Vec<Layer> {
    let mut out = Vec::new();
    let Some(m) = &fact.meaning else {
        return out;
    };
    let ev = |k: &str| fact.evidence.get(k).copied();
    let accent = || Some(ColorSpec::Constant("accent".to_owned()));
    for (gi, g) in m.geometry.iter().enumerate() {
        if geometry_keys(g).iter().any(|k| ev(k).is_none()) {
            continue;
        }
        let gf = |what: &str| format!("geometry.{gi}.{what}");
        match g {
            Geometry::Level { key, .. } => {
                let mut l = layer(&uniq(taken, "level"), Mark::Rule, panel);
                bind(&mut l, "/encoding/y", fact, key);
                bind(&mut l, "/encoding/x", fact, &gf("from_bar"));
                bind(&mut l, "/encoding/x2", fact, &gf("to_bar"));
                l.encoding.color = accent();
                l.encoding.width = Some(2.0);
                out.push(l);
            }
            Geometry::Band { upper, lower, .. } => {
                let mut l = layer(&uniq(taken, "band"), Mark::Band, panel);
                bind(&mut l, "/encoding/y", fact, upper);
                bind(&mut l, "/encoding/y2", fact, lower);
                bind(&mut l, "/encoding/x", fact, &gf("from_bar"));
                bind(&mut l, "/encoding/x2", fact, &gf("to_bar"));
                l.encoding.color = accent();
                l.encoding.opacity = Some(0.18);
                l.encoding.width = Some(1.0);
                out.push(l);
            }
            Geometry::Points { points } => {
                for (j, w) in points.windows(2).enumerate() {
                    let mut l = layer(&uniq(taken, "swing"), Mark::Rule, panel);
                    let (a, b) = (&w[0], &w[1]);
                    bind(&mut l, "/encoding/y", fact, &a.key);
                    bind(&mut l, "/encoding/y2", fact, &b.key);
                    let pa = gf(&format!("points.{j}.bar"));
                    let pb = gf(&format!("points.{}.bar", j + 1));
                    bind(&mut l, "/encoding/x", fact, &pa);
                    bind(&mut l, "/encoding/x2", fact, &pb);
                    l.encoding.color = accent();
                    l.encoding.width = Some(2.0);
                    out.push(l);
                }
                for (j, pt) in points.iter().enumerate() {
                    let mut l = layer(&uniq(taken, "swing-point"), Mark::Point, panel);
                    bind(&mut l, "/encoding/y", fact, &pt.key);
                    bind(&mut l, "/encoding/x", fact, &gf(&format!("points.{j}.bar")));
                    l.encoding.color = accent();
                    l.encoding.size = Some(SizeSpec::Constant(9.0));
                    out.push(l);
                }
            }
        }
    }
    out
}

/// The R3 chart view: the R2 chart rule plus, in the candle panel, the evidence geometry
/// of every fact of the finding (in order), the capsule's glyph over the first fact's bar
/// (when a capsule with a story presents the finding), and, when a geometry's values are
/// absent (redacted), a note (`evidence`) saying which data entitlement the evidence
/// needs. The visible range starts 30 bars before the first fact or at the first
/// geometry bar, whichever is earlier.
fn chart_view_r3(
    look: &Look,
    finding: &Finding,
    fact: &Fact,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let (bs, bars) = bar_series_bars(fact, p)?;
    let n = bars.len() as u64;
    let facts: Vec<Cow<'_, Fact>> = finding.facts.iter().filter_map(|f| p.fact(f)).collect();
    let mut from = fact.bar.saturating_sub(CHART_LOOKBACK);
    for f in &facts {
        for g in f.meaning.iter().flat_map(|m| &m.geometry) {
            let first = match g {
                Geometry::Level { from_bar, .. } | Geometry::Band { from_bar, .. } => *from_bar,
                Geometry::Points { points } => points.iter().map(|q| q.bar).min().unwrap_or(f.bar),
            };
            from = from.min(first.saturating_sub(5));
        }
    }
    let from = from.min(n - 1);
    let mut candles = layer("candles", Mark::Candle, 0);
    candles.series = Some(bs.clone());
    let mut taken: Vec<String> = vec!["candles".to_owned()];
    let mut layers = vec![candles];
    let mut note: Option<&Fact> = None;
    for f in &facts {
        if f.series == bs {
            // Bands go under the candles, lines and points over them.
            for l in evidence_layers(f, 0, &mut taken) {
                if l.mark == Mark::Band {
                    let at = layers
                        .iter()
                        .position(|x| x.mark == Mark::Candle)
                        .unwrap_or(0);
                    layers.insert(at, l);
                } else {
                    layers.push(l);
                }
            }
        }
        if note.is_none() && evidence_note(f).is_some() {
            note = Some(f);
        }
    }
    if let Some(f) = note {
        let mut t = layer(&uniq(&mut taken, "evidence"), Mark::Text, 0);
        t.fact = Some(f.id.clone());
        t.encoding.text = Some("requires".to_owned());
        t.encoding.x = Some(panel_at(0.0, 4.0));
        t.encoding.y = Some(panel_at(0.0, look.glyph.chart_px + 8.0));
        t.encoding.size = Some(SizeSpec::Constant(look.types.body.px));
        t.encoding.color = Some(ColorSpec::Constant("warn".to_owned()));
        layers.push(t);
    }
    if p.glyph(&finding.id).is_some() {
        let mut g = layer(&uniq(&mut taken, "glyph"), Mark::Glyph, 0);
        g.finding = Some(finding.id.clone());
        g.encoding.size = Some(SizeSpec::Constant(look.glyph.chart_px));
        layers.push(g);
    }
    let mut panels = vec![linear_panel(0)];
    if fact.series != bs {
        panels.push(linear_panel(1));
        let mut line = layer(&uniq(&mut taken, "line"), Mark::Line, 1);
        line.series = Some(fact.series.clone());
        layers.push(line);
        if let Some(mut rule) = level_rule(fact, 1) {
            rule.id = Some(uniq(&mut taken, "level"));
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

// ---------------------------------------------------------------------------
// R4: opening between depths (`docs/GRAMMAR.md` §18.6, `docs/PROTOCOL.md` §4.6)
// ---------------------------------------------------------------------------

/// One depth further: sentence → capsule → chart → evidence; `None` at evidence.
#[must_use]
pub fn deeper(d: OpenDepth) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence => Some(OpenDepth::Capsule),
        OpenDepth::Capsule => Some(OpenDepth::Chart),
        OpenDepth::Chart => Some(OpenDepth::Evidence),
        OpenDepth::Evidence => None,
    }
}

/// One depth back; `None` at sentence.
#[must_use]
pub fn shallower(d: OpenDepth) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence => None,
        OpenDepth::Capsule => Some(OpenDepth::Sentence),
        OpenDepth::Chart => Some(OpenDepth::Capsule),
        OpenDepth::Evidence => Some(OpenDepth::Chart),
    }
}

/// The presenter depth that draws an open depth (`evidence` is drawn at `chart`).
#[must_use]
pub fn presenter_depth(d: OpenDepth) -> Depth {
    match d {
        OpenDepth::Sentence => Depth::Sentence,
        OpenDepth::Capsule => Depth::Capsule,
        OpenDepth::Chart | OpenDepth::Evidence => Depth::Chart,
    }
}

/// The opening rule of a tap (R4): what a tap on a capsule's scene at depth `d` opens.
/// `item` is what the point query found under the finger (`None`: empty area).
///
/// - `sentence` (a face or sentence in the feed): anywhere → `capsule`.
/// - `capsule` (the opened capsule): anywhere → `chart`.
/// - `chart`: on an element of the evidence — a fact or finding layer (the fact point,
///   the glyph) or a layer with a constant whose origin is a fact (the level, band and
///   swing geometry) → `evidence`; anywhere else nothing opens (the tap focuses the bar).
/// - `evidence`: nothing opens.
#[must_use]
pub fn tap_opens(d: OpenDepth, item: Option<&HitItem>) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence | OpenDepth::Capsule => deeper(d),
        OpenDepth::Chart => item.filter(|i| is_evidence(i)).map(|_| OpenDepth::Evidence),
        OpenDepth::Evidence => None,
    }
}

fn is_evidence(i: &HitItem) -> bool {
    matches!(i.data, DataSource::Fact { .. } | DataSource::Finding { .. })
        || i.constants
            .values()
            .any(|c| matches!(c.origin, Origin::Fact { .. }))
}

fn union(a: Option<PixelRect>, b: PixelRect) -> PixelRect {
    match a {
        None => b,
        Some(a) => PixelRect {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        },
    }
}

/// The elements of a capsule's scene that a depth transition can keep, by id, with their
/// boxes (R4, `docs/GRAMMAR.md` §18.6): `glyph` (a glyph layer), `headline` and `ledger`
/// (the finding's text layers), `fact:<id>` (a layer drawing a fact), `evidence:<fact>`
/// (layers whose constants come from a fact: level, band, swing), `price` (the panel of
/// the layer drawing `price_series`), `bar:<i>` (the column of the marked bar
/// `marked_bar` in that panel, when it is visible). A box is the union of the element's
/// item hit boxes.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn transition_anchors(
    scene: &SceneDesc,
    price_series: Option<&str>,
    marked_bar: Option<u64>,
) -> BTreeMap<String, PixelRect> {
    let mut out: BTreeMap<String, PixelRect> = BTreeMap::new();
    let put = |out: &mut BTreeMap<String, PixelRect>, k: String, r: PixelRect| {
        let prev = out.get(&k).copied();
        out.insert(k, union(prev, r));
    };
    let mut price_panel: Option<PixelRect> = None;
    for p in &scene.panels {
        for l in &p.layers {
            let mut keys: Vec<String> = Vec::new();
            match (&l.data, l.mark) {
                (_, Mark::Glyph) => keys.push("glyph".to_owned()),
                (DataSource::Finding { .. }, Mark::Text) => {
                    for f in ["headline", "ledger"] {
                        if l.fields.iter().any(|x| x == f) {
                            keys.push(f.to_owned());
                        }
                    }
                }
                (DataSource::Fact { fact }, _) => keys.push(format!("fact:{fact}")),
                (DataSource::Series { series }, Mark::Candle | Mark::Line)
                    if Some(series.as_str()) == price_series && price_panel.is_none() =>
                {
                    price_panel = Some(p.rect);
                    put(&mut out, "price".to_owned(), p.rect);
                }
                _ => {}
            }
            for c in l.constants.values() {
                if let Origin::Fact { fact, .. } = &c.origin {
                    let k = format!("evidence:{fact}");
                    if !keys.contains(&k) {
                        keys.push(k);
                    }
                }
            }
            for k in keys {
                for it in &l.items {
                    put(&mut out, k.clone(), it.hit);
                }
            }
        }
    }
    if let (Some(rect), Some(b)) = (price_panel, marked_bar) {
        if b >= scene.x.from_bar && b <= scene.x.to_bar {
            let c = crate::scale::slot_x(&scene.x, b);
            let half = (scene.x.slot / 2.0).max(1.0);
            put(
                &mut out,
                format!("bar:{b}"),
                PixelRect {
                    left: (c - half).floor() as i32,
                    top: rect.top,
                    right: (c + half).ceil() as i32,
                    bottom: rect.bottom,
                },
            );
        }
    }
    out
}

/// The elements a transition keeps (R4): the ids both scenes have, in id order, with
/// both boxes. A side without a scene (`None`, the sentence depth) still shows the
/// sentence: `headline` is kept with no box on that side.
#[must_use]
pub fn shared_elements(
    before: Option<&BTreeMap<String, PixelRect>>,
    after: Option<&BTreeMap<String, PixelRect>>,
) -> Vec<SharedElement> {
    match (before, after) {
        (Some(b), Some(a)) => b
            .iter()
            .filter_map(|(k, rb)| {
                a.get(k).map(|ra| SharedElement {
                    id: k.clone(),
                    before: Some(*rb),
                    after: Some(*ra),
                })
            })
            .collect(),
        (b, a) => vec![SharedElement {
            id: "headline".to_owned(),
            before: b.and_then(|m| m.get("headline").copied()),
            after: a.and_then(|m| m.get("headline").copied()),
        }],
    }
}
