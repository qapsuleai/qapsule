//! R3 (`docs/GRAMMAR.md` §18): the capsule card, evidence geometry and the glyph's
//! parameters.

use super::*;

// ---------------------------------------------------------------------------
// R3: capsule card, evidence geometry, glyph parameters (`docs/GRAMMAR.md` §18)
// ---------------------------------------------------------------------------

/// Closed ledger rows a story needs at birth before its glyph shows a confidence fill
/// (the M2 template's ledger-sentence threshold).
pub const GLYPH_MIN_LEDGER_N: u32 = qp_schema::params::system::GLYPH_MIN_LEDGER_N_DEFAULT;

/// Bars after the last transition at which a glyph's freshness is one half.
pub const FRESHNESS_HALF_BARS: f64 = qp_schema::params::system::GLYPH_FRESHNESS_HALF_BARS_DEFAULT;

/// Glyph size in the capsule card (the `r4` look's `glyph.card_px`).
pub const CARD_GLYPH_PX: f64 = qp_look::look::R4.glyph.card_px;
/// Glyph size over the chart at `chart` depth (the `r4` look's `glyph.chart_px`).
pub const CHART_GLYPH_PX: f64 = qp_look::look::R4.glyph.chart_px;

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
pub fn has_story(finding: &str, p: &dyn DataProvider) -> bool {
    p.capsule(finding).is_some_and(|c| c.story.is_some())
}

pub fn panel_at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

pub fn named_panel(id: u32, name: &str, weight: f64) -> Panel {
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
    capsule_card_view_in(&qp_look::look::R4, finding, max_points, p, id)
}

/// (R5/T3) [`capsule_card_view`] with the sizes of `look`: glyph `glyph.card_px`, the
/// sentence in the `body` role beside it (`space.glyph_gap`), the ledger line in the
/// `label` role.
///
/// (R5/m17-int, `docs/LANGUAGE.md`) Under a voice ([`DataProvider::voice`]) the card is the
/// open capsule's voice: the `sentence` layer writes the voice's **detail** (the headline
/// is the open layer's title, and the face's), the `ledger` layer its track-record
/// sentence in a two-line band at the card's bottom, and the card panel takes four fifths
/// of the height (the mini price line one fifth).
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
    let voiced = p.voice(finding).is_some();
    let mut sentence = layer("sentence", Mark::Text, 0);
    sentence.finding = Some(finding.to_owned());
    sentence.encoding.text = Some(if voiced { "detail" } else { "headline" }.to_owned());
    sentence.encoding.x = Some(panel_at(0.0, card_px + look.space.glyph_gap));
    sentence.encoding.y = Some(panel_at(0.0, 0.0));
    sentence.encoding.size = Some(SizeSpec::Constant(look.types.body.px));
    let mut ledger = layer("ledger", Mark::Text, 0);
    ledger.finding = Some(finding.to_owned());
    ledger.encoding.text = Some("ledger".to_owned());
    ledger.encoding.x = Some(panel_at(0.0, 0.0));
    // The ledger line's top: its size plus the look's gap (`r4`: 2 px) above the panel's
    // bottom. (R5/m17-int) Under a voice, a sentence: a band of two lines.
    let band = if voiced {
        2.0 * (look.types.label.px * look.types.label.line).round()
    } else {
        look.types.label.px
    };
    ledger.encoding.y = Some(panel_at(1.0, -(band + look.offsets.ledger_gap)));
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
        panels: if voiced {
            vec![named_panel(0, "card", 4.0), named_panel(1, "price", 1.0)]
        } else {
            vec![named_panel(0, "card", 3.0), named_panel(1, "price", 2.0)]
        },
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
pub fn uniq(taken: &mut Vec<String>, base: &str) -> String {
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
/// the chart says what the evidence needs instead ([`evidence_note`]). Colour, widths, the
/// band's opacity and the point size come from the look (`look.evidence`).
#[must_use]
pub fn evidence_layers(
    look: &Look,
    fact: &Fact,
    panel: u32,
    taken: &mut Vec<String>,
) -> Vec<Layer> {
    let mut out = Vec::new();
    let Some(m) = &fact.meaning else {
        return out;
    };
    let ev = |k: &str| fact.evidence.get(k).copied();
    let ev_look = &look.evidence;
    let accent = || Some(ColorSpec::Constant(ev_look.color.to_owned()));
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
                l.encoding.width = Some(ev_look.rule_w);
                out.push(l);
            }
            Geometry::Band { upper, lower, .. } => {
                let mut l = layer(&uniq(taken, "band"), Mark::Band, panel);
                bind(&mut l, "/encoding/y", fact, upper);
                bind(&mut l, "/encoding/y2", fact, lower);
                bind(&mut l, "/encoding/x", fact, &gf("from_bar"));
                bind(&mut l, "/encoding/x2", fact, &gf("to_bar"));
                l.encoding.color = accent();
                l.encoding.opacity = Some(ev_look.band_alpha);
                l.encoding.width = Some(ev_look.band_w);
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
                    l.encoding.width = Some(ev_look.rule_w);
                    out.push(l);
                }
                for (j, pt) in points.iter().enumerate() {
                    let mut l = layer(&uniq(taken, "swing-point"), Mark::Point, panel);
                    bind(&mut l, "/encoding/y", fact, &pt.key);
                    bind(&mut l, "/encoding/x", fact, &gf(&format!("points.{j}.bar")));
                    l.encoding.color = accent();
                    l.encoding.size = Some(SizeSpec::Constant(ev_look.point_px));
                    out.push(l);
                }
            }
        }
    }
    out
}

#[allow(clippy::too_many_lines)]
/// The R3 chart view: the R2 chart rule plus, in the candle panel, the evidence geometry
/// of every fact of the finding (in order), the capsule's glyph over the first fact's bar
/// (when a capsule with a story presents the finding), and, when a geometry's values are
/// absent (redacted), a note (`evidence`) saying which data entitlement the evidence
/// needs. The visible range starts 30 bars before the first fact or at the first
/// geometry bar, whichever is earlier.
pub(super) fn chart_view_r3(
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
            for l in evidence_layers(look, f, 0, &mut taken) {
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
        t.encoding.x = Some(panel_at(0.0, look.offsets.note_x));
        t.encoding.y = Some(panel_at(0.0, look.glyph.chart_px + look.offsets.note_gap));
        t.encoding.size = Some(SizeSpec::Constant(look.types.body.px));
        t.encoding.color = Some(ColorSpec::Constant(look.evidence.note_color.to_owned()));
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

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn card_glyph_constants_bind_registry() {
        assert_eq!(super::GLYPH_MIN_LEDGER_N, 20);
        assert_eq!(super::FRESHNESS_HALF_BARS.to_bits(), 5.0f64.to_bits());
    }
}
