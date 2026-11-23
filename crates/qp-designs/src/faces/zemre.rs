//! (R5/M16 `zemre`) The face of the "time strip" direction
//! (`rounds/R5/experiments/M16/zemre/DESIGN.md`).
//!
//! Two stacked panels:
//!
//! - **`card`** (panel-positioned, no scale): the glyph at the top left, the headline sentence
//!   beside it (`headline` role, up to four lines), the track-record sentence under both
//!   (`label` role, `muted`, two lines).
//! - **`life`** (a time strip): the bars of the claim's **natural life window** — its
//!   horizon (`Story::horizon_bars`) plus a quarter of it before birth, or, once the claim
//!   is older than its horizon, its whole age plus that quarter — with the close price
//!   (`muted`) and a vertical rule at the birth bar (`born`, origin: the fact's `bar`). The
//!   evidence geometry is left to the open capsule and the chart: a level's rule would cross
//!   the strip's legend.
//!   The right edge is now. Because the window is measured in the claim's own life, the
//!   birth rule's place reads the same on every face: at the right edge a claim born now,
//!   at a fifth of the width one at the end of its natural life, further left one past it
//!   (`birth_fraction`). Every face's right gutter holds the price now (the strip's last
//!   value tag), so the feed has a column for "now".
//!
//! A finding without a story, without bars or without facts falls back to the capsule card
//! ([`qp_grammar::depth::capsule_view_in`]). Sizes and gaps come from the look.

use std::collections::BTreeMap;

use qp_schema::grammar::{
    ColorSpec, Layer, Mark, Origin, Panel, PanelPos, Position, Scale, ScaleKind, SizeSpec, View,
};

use qp_grammar::provider::{DataProvider, HeadlineParts, SeriesKind};
use qp_look::error::GrammarError;
use qp_look::look::Look;

/// Headline lines the card holds beside the glyph.
const HEADLINE_LINES: f64 = 4.0;
/// Lines of the track-record sentence.
const LEDGER_LINES: f64 = 2.0;
/// Height of the life strip (CSS px).
const STRIP_PX: f64 = 58.0;
/// Gap between the glyph/headline block and the track-record sentence.
const LEDGER_GAP: f64 = 8.0;
/// Bars before birth in the window, as a share of the horizon.
const LEAD: f64 = 0.25;
/// Horizon used when a story has none.
const DEFAULT_HORIZON: u32 = 20;

fn line_px(look: &Look, role: &qp_look::look::TypeRole) -> f64 {
    let _ = look;
    (role.px * role.line).round()
}

/// Height of the `card` panel in `look`.
fn card_px(look: &Look) -> f64 {
    let head = HEADLINE_LINES * line_px(look, &look.types.headline);
    head.max(look.glyph.face_px) + LEDGER_GAP + LEDGER_LINES * line_px(look, &look.types.label)
}

/// The face height in `look` (CSS px at the phone width).
#[must_use]
pub fn height(look: &Look) -> u32 {
    let plot = look.space.plot;
    let h = f64::from(plot[2])
        + card_px(look)
        + f64::from(look.space.panel_gap)
        + STRIP_PX
        + f64::from(plot[3]);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let h = h.ceil() as u32;
    h.clamp(120, 844)
}

/// The window of the life strip: `(first bar, bars)` for a claim born at bar `born` with
/// natural life `horizon` bars, `now` the last bar. The window ends at `now`; it holds the
/// horizon plus a lead of a quarter of it, or the claim's age plus that lead once the claim
/// is older than its horizon; it starts at bar 0 at the earliest.
#[must_use]
pub fn window(born: u64, horizon: u32, now: u64) -> (u64, u64) {
    let h = u64::from(if horizon == 0 {
        DEFAULT_HORIZON
    } else {
        horizon
    });
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let lead = (h as f64 * LEAD).ceil() as u64;
    let age = now.saturating_sub(born);
    let len = (h + lead).max(age + lead);
    let first = now.saturating_sub(len);
    (first, now - first + 1)
}

/// Where the birth bar sits in the window, as a fraction of its width from the left
/// (`0` left edge, `1` right edge = now): the channel the strip adds (DESIGN.md "Yaş"). The
/// compiler places the birth rule by the window and the bar itself; this is the same
/// quantity, for the tests and the measurement.
#[cfg(test)]
#[must_use]
pub fn birth_fraction(born: u64, horizon: u32, now: u64) -> f64 {
    let (first, len) = window(born, horizon, now);
    #[allow(clippy::cast_precision_loss)]
    let f = (born.clamp(first, now) - first) as f64 / (len.max(2) - 1) as f64;
    f
}

/// The headline template (without a voice): instrument, event, when and whether it still
/// holds, from the reader's pieces; at most 80 characters (the "when" goes first).
#[must_use]
pub fn headline(hp: &HeadlineParts) -> String {
    let full = format!(
        "{}: {} · {}, {}",
        hp.instrument, hp.event, hp.when, hp.validity_phrase
    );
    if full.chars().count() <= 80 {
        return full;
    }
    let short = format!("{}: {} · {}", hp.instrument, hp.event, hp.validity_phrase);
    if short.chars().count() <= 80 {
        return short;
    }
    short.chars().take(79).collect::<String>() + "…"
}

fn layer(id: &str, mark: Mark, panel: u32) -> Layer {
    let mut l = Layer::new(mark, panel);
    l.id = Some(id.to_owned());
    l
}

fn at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

fn panel(id: u32, name: &str, weight: f64) -> Panel {
    Panel {
        id: Some(id),
        name: Some(name.to_owned()),
        weight: Some(weight),
        y: Scale {
            kind: ScaleKind::Linear,
            domain: None,
            id: Some(format!("y{id}")),
        },
    }
}

