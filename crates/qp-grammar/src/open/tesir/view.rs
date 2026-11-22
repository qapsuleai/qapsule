//! `tesir`'s scene ([`view`]).

use super::*;

// ---------------------------------------------------------------------------
// The scene
// ---------------------------------------------------------------------------

/// A finding text layer at a panel position, wrapping at `edge` px past the panel's right
/// (the plot's right margin holds no value scale in a card panel).
#[allow(clippy::too_many_arguments)]
fn text(
    id: &str,
    field: &str,
    finding: &str,
    panel: u32,
    (x, y): (f64, f64),
    size: f64,
    color: Option<&str>,
    edge: f64,
) -> Layer {
    let mut l = layer(id, Mark::Text, panel);
    l.finding = Some(finding.to_owned());
    l.encoding.text = Some(field.to_owned());
    l.encoding.x = Some(panel_at(0.0, x));
    l.encoding.x2 = Some(panel_at(1.0, edge));
    l.encoding.y = Some(panel_at(0.0, y));
    l.encoding.size = Some(SizeSpec::Constant(size));
    l.encoding.color = color.map(|c| ColorSpec::Constant(c.to_owned()));
    l
}

fn trust_role(c: Option<DeviceCheck>) -> &'static str {
    match c {
        Some(DeviceCheck::Device) => "valid",
        Some(DeviceCheck::Checking) => "muted",
        Some(DeviceCheck::Differs) => "warn",
        _ => "accent",
    }
}

