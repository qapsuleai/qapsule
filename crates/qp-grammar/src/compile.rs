//! `compile`: view + data provider + options → scene description (`docs/GRAMMAR.md` §14).

use std::borrow::Cow;
use std::collections::BTreeSet;

use qp_schema::grammar::{
    ColorSpec, Emphasis, EmphasisKind, Layer, Mark, Operand, Panel, Position, Scale, ScaleKind,
    View,
};
use qp_schema::scene::{
    Align, DataSource, Lod, PixelRect, Prim, Rgba, SceneDesc, SceneEmphasis, ScenePanel, XMap, YMap,
};

use crate::detmath::{self, nice_step, px, year_month};
use crate::draw::{draw, est_width, rect, Geo};
use crate::error::{ErrorKind, GrammarError};
use crate::frame::Frame;
use crate::layer::{prepare, Geom, Prepared};
use crate::provider::{DataProvider, SeriesKind};
use crate::scale::value_y;
use crate::theme::{with_opacity, Theme};

/// Smallest and largest accepted frame sizes (R0's limits).
pub const MIN_WIDTH: u32 = 200;
/// Smallest accepted height.
pub const MIN_HEIGHT: u32 = 120;
/// Largest accepted width or height.
pub const MAX_DIM: u32 = 16_384;

/// Plot margins (R0): left, right (from `W`), top, bottom (from `H`).
const LEFT: i32 = 8;
const RIGHT_MARGIN: i32 = 72;
const TOP: i32 = 34;
const BOTTOM_MARGIN: i32 = 24;
/// Gap between stacked panels.
const PANEL_GAP: i32 = 8;
/// Smallest panel height.
const MIN_PANEL_H: i32 = 10;
const TITLE_PX: f32 = 16.0;
const LABEL_PX: f32 = 11.0;
/// Alpha of emphasis strips (accent) and veils (background).
const STRIP_ALPHA: u8 = 56;
const VEIL_ALPHA: u8 = 160;
/// Hard cap on grid lines per panel, and the largest tick index that is still exact.
const MAX_TICKS: u32 = 64;
const MAX_EXACT_TICK: f64 = 4_503_599_627_370_496.0; // 2^52

/// Transient inputs of one compile: frame size, a connection's viewport, transient
/// emphasis (attention directives, narration steps) and a theme override.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileOptions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Visible slot range `(from_bar, to_bar)`, inclusive (a connection's transient
    /// viewport). Overrides `x.domain`; it is clamped to the existing slots.
    pub range: Option<(u64, u64)>,
    /// Transient emphasis, drawn after the view's persistent emphasis.
    pub emphasis: Vec<Emphasis>,
    /// Theme override (`dark`, `light`); the view's theme otherwise.
    pub theme: Option<String>,
    /// (R4/M11) Outcome style of glyphs by name (`qp_grammar::glyph::outcome::STYLES`);
    /// absent = `r3`, the R3 drawing. A view setting like the theme override.
    pub outcome: Option<String>,
    /// Level of detail when slots are narrower than one pixel (default `true`).
    pub lod: bool,
    /// Surface reduction: aggregate when more than this many slots are visible
    /// (`docs/GRAMMAR.md` §13, `profile.max_points`).
    pub max_points: Option<u64>,
    /// (R4/T5) Glyph alphabet by name (`crate::glyph::ALPHABETS`); the product's if
    /// `None`.
    pub alphabet: Option<String>,
}

impl CompileOptions {
    /// Options for a `width` x `height` frame, everything else default.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            range: None,
            emphasis: Vec::new(),
            theme: None,
            outcome: None,
            lod: true,
            max_points: None,
            alphabet: None,
        }
    }
}

/// The view's panels, canonical (`docs/GRAMMAR.md` §11 rule 1) even when the input still
/// uses the deprecated `y` list.
fn canonical_panels(view: &View) -> Result<Vec<Panel>, GrammarError> {
    if !view.panels.is_empty() && !view.y.is_empty() {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/y",
            "both `panels` and the deprecated `y` are set",
        ));
    }
    let mut panels: Vec<Panel> = if !view.panels.is_empty() {
        view.panels.clone()
    } else if !view.y.is_empty() {
        (0u32..)
            .zip(&view.y)
            .map(|(i, y)| Panel {
                id: Some(i),
                name: None,
                weight: None,
                y: y.clone(),
            })
            .collect()
    } else {
        let ids: BTreeSet<u32> = view.layers.iter().map(|l| l.panel).collect();
        ids.into_iter()
            .map(|i| Panel {
                id: Some(i),
                name: None,
                weight: None,
                y: Scale::new(ScaleKind::Linear),
            })
            .collect()
    };
    let mut next = panels
        .iter()
        .filter_map(|p| p.id)
        .max()
        .map_or(0, |m| m + 1);
    for p in &mut panels {
        if p.id.is_none() {
            p.id = Some(next);
            next += 1;
        }
    }
    let mut seen = BTreeSet::new();
    for (i, p) in panels.iter().enumerate() {
        let id = p.id.unwrap_or_default();
        if !seen.insert(id) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/id"),
                format!("duplicate panel id {id}"),
            ));
        }
    }
    Ok(panels)
}