#[allow(clippy::too_many_lines)]
/// The `zemre` face of `finding`.
///
/// # Errors
/// As the capsule card's: an unknown finding or fact, a series without bars.
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let fallback = || qp_grammar::depth::capsule_view_in(look, finding, max_points, p, id);
    let Some(story) = p.capsule(finding).and_then(|c| c.story.clone()) else {
        return fallback();
    };
    let Some(fact) = p
        .finding(finding)
        .and_then(|f| f.facts.first().cloned())
        .and_then(|fid| p.fact(&fid).map(std::borrow::Cow::into_owned))
    else {
        return fallback();
    };
    let bar_series = match p.series_kind(&fact.series) {
        Some(SeriesKind::Bars) => Some(fact.series.clone()),
        Some(SeriesKind::Values) => p.input_bars(&fact.series),
        _ => None,
    };
    let Some(bs) = bar_series else {
        return fallback();
    };
    let Some(bars) = p.bars(&bs).filter(|b| !b.is_empty()) else {
        return fallback();
    };
    let now = bars.len() as u64 - 1;
    if fact.bar > now {
        return fallback();
    }
    let (first, _) = window(fact.bar, story.horizon_bars, now);
    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    let x = Scale {
        kind: ScaleKind::Time,
        domain: Some([bars[i(first)].t as f64, bars[i(now)].t as f64]),
        id: Some("x".to_owned()),
    };

    // Panel 0, `card`: glyph, headline, track record.
    let g = look.glyph.face_px;
    let mut glyph = layer("glyph", Mark::Glyph, 0);
    glyph.finding = Some(finding.to_owned());
    glyph.encoding.x = Some(at(0.0, 0.0));
    glyph.encoding.y = Some(at(0.0, 0.0));
    glyph.encoding.size = Some(SizeSpec::Constant(g));
    let mut layers = vec![glyph];
    let voiced = p.voice(finding).is_some();
    let has_headline = voiced
        || qp_grammar::designs::headline_text(Some(headline), finding, p).is_some()
        || p.headline(finding).is_some();
    if has_headline {
        let mut h = layer("headline", Mark::Text, 0);
        h.finding = Some(finding.to_owned());
        h.encoding.text = Some("headline".to_owned());
        h.encoding.x = Some(at(0.0, g + look.space.glyph_gap));
        h.encoding.y = Some(at(0.0, 0.0));
        h.encoding.size = Some(SizeSpec::Constant(look.types.headline.px));
        layers.push(h);
    }
    let head = (HEADLINE_LINES * line_px(look, &look.types.headline)).max(g);
    let mut ledger = layer("ledger", Mark::Text, 0);
    ledger.finding = Some(finding.to_owned());
    ledger.encoding.text = Some("ledger".to_owned());
    ledger.encoding.x = Some(at(0.0, 0.0));
    ledger.encoding.y = Some(at(0.0, head + LEDGER_GAP));
    ledger.encoding.size = Some(SizeSpec::Constant(look.types.label.px));
    ledger.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));
    layers.push(ledger);

    // Panel 1, `life`: the natural life window.
    let mut price = layer("price", Mark::Line, 1);
    price.series = Some(bs);
    price.encoding.y = Some(Position::Name("c".to_owned()));
    price.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));
    price.encoding.width = Some(1.5);
    // The strip's legend: the close, as on the capsule card ("C …").
    price.label = Some("C".to_owned());
    layers.push(price);
    let mut born = layer("born", Mark::Rule, 1);
    born.origin = BTreeMap::from([(
        "/encoding/x".to_owned(),
        Origin::Fact {
            fact: fact.id.clone(),
            field: "bar".to_owned(),
        },
    )]);
    born.encoding.color = Some(ColorSpec::Constant("text".to_owned()));
    born.encoding.width = Some(2.0);
    layers.push(born);

    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        theme: None,
        x,
        y: Vec::new(),
        panels: vec![panel(0, "card", card_px(look)), panel(1, "life", STRIP_PX)],
        layers,
        emphasis: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_holds_the_horizon_and_follows_the_age() {
        // Born now: the window is the horizon plus a quarter, birth at the right edge.
        assert_eq!(window(100, 20, 100), (75, 26));
        assert!((birth_fraction(100, 20, 100) - 1.0).abs() < 1e-12);
        // Halfway through its life.
        let f = birth_fraction(90, 20, 100);
        assert!(f > 0.55 && f < 0.65, "{f}");
        // At the end of its natural life: a fifth of the width (the lead).
        let f = birth_fraction(80, 20, 100);
        assert!((f - 0.2).abs() < 1e-12, "{f}");
        // Past it: further left, never out of the window.
        let f = birth_fraction(40, 20, 100);
        assert!(f > 0.0 && f < 0.1, "{f}");
        // The series start clamps the window.
        assert_eq!(window(2, 20, 5), (0, 6));
        // No horizon: the default.
        assert_eq!(window(100, 0, 100), (75, 26));
    }

    #[test]
    fn birth_moves_left_as_the_claim_ages() {
        let mut prev = 2.0;
        for age in 0..60 {
            let f = birth_fraction(1000 - age, 12, 1000);
            assert!(f < prev, "age {age}: {f} !< {prev}");
            prev = f;
        }
    }

    #[test]
    fn heights_fit_the_phone() {
        for l in qp_look::look::LOOKS {
            let h = height(l);
            assert!((120..=844).contains(&h), "{}: {h}", l.name);
        }
    }
}