/// The capsule depth's view (see the module docs). A finding without a story (R2) or
/// without a meaning keeps today's view.
///
/// # Errors
/// As [`depth::capsule_view_in`].
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let s = match src(finding, p) {
        Some(s) if depth::has_story(finding, p) && meant(&s.facts).is_some() => s,
        _ => return depth::capsule_view_in(look, finding, max_points, p, id),
    };
    let (_, fact) = depth::finding_and_fact(finding, p)?;
    let (bs, bars) = depth::bar_series_bars(&fact, p)?;
    let n = bars.len() as u64;
    let sl = slots(look);
    let t = &look.types;
    let [m_left, m_right, _, _] = look.space.plot;
    let edge = f64::from(m_right - m_left);
    let mut layers: Vec<Layer> = Vec::new();

    // 0 — the claim.
    let mut glyph = layer("glyph", Mark::Glyph, 0);
    glyph.finding = Some(finding.to_owned());
    glyph.encoding.x = Some(panel_at(0.0, 0.0));
    glyph.encoding.y = Some(panel_at(0.0, 0.0));
    glyph.encoding.size = Some(SizeSpec::Constant(look.glyph.card_px));
    layers.push(glyph);
    let valid = p.glyph(finding).and_then(|g| g.validity).map_or_else(
        || {
            p.capsule(finding)
                .and_then(|c| c.story.as_ref().map(|st| !st.stage.is_closed()))
                .unwrap_or(true)
        },
        |v| v == Validity::Valid,
    );
    layers.push(text(
        "headline",
        "headline",
        finding,
        0,
        (look.glyph.card_px + look.space.glyph_gap, 0.0),
        t.headline.px,
        (!valid).then_some(NO_LONGER_HEADLINE),
        edge,
    ));

    // 1 — what it rests on.
    let row1 = sl.sec + lh(&t.label) + sl.tight;
    let row2 = row1 + lh(&t.title) + sl.tight;
    layers.push(text(
        "open-h-basis",
        "open.h.basis",
        finding,
        1,
        (0.0, sl.sec),
        t.label.px,
        Some("muted"),
        edge,
    ));
    let lock = meant(&s.facts).is_some_and(locked);
    layers.push(text(
        "open-basis",
        "open.basis",
        finding,
        1,
        (0.0, row1),
        t.title.px,
        lock.then_some(look.evidence.note_color),
        edge,
    ));
    layers.push(text(
        "open-basis2",
        "open.basis2",
        finding,
        1,
        (0.0, row2),
        t.label.px,
        Some("muted"),
        edge,
    ));

    // 2 — the evidence in small: the line the evidence sits on and its geometry (by
    // origin only: a redacted reader gets the line alone).
    let mut taken: Vec<String> = layers.iter().filter_map(|l| l.id.clone()).collect();
    let on_bars = p.series_kind(&fact.series) == Some(SeriesKind::Bars);
    let mut line = layer(&uniq(&mut taken, "line"), Mark::Line, 2);
    if on_bars {
        line.series = Some(bs.clone());
        line.encoding.y = Some(Position::Name("c".to_owned()));
    } else {
        line.series = Some(fact.series.clone());
        if let Some(f) = depth::main_field(&fact.series, p) {
            line.encoding.y = Some(Position::Name(f));
        }
    }
    layers.push(line);
    let (mut first, mut last) = (fact.bar, fact.bar);
    for f in &s.facts {
        if f.series != fact.series {
            continue;
        }
        last = last.max(f.bar);
        for g in f.meaning.iter().flat_map(|m| &m.geometry) {
            let (a, b) = match g {
                Geometry::Level {
                    from_bar, to_bar, ..
                }
                | Geometry::Band {
                    from_bar, to_bar, ..
                } => (*from_bar, *to_bar),
                Geometry::Points { points } => (
                    points.iter().map(|q| q.bar).min().unwrap_or(f.bar),
                    points.iter().map(|q| q.bar).max().unwrap_or(f.bar),
                ),
            };
            first = first.min(a);
            last = last.max(b);
        }
        for l in depth::evidence_layers(look, f, 2, &mut taken) {
            layers.push(l);
        }
    }
    if on_bars && !lock {
        let mut point = layer(&uniq(&mut taken, "fact"), Mark::Point, 2);
        point.fact = Some(fact.id.clone());
        point.encoding.y = Some(Position::Name("value".to_owned()));
        point.encoding.color = Some(ColorSpec::Constant(look.evidence.color.to_owned()));
        layers.push(point);
    }
    // The evidence's own window: from a few bars before it starts to a few bars after it
    // ends (at least the chart's look-back), not to the live bar — the picture of what the
    // claim rests on does not move with every tick (a repaint of the tall column while it
    // scrolls costs long frames, K52; where things stand now is the detail's and the
    // chart's).
    let _ = max_points;
    // The newest bar may still be forming (a live tick moves it): left out unless the
    // evidence itself reaches it.
    let to = last
        .saturating_add(look.open.thumb_lead)
        .min(n.saturating_sub(2).max(last))
        .min(n - 1);
    let from = first
        .saturating_sub(look.open.thumb_lead)
        .min(to.saturating_sub(depth::CHART_LOOKBACK));

    // 3 — why, and where things stand (under the thumbnail's month labels).
    layers.push(text(
        "open-why",
        "open.why",
        finding,
        3,
        (0.0, sl.month),
        t.body.px,
        None,
        edge,
    ));

    // 4 — who verified it.
    layers.push(text(
        "open-h-trust",
        "open.h.trust",
        finding,
        4,
        (0.0, sl.sec),
        t.label.px,
        Some("muted"),
        edge,
    ));
    layers.push(text(
        TRUST_LAYER,
        "open.trust",
        finding,
        4,
        (0.0, row1),
        t.title.px,
        Some(trust_role(s.check)),
        edge,
    ));
    layers.push(text(
        "open-trust2",
        "open.trust2",
        finding,
        4,
        (0.0, row2),
        t.label.px,
        Some("muted"),
        edge,
    ));

    // 5 — the track record.
    layers.push(text(
        "open-h-ledger",
        "open.h.ledger",
        finding,
        5,
        (0.0, sl.sec),
        t.label.px,
        Some("muted"),
        edge,
    ));
    layers.push(text(
        "ledger",
        "ledger",
        finding,
        5,
        (0.0, row1),
        t.body.px,
        None,
        edge,
    ));

    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    let names = ["claim", "basis", "evidence", "why", "trust", "record"];
    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        theme: None,
        x: depth::time_scale(bars[i(from)].t, bars[i(to)].t),
        y: Vec::new(),
        panels: names
            .iter()
            .zip(sl.h)
            .zip(0u32..)
            .map(|((nm, h), id)| named_panel(id, nm, h))
            .collect(),
        layers,
        emphasis: Vec::new(),
    })
}