/// Layer ids, assigning the smallest unused `l<k>` to layers without one (§11 rule 3).
fn layer_ids(layers: &[Layer]) -> Result<Vec<String>, GrammarError> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    for (i, l) in layers.iter().enumerate() {
        if let Some(id) = &l.id {
            if !used.insert(id.clone()) {
                return Err(GrammarError::new(
                    ErrorKind::Invalid,
                    format!("/layers/{i}/id"),
                    format!("duplicate layer id `{id}`"),
                ));
            }
        }
    }
    let mut k = 0u32;
    Ok(layers
        .iter()
        .map(|l| {
            l.id.clone().unwrap_or_else(|| loop {
                let c = format!("l{k}");
                k += 1;
                if used.insert(c.clone()) {
                    break c;
                }
            })
        })
        .collect())
}

/// The view's first bar series (`docs/GRAMMAR.md` §1).
fn first_bar_series(view: &View, p: &dyn DataProvider) -> Option<String> {
    let bars = |s: &str| p.series_kind(s) == Some(SeriesKind::Bars);
    view.layers
        .iter()
        .filter(|l| l.mark == Mark::Candle)
        .find_map(|l| l.series.clone().filter(|s| bars(s)))
        .or_else(|| {
            view.layers
                .iter()
                .find_map(|l| l.series.clone().filter(|s| bars(s)))
        })
        .or_else(|| {
            view.layers.iter().find_map(|l| {
                let s = l.series.as_ref()?;
                (p.series_kind(s) == Some(SeriesKind::Values))
                    .then(|| p.input_bars(s))
                    .flatten()
                    .filter(|b| bars(b))
            })
        })
        .or_else(|| {
            // A fact/finding layer: the bar series of its (first) fact's series.
            view.layers.iter().find_map(|l| {
                let fact = match (&l.fact, &l.finding) {
                    (Some(f), _) => p.fact(f),
                    (_, Some(f)) => p
                        .finding(f)
                        .and_then(|x| x.facts.first().and_then(|f| p.fact(f))),
                    _ => None,
                }?;
                match p.series_kind(&fact.series)? {
                    SeriesKind::Bars => Some(fact.series.clone()),
                    SeriesKind::Values => p.input_bars(&fact.series).filter(|b| bars(b)),
                    SeriesKind::Points => None,
                }
            })
        })
}

/// Every point series the view reads (layer data and channel references).
/// Times of the slots after the last bar (`last_t`): the k-th point after the last bar
/// of any point series the view draws sits in slot `last bar + k` (`docs/GRAMMAR.md` §3);
/// the first series to reach a slot gives its time.
fn ext_times(view: &View, provider: &dyn DataProvider, last_t: i64) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for s in point_series(view, provider) {
        if let Some(pts) = provider.points(&s) {
            for (k, pt) in pts.iter().filter(|pt| pt.t > last_t).enumerate() {
                if k >= out.len() {
                    out.push(pt.t);
                }
            }
        }
    }
    out
}

/// Times of the view's future slots (slots after the last bar of its bar series, taken
/// by point series such as a forecast fan; `docs/GRAMMAR.md` §3), in slot order. Empty
/// when the view has no bar series, no bars or no point after the last bar. The
/// authority uses it to let a connection's viewport reach the future slots (K25).
#[must_use]
pub fn future_times(view: &View, provider: &dyn DataProvider) -> Vec<i64> {
    let Some(bs) = first_bar_series(view, provider) else {
        return Vec::new();
    };
    match provider
        .bars(&bs)
        .as_deref()
        .and_then(<[qp_schema::Bar]>::last)
    {
        Some(last) => ext_times(view, provider, last.t),
        None => Vec::new(),
    }
}

fn point_series(view: &View, p: &dyn DataProvider) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if p.series_kind(s) == Some(SeriesKind::Points) && !out.iter().any(|x| x == s) {
            out.push(s.to_owned());
        }
    };
    for l in &view.layers {
        if let Some(s) = &l.series {
            add(s);
        }
        for pos in [&l.encoding.x, &l.encoding.y, &l.encoding.y2]
            .into_iter()
            .flatten()
        {
            if let Position::Ref(r) = pos {
                if let Some(s) = &r.series {
                    add(s);
                }
            }
        }
        if let Some(ColorSpec::When(w)) = &l.encoding.color {
            if let Operand::Ref(r) = &w.when.than {
                if let Some(s) = &r.series {
                    add(s);
                }
            }
        }
    }
    out
}

/// Panel rectangles: the plot split by weight with 8 px gaps (last panel takes the rest).
fn panel_rects(
    panels: &[Panel],
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
) -> Result<Vec<PixelRect>, GrammarError> {
    let n = i32::try_from(panels.len()).unwrap_or(i32::MAX);
    let mut weights = Vec::with_capacity(panels.len());
    for (i, p) in panels.iter().enumerate() {
        let w = p.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 });
        if !(w.is_finite() && w > 0.0) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/weight"),
                "weight must be > 0",
            ));
        }
        weights.push(w);
    }
    let avail = bottom - top - PANEL_GAP * (n - 1);
    if avail < MIN_PANEL_H * n {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/panels",
            format!("{n} panels do not fit in a {}-pixel plot", bottom - top),
        ));
    }
    // Normalised by the largest weight so the sum cannot overflow (any finite weight > 0
    // is accepted, e.g. 1.7e308); every panel keeps at least MIN_PANEL_H pixels.
    let wmax = weights.iter().copied().fold(0.0_f64, f64::max);
    let weights: Vec<f64> = weights.iter().map(|w| w / wmax).collect();
    let total: f64 = weights.iter().sum();
    let mut rects = Vec::with_capacity(panels.len());
    let mut y = top;
    let mut used = 0;
    for (i, w) in weights.iter().enumerate() {
        let after = i32::try_from(panels.len() - i - 1).unwrap_or(0) * MIN_PANEL_H;
        #[allow(clippy::cast_possible_truncation)]
        let h = if i + 1 == panels.len() {
            avail - used
        } else {
            ((f64::from(avail) * w / total).floor() as i32)
                .max(MIN_PANEL_H)
                .min(avail - used - after)
        };
        rects.push(PixelRect {
            left,
            top: y,
            right,
            bottom: y + h,
        });
        used += h;
        y += h + PANEL_GAP;
    }
    Ok(rects)
}

/// Automatic or fixed domain of a panel scale (`docs/GRAMMAR.md` §3).
fn domain(scale: &Scale, values: &[f64], i: usize) -> Result<(f64, f64), GrammarError> {
    let log = scale.kind == ScaleKind::Log;
    if let Some([a, b]) = scale.domain {
        if !(a.is_finite() && b.is_finite() && a < b && (b - a).is_finite()) || (log && a <= 0.0) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/y/domain"),
                format!(
                    "domain [{a}, {b}] is invalid (need min < max{})",
                    if log { ", min > 0 on a log scale" } else { "" }
                ),
            ));
        }
        return Ok((a, b));
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        if log && *v <= 0.0 {
            continue;
        }
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if lo > hi {
        return Ok(if log { (1.0, 10.0) } else { (0.0, 1.0) });
    }
    if log {
        let (llo, lhi) = (
            detmath::log10(lo).unwrap_or(0.0),
            detmath::log10(hi).unwrap_or(0.0),
        );
        let span = (lhi - llo).max(1e-9);
        Ok((
            detmath::pow10(llo - span * 0.05),
            detmath::pow10(lhi + span * 0.05),
        ))
    } else {
        // Spans that overflow (values near ±f64::MAX) are clamped so the domain stays
        // finite; ordinary data takes R0's exact formula.
        let span = (hi - lo).clamp(1e-9, f64::MAX);
        let (a, b) = (
            (lo - span * 0.05).max(-f64::MAX),
            (hi + span * 0.05).min(f64::MAX),
        );
        if a < b && (b - a).is_finite() {
            return Ok((a, b));
        }
        let (a, b) = (a.max(-f64::MAX / 2.0), b.min(f64::MAX / 2.0));
        Ok(if a < b { (a, b) } else { (0.0, 1.0) })
    }
}

/// Values a prepared layer contributes to its panel's automatic domain.
fn domain_values(pl: &Prepared, out: &mut Vec<f64>) {
    if pl.geom == Geom::RuleV {
        return;
    }
    for t in &pl.traces {
        for r in &t.rows {
            out.extend(r.v.iter().flatten().copied());
        }
    }
}

/// Grid values of a panel: linear nice steps (R0), or 1-2-5 × 10^k on a log scale.
fn ticks(y: &YMap, divs: f64) -> Vec<f64> {
    let linear = |out: &mut Vec<f64>| {
        // Bounded integer loop (security review): `k += 1.0` stops advancing once
        // |k| >= 2^53, so the tick index is an integer with a hard cap.
        let step = nice_step((y.max - y.min) / divs);
        if !(step.is_finite() && step > 0.0) {
            return;
        }
        let k0 = (y.min / step).ceil();
        if !k0.is_finite() || k0.abs() >= MAX_EXACT_TICK {
            return;
        }
        for i in 0..MAX_TICKS {
            let v = (k0 + f64::from(i)) * step;
            if v > y.max {
                break;
            }
            out.push(v);
        }
    };
    let mut out = Vec::new();
    if y.kind != ScaleKind::Log {
        linear(&mut out);
        return out;
    }
    let (Some(a), Some(b)) = (detmath::log10(y.min), detmath::log10(y.max)) else {
        return out;
    };
    #[allow(clippy::cast_possible_truncation)]
    let (e0, e1) = (a.floor() as i32, b.ceil() as i32);
    let mut all = Vec::new();
    let mut ones = Vec::new();
    for e in e0..=e1 {
        let p = detmath::pow10_int(e);
        for m in [1.0, 2.0, 5.0] {
            let v = m * p;
            if v >= y.min && v <= y.max {
                all.push(v);
                if m == 1.0 {
                    ones.push(v);
                }
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    if all.len() < 2 {
        linear(&mut out);
    } else if all.len() as f64 > 2.0 * divs && ones.len() >= 2 {
        out = ones;
    } else {
        out = all;
    }
    out
}

struct Resolved {
    kind: EmphasisKind,
    panels: Vec<u32>,
    layer: Option<String>,
    bars: Option<(u64, u64)>,
}

/// Resolves emphasis targets (`docs/GRAMMAR.md` §8).
fn resolve_emphasis(
    view: &View,
    list: &[(String, &Emphasis)],
    prepared: &[Prepared],
    panel_ids: &[u32],
    p: &dyn DataProvider,
) -> Result<Vec<Resolved>, GrammarError> {
    let mut out = Vec::new();
    for (path, e) in list {
        let t = &e.target;
        if t.view.as_deref().is_some_and(|v| v != view.id) {
            continue;
        }
        let nf = |what: &str, why: String| {
            GrammarError::new(ErrorKind::NotFound, format!("{path}/target/{what}"), why)
        };
        let mut layers: Vec<&Prepared> = Vec::new();
        if let Some(l) = &t.layer {
            layers.push(
                prepared
                    .iter()
                    .find(|x| &x.id == l)
                    .ok_or_else(|| nf("layer", format!("layer `{l}` does not exist")))?,
            );
        } else if let Some(s) = &t.series {
            layers.extend(prepared.iter().filter(|x| {
                matches!(&x.data, qp_schema::scene::DataSource::Series { series } if series == s)
            }));
            if layers.is_empty() {
                return Err(nf("series", format!("no layer draws series `{s}`")));
            }
        }
        let mut bars = match (t.from_bar, t.to_bar) {
            (Some(a), Some(b)) => Some((a.min(b), a.max(b))),
            (Some(a), None) | (None, Some(a)) => Some((a, a)),
            (None, None) => None,
        };
        if let Some(item) = &t.item {
            let fact_bars: Vec<u64> = if let Some(f) = p.fact(item) {
                vec![f.bar]
            } else if let Some(f) = p.finding(item) {
                f.facts
                    .iter()
                    .filter_map(|x| p.fact(x))
                    .map(|x| x.bar)
                    .collect()
            } else {
                return Err(nf(
                    "item",
                    format!("item `{item}` is not a fact or finding"),
                ));
            };
            if let (Some(a), Some(b)) = (fact_bars.iter().min(), fact_bars.iter().max()) {
                bars = Some((*a, *b));
            }
        }
        let panels = match t.panel {
            Some(pid) => {
                if !panel_ids.contains(&pid) {
                    return Err(nf("panel", format!("panel {pid} does not exist")));
                }
                vec![pid]
            }
            None => {
                let mut ps: Vec<u32> = layers.iter().map(|l| l.panel).collect();
                ps.dedup();
                ps
            }
        };
        if layers.is_empty() {
            out.push(Resolved {
                kind: e.kind,
                panels,
                layer: None,
                bars,
            });
        } else {
            for l in layers {
                out.push(Resolved {
                    kind: e.kind,
                    panels: if t.panel.is_some() {
                        panels.clone()
                    } else {
                        vec![l.panel]
                    },
                    layer: Some(l.id.clone()),
                    bars,
                });
            }
        }
    }
    Ok(out)
}

/// Compiles `view` with data from `provider` into a scene description.
///
/// # Errors
/// [`GrammarError`] naming the layer, the JSON pointer and the reason for anything that
/// cannot be resolved: unknown series/field/fact/finding/panel/threshold, a mark without
/// the data it needs, a constant without origin, invalid colours, domains or sizes.
#[allow(clippy::too_many_lines)]
pub fn compile(
    view: &View,
    provider: &dyn DataProvider,
    opts: &CompileOptions,
) -> Result<SceneDesc, GrammarError> {
    let (width, height) = (opts.width, opts.height);
    if !(MIN_WIDTH..=MAX_DIM).contains(&width) || !(MIN_HEIGHT..=MAX_DIM).contains(&height) {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "",
            format!("size {width}x{height} out of range ({MIN_WIDTH}x{MIN_HEIGHT} ..= {MAX_DIM}x{MAX_DIM})"),
        ));
    }
    let theme = match opts.theme.as_deref().or(view.theme.as_deref()) {
        Some(n) => Theme::by_name(n)?,
        None => Theme::dark(),
    };
    let outcome = match opts.outcome.as_deref() {
        Some(n) => crate::glyph::outcome::by_name(n)?,
        None => crate::glyph::outcome::default_style(),
    };
    let alphabet = crate::glyph::alphabet_named(opts.alphabet.as_deref()).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "",
            format!(
                "unknown glyph alphabet `{}` (known: {})",
                opts.alphabet.as_deref().unwrap_or_default(),
                crate::glyph::ALPHABETS.join(", ")
            ),
        )
    })?;
    if view.x.kind != ScaleKind::Time {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/x/kind",
            "the horizontal scale must be `time`",
        ));
    }
    let panels = canonical_panels(view)?;
    let panel_ids: Vec<u32> = panels.iter().map(|p| p.id.unwrap_or_default()).collect();
    for (i, p) in panels.iter().enumerate() {
        if p.y.kind == ScaleKind::Time {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/y/kind"),
                "a panel scale is `linear` or `log`",
            ));
        }
    }
    let ids = layer_ids(&view.layers)?;
    for (l, id) in view.layers.iter().zip(&ids) {
        if !panel_ids.contains(&l.panel) {
            return Err(GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/panel",
                format!(
                    "panel {} does not exist (panels: {})",
                    l.panel,
                    panels
                        .iter()
                        .map(|p| match &p.name {
                            Some(n) => format!("{} ({n})", p.id.unwrap_or_default()),
                            None => p.id.unwrap_or_default().to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    let bar_series = first_bar_series(view, provider).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::NotFound,
            "/layers",
            "the view has no bar series (no candle, bar-series or study layer to take bar slots from)",
        )
    })?;
    let bars: Cow<'_, [qp_schema::Bar]> = provider.bars(&bar_series).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::NotFound,
            "/layers",
            format!("bar series `{bar_series}` has no bars"),
        )
    })?;
    if bars.is_empty() {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/layers",
            format!("bar series `{bar_series}` has no bars"),
        ));
    }
    // Slots after the last bar (point series, `docs/GRAMMAR.md` §3).
    let ext_times = ext_times(view, provider, bars[bars.len() - 1].t);
    let n_bars = bars.len() as u64;
    let total = n_bars + ext_times.len() as u64;

    let (wi, hi) = (width.cast_signed(), height.cast_signed());
    let (left, right, top, bottom) = (LEFT, wi - RIGHT_MARGIN, TOP, hi - BOTTOM_MARGIN);
    let mut fr = Frame {
        p: provider,
        theme,
        bar_series: bar_series.clone(),
        bars,
        ext_times,
        from: 0,
        to: total - 1,
        k: 1,
    };
    // Visible range: transient viewport, else the persistent range, else everything.
    if let Some((a, b)) = opts.range {
        let (a, b) = (a.min(b), a.max(b));
        if a >= total {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                "",
                format!(
                    "visible range starts at slot {a}, past the last slot {}",
                    total - 1
                ),
            ));
        }
        fr.from = a;
        fr.to = b.min(total - 1);
    } else if let Some([t0, t1]) = view.x.domain {
        #[allow(clippy::cast_precision_loss)]
        let slots: Vec<u64> = (0..total)
            .filter(|s| {
                fr.slot_time(*s)
                    .is_some_and(|t| (t as f64) >= t0 && (t as f64) <= t1)
            })
            .collect();
        match (slots.first(), slots.last()) {
            (Some(a), Some(b)) => {
                fr.from = *a;
                fr.to = *b;
            }
            _ => {
                return Err(GrammarError::new(
                    ErrorKind::Invalid,
                    "/x/domain",
                    format!("no bar falls in the range [{t0}, {t1}]"),
                ))
            }
        }
    }
    let count = fr.to - fr.from + 1;
    let plot_w = right - left;
    #[allow(clippy::cast_precision_loss)]
    let slot = f64::from(plot_w) / count as f64;
    let mut k = 1u64;
    if opts.lod && slot < 1.0 {
        k = count.div_ceil(u64::from(plot_w.unsigned_abs()));
    }
    if let Some(mp) = opts.max_points.filter(|m| *m > 0) {
        if count > mp {
            k = k.max(count.div_ceil(mp));
        }
    }
    fr.k = k;
    let lod = (k >= 2).then(|| Lod {
        bars_per_bucket: k,
        buckets: crate::frame::bucket_count(count, k),
    });
    let xmap = XMap {
        from_bar: fr.from,
        to_bar: fr.to,
        left: f64::from(left),
        slot,
    };

    // Prepare layers (palette counted per panel).
    let mut palettes = vec![0usize; panels.len()];
    let mut prepared = Vec::with_capacity(view.layers.len());
    for (l, id) in view.layers.iter().zip(&ids) {
        let pi = panel_ids.iter().position(|p| *p == l.panel).unwrap_or(0);
        prepared.push(prepare(&fr, l, id, &mut palettes[pi]).map_err(|e| e.with_layer(id))?);
    }

    // Panels: rectangles and value maps.
    let rects = panel_rects(&panels, left, right, top, bottom)?;
    let mut ymaps = Vec::with_capacity(panels.len());
    for (i, (p, rc)) in panels.iter().zip(&rects).enumerate() {
        let mut vals = Vec::new();
        for pl in prepared.iter().filter(|pl| pl.panel == panel_ids[i]) {
            domain_values(pl, &mut vals);
        }
        let (min, max) = domain(&p.y, &vals, i)?;
        ymaps.push(YMap {
            kind: p.y.kind,
            min,
            max,
            top: f64::from(rc.top),
            bottom: f64::from(rc.bottom),
        });
    }

    // Emphasis: persistent, then transient.
    let mut elist: Vec<(String, &Emphasis)> = Vec::new();
    for (i, e) in view.emphasis.iter().enumerate() {
        elist.push((format!("/emphasis/{i}"), e));
    }
    for (i, e) in opts.emphasis.iter().enumerate() {
        elist.push((format!("(options)/emphasis/{i}"), e));
    }
    let emph = resolve_emphasis(view, &elist, &prepared, &panel_ids, provider)?;
    let bold: BTreeSet<&str> = emph
        .iter()
        .filter(|e| e.kind == EmphasisKind::Highlight)
        .filter_map(|e| e.layer.as_deref())
        .collect();

    let theme = &fr.theme;
    let mut under: Vec<Prim> = Vec::new();
    let mut over: Vec<Prim> = Vec::new();
    let target_rects = |ps: &[u32]| -> Vec<PixelRect> {
        panel_ids
            .iter()
            .zip(&rects)
            .filter(|(id, _)| ps.is_empty() || ps.contains(id))
            .map(|(_, r)| *r)
            .collect()
    };
    let bar_px = |a: u64, b: u64| -> Option<(i32, i32)> {
        if b < fr.from || a > fr.to {
            return None;
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let x = |s: u64| (xmap.left + (s as f64 - fr.from as f64) * slot).floor() as i32;
        let x0 = x(a.max(fr.from)).clamp(left, right);
        let x1 = x(b.min(fr.to) + 1).clamp(left, right).max(x0 + 1);
        Some((x0, x1))
    };
    let strip = with_opacity(
        [theme.accent[0], theme.accent[1], theme.accent[2], 255],
        f64::from(STRIP_ALPHA) / 255.0,
    );
    for e in emph.iter().filter(|e| e.kind == EmphasisKind::Highlight) {
        let span = match e.bars {
            Some((a, b)) => bar_px(a, b),
            None if e.layer.is_none() && !e.panels.is_empty() => Some((left, right)),
            None => None,
        };
        if let Some((x0, x1)) = span {
            for rc in target_rects(&e.panels) {
                under.push(rect(x0, rc.top, x1 - x0, rc.bottom - rc.top, strip));
            }
        }
    }

    // Last-value tags: the first data layer of each panel (pixel row, value, colour).
    // Computed before the grid so a value label under a tag can be left out.
    let tags: Vec<Option<(i32, f64, Rgba)>> = rects
        .iter()
        .enumerate()
        .map(|(pi, rc)| {
            let pl = prepared
                .iter()
                .filter(|pl| pl.panel == panel_ids[pi] && pl.has_data)
                .find(|pl| {
                    matches!(
                        pl.geom,
                        Geom::Candle | Geom::Line | Geom::Area | Geom::Band | Geom::Bar
                    )
                })?;
            let (v, color) = last_value(pl)?;
            let ly = px(value_y(&ymaps[pi], v)?);
            (ly >= rc.top && ly <= rc.bottom).then_some((ly, v, color))
        })
        .collect();
    // A card panel (R3): every layer in it is panel-positioned (a capsule card's glyph,
    // sentence and ledger line). It has no value scale to show: no grid, value labels or
    // axis lines.
    let card: Vec<bool> = panel_ids
        .iter()
        .map(|pid| {
            let mut ls = prepared.iter().filter(|pl| pl.panel == *pid).peekable();
            ls.peek().is_some() && ls.all(|pl| pl.anchor.y.is_some())
        })
        .collect();
    // Horizontal grid and value labels (R0 order: line, label, line, label...).
    for (((rc, y), tag), _) in rects
        .iter()
        .zip(&ymaps)
        .zip(&tags)
        .zip(&card)
        .filter(|(_, c)| !**c)
    {
        let divs = (f64::from(rc.bottom - rc.top) / 90.0)
            .round()
            .clamp(2.0, 6.0);
        for v in ticks(y, divs) {
            let Some(yy) = value_y(y, v) else { continue };
            let yy = px(yy);
            if yy > rc.top && yy < rc.bottom {
                under.push(rect(rc.left, yy, rc.right - rc.left, 1, theme.grid));
                // The label's box (baseline yy + 4, LABEL_PX tall) under the tag's box
                // (ly - 8 .. ly + 8): the tag wins, the label is left out.
                #[allow(clippy::cast_possible_truncation)]
                let label_top = yy + 4 - LABEL_PX as i32;
                if tag.is_some_and(|(ly, _, _)| label_top < ly + 8 && yy + 4 > ly - 8) {
                    continue;
                }
                under.push(Prim::Text {
                    x: rc.right + 6,
                    baseline: yy + 4,
                    px: LABEL_PX,
                    text: format!("{v:.2}"),
                    color: theme.muted,
                    align: Align::Left,
                });
            }
        }
    }
    // Vertical grid and month labels (labels under the bottom panel only).
    let last_rect = rects[rects.len() - 1];
    let mut prev_month: Option<(i64, i64)> = None;
    let mut last_label_x = i32::MIN;
    for s in fr.from..=fr.to {
        let Some(t) = fr.slot_time(s) else { continue };
        let ym = year_month(t.div_euclid(86_400_000));
        if prev_month.is_some_and(|p| p != ym) {
            #[allow(clippy::cast_possible_truncation)]
            let x = crate::scale::slot_x(&xmap, s) as i32;
            if x.saturating_sub(last_label_x) >= 72 {
                for (rc, _) in rects.iter().zip(&card).filter(|(_, c)| !**c) {
                    under.push(rect(x, rc.top, 1, rc.bottom - rc.top, theme.grid));
                }
                under.push(Prim::Text {
                    x: x - 21,
                    baseline: last_rect.bottom + 16,
                    px: LABEL_PX,
                    text: format!("{:04}-{:02}", ym.0, ym.1),
                    color: theme.muted,
                    align: Align::Left,
                });
                last_label_x = x;
            }
        }
        prev_month = Some(ym);
    }
    // Axes.
    for (rc, _) in rects.iter().zip(&card).filter(|(_, c)| !**c) {
        under.push(rect(
            rc.right,
            rc.top,
            1,
            rc.bottom - rc.top + 1,
            theme.axis,
        ));
        under.push(rect(rc.left, rc.bottom, rc.right - rc.left, 1, theme.axis));
    }

    // Layers.
    let mut scene_panels: Vec<ScenePanel> = panels
        .iter()
        .zip(&rects)
        .zip(&ymaps)
        .map(|((p, rc), y)| ScenePanel {
            id: p.id.unwrap_or_default(),
            name: p.name.clone(),
            rect: *rc,
            y: *y,
            layers: Vec::new(),
        })
        .collect();
    for pl in &prepared {
        let pi = panel_ids.iter().position(|p| *p == pl.panel).unwrap_or(0);
        let g = Geo {
            x: &xmap,
            y: &ymaps[pi],
            rect: rects[pi],
            k,
            bold: bold.contains(pl.id.as_str()),
            theme,
            outcome,
            alphabet,
        };
        scene_panels[pi].layers.push(draw(pl, &g));
    }

    // Last-value tags.
    for (rc, tag) in rects.iter().zip(&tags) {
        let Some((ly, v, color)) = *tag else {
            continue;
        };
        let color = [color[0], color[1], color[2], 255];
        over.push(rect(rc.right + 1, ly - 8, wi - rc.right - 1, 16, color));
        over.push(Prim::Text {
            x: rc.right + 6,
            baseline: ly + 4,
            px: LABEL_PX,
            text: format!("{v:.2}"),
            color: theme.bg,
            align: Align::Left,
        });
    }
    // Title row.
    let title = view.title.clone().unwrap_or_else(|| {
        format!(
            "{} {}",
            provider.symbol(&bar_series).unwrap_or_default(),
            provider.interval(&bar_series).unwrap_or_default()
        )
    });
    // "C <close>" only when the first panel draws the bar series itself (its price).
    let show_close = prepared.iter().any(|pl| {
        pl.panel == panel_ids[0]
            && matches!(&pl.data, DataSource::Series { series } if *series == bar_series)
    });
    let title_x = left + 4;
    let close_x = left + 140;
    let title_room = if show_close {
        close_x - title_x - 8
    } else {
        wi - 8 - title_x
    };
    let title = fit_text(&title, title_room, f64::from(TITLE_PX));
    let mut text_right = title_x + est_width(&title, f64::from(TITLE_PX));
    over.push(Prim::Text {
        x: title_x,
        baseline: 23,
        px: TITLE_PX,
        text: title,
        color: theme.text,
        align: Align::Left,
    });
    if show_close {
        let last_bar = usize::try_from(fr.to.min(n_bars - 1)).unwrap_or(0);
        let lb = &fr.bars[last_bar];
        let text = format!("C {:.2}", lb.c);
        text_right = close_x + est_width(&text, f64::from(TITLE_PX));
        over.push(Prim::Text {
            x: close_x,
            baseline: 23,
            px: TITLE_PX,
            text,
            color: if lb.c >= lb.o { theme.up } else { theme.down },
            align: Align::Left,
        });
    }
    // Legends: panel 0 right-aligned in the title row; other panels top-left inside.
    for (pi, rc) in rects.iter().enumerate() {
        let legends: Vec<(String, Rgba)> = prepared
            .iter()
            .filter(|pl| pl.panel == panel_ids[pi])
            .filter(|pl| matches!(pl.geom, Geom::Line | Geom::Area | Geom::Band))
            .filter_map(|pl| {
                let label = pl.label.clone()?;
                let (v, color) = last_value(pl).unwrap_or((f64::NAN, pl.traces.first()?.base));
                let text = if pl.geom == Geom::Band || pl.traces.len() > 1 || !v.is_finite() {
                    label
                } else {
                    format!("{label} {v:.2}")
                };
                Some((text, pl.traces.first().map_or(color, |t| t.base)))
            })
            .collect();
        // Identical legend texts within a panel: the first one is kept.
        let mut seen = BTreeSet::new();
        let legends: Vec<(String, Rgba)> = legends
            .into_iter()
            .filter(|(t, _)| seen.insert(t.clone()))
            .collect();
        if pi == 0 {
            // Right-aligned from the right edge; the right-most are kept, and a legend
            // that would reach the title/close text is dropped with all left of it.
            let mut placed: Vec<(String, Rgba, i32)> = Vec::new();
            let mut x = wi - 8;
            for (text, color) in legends.into_iter().rev() {
                let w = est_width(&text, f64::from(LABEL_PX));
                if x - w < text_right + 12 {
                    break;
                }
                placed.push((text, color, x));
                x -= w + 12;
            }
            placed.reverse();
            for (text, color, x) in placed {
                over.push(Prim::Text {
                    x,
                    baseline: 23,
                    px: LABEL_PX,
                    text,
                    color,
                    align: Align::Right,
                });
            }
        } else {
            let mut x = rc.left + 4;
            for (text, color) in legends {
                let w = est_width(&text, f64::from(LABEL_PX));
                over.push(Prim::Text {
                    x,
                    baseline: rc.top + 12,
                    px: LABEL_PX,
                    text,
                    color,
                    align: Align::Left,
                });
                x += w + 12;
            }
        }
    }
    // Veils (focus, dim).
    let veil = [theme.bg[0], theme.bg[1], theme.bg[2], VEIL_ALPHA];
    let layer_prims = |id: &str| -> Vec<Prim> {
        scene_panels
            .iter()
            .flat_map(|p| p.layers.iter())
            .filter(|l| l.id == id)
            .flat_map(|l| l.prims.iter().cloned())
            .collect()
    };
    for e in emph.iter().filter(|e| e.kind != EmphasisKind::Highlight) {
        let all = |over: &mut Vec<Prim>, ps: &[u32]| {
            for rc in target_rects(ps) {
                over.push(rect(
                    rc.left,
                    rc.top,
                    rc.right - rc.left,
                    rc.bottom - rc.top,
                    veil,
                ));
            }
        };
        match (e.kind, &e.layer, e.bars) {
            (EmphasisKind::Focus, Some(l), _) => {
                all(&mut over, &[]);
                over.extend(layer_prims(l));
            }
            (EmphasisKind::Focus, None, bars) => {
                for (id, rc) in panel_ids.iter().zip(&rects) {
                    let target = e.panels.is_empty() || e.panels.contains(id);
                    match (target, bars.and_then(|(a, b)| bar_px(a, b))) {
                        (true, Some((x0, x1))) => {
                            over.push(rect(
                                rc.left,
                                rc.top,
                                x0 - rc.left,
                                rc.bottom - rc.top,
                                veil,
                            ));
                            over.push(rect(x1, rc.top, rc.right - x1, rc.bottom - rc.top, veil));
                        }
                        (true, None) if bars.is_none() => {}
                        _ => over.push(rect(
                            rc.left,
                            rc.top,
                            rc.right - rc.left,
                            rc.bottom - rc.top,
                            veil,
                        )),
                    }
                }
            }
            (EmphasisKind::Dim, Some(l), _) => {
                over.extend(layer_prims(l).into_iter().map(|p| recolor(p, veil)));
            }
            (EmphasisKind::Dim, None, Some((a, b))) => {
                if let Some((x0, x1)) = bar_px(a, b) {
                    for rc in target_rects(&e.panels) {
                        over.push(rect(x0, rc.top, x1 - x0, rc.bottom - rc.top, veil));
                    }
                }
            }
            (EmphasisKind::Dim, None, None) => all(&mut over, &e.panels),
            _ => {}
        }
    }

    Ok(SceneDesc {
        view: view.id.clone(),
        width,
        height,
        background: theme.bg,
        x: xmap,
        bars: n_bars,
        lod,
        panels: scene_panels,
        emphasis: emph
            .into_iter()
            .map(|e| SceneEmphasis {
                kind: e.kind,
                panels: e.panels,
                layer: e.layer,
                from_bar: e.bars.map(|b| b.0),
                to_bar: e.bars.map(|b| b.1),
            })
            .collect(),
        under,
        over,
    })
}

/// `text` cut to fit `room` pixels at `size` (estimate: 0.6 · size per char), ending in
/// "…" when cut.
fn fit_text(text: &str, room: i32, size: f64) -> String {
    if est_width(text, size) <= room {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut n = chars.len();
    while n > 0 {
        let mut t: String = chars[..n].iter().collect::<String>().trim_end().to_owned();
        t.push('…');
        if est_width(&t, size) <= room {
            return t;
        }
        n -= 1;
    }
    String::new()
}

/// Last drawn value of a layer and its colour (candle: close and direction colour).
fn last_value(pl: &Prepared) -> Option<(f64, Rgba)> {
    let t = pl.traces.first()?;
    let r = t
        .rows
        .iter()
        .rev()
        .find(|r| r.v.iter().any(Option::is_some))?;
    let v = match (pl.geom, t.lod_line) {
        (Geom::Candle, _) | (_, true) => r.v[3],
        _ => r.v[0],
    }?;
    let color = if pl.geom == Geom::Candle {
        r.color
    } else {
        t.base
    };
    Some((v, color))
}

fn recolor(p: Prim, c: Rgba) -> Prim {
    match p {
        Prim::Rect { x, y, w, h, .. } => Prim::Rect {
            x,
            y,
            w,
            h,
            color: c,
        },
        Prim::Polyline { points, width, .. } => Prim::Polyline {
            points,
            width,
            color: c,
        },
        Prim::Fill { points, .. } => Prim::Fill { points, color: c },
        Prim::Circle { cx, cy, r, .. } => Prim::Circle {
            cx,
            cy,
            r,
            color: c,
        },
        Prim::Text {
            x,
            baseline,
            px,
            text,
            align,
            ..
        } => Prim::Text {
            x,
            baseline,
            px,
            text,
            color: c,
            align,
        },
    }
}
