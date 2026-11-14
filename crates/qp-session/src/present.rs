//! The deterministic presenter (`docs/PROTOCOL.md` §10.2, §10.3).

use qp_schema::protocol::{Presentation, Profile, SeriesValues, SurfaceKind};
use qp_schema::{Depth, Document, Finding, Layer, Mark, Scale, ScaleKind, View};

use crate::error::num;
use crate::values::{bar_series, values, Cache};

/// Profile for a surface kind (§10.1), optionally capped at `max_depth`.
#[must_use]
pub fn profile_for(kind: SurfaceKind, max_depth: Option<Depth>) -> Profile {
    let (depth, series_data, max_points, max_glyphs, speech) = match kind {
        SurfaceKind::Desktop => (Depth::Chart, true, 500, 6, false),
        SurfaceKind::Phone => (Depth::Capsule, true, 60, 3, false),
        SurfaceKind::Watch => (Depth::Sentence, false, 0, 1, false),
        SurfaceKind::Voice => (Depth::Sentence, false, 0, 0, true),
        SurfaceKind::Headless => (Depth::Chart, true, 10000, 6, false),
    };
    let depth = match max_depth {
        Some(m) if rank(m) < rank(depth) => m,
        _ => depth,
    };
    Profile {
        depth,
        series_data,
        max_points,
        max_glyphs,
        speech,
    }
}

pub(crate) fn rank(d: Depth) -> u8 {
    match d {
        Depth::Sentence => 0,
        Depth::Capsule => 1,
        Depth::Chart => 2,
        Depth::Workspace => 3,
    }
}

/// The view the presenter builds for a finding: panel 0 candles of the first fact's bar
/// series; if the fact is on a study series, panel 1 its line and a rule at
/// `evidence.level`; `x.domain = [t(max(0, bar - 30)), t(last bar)]`; linear y scales.
///
/// # Errors
/// The finding has no facts, or its first fact's series has no bars.
pub(crate) fn presenter_view(doc: &Document, finding: &Finding, id: &str) -> Result<View, String> {
    let fact_id = finding
        .facts
        .first()
        .ok_or_else(|| format!("finding `{}` has no facts to present", finding.id))?;
    let fact = doc
        .facts
        .iter()
        .find(|f| &f.id == fact_id)
        .ok_or_else(|| format!("fact `{fact_id}` does not exist"))?;
    let bs = bar_series(doc, &fact.series)
        .ok_or_else(|| format!("series `{}` has no bars", fact.series))?;
    let bars = bs.bars().unwrap_or_default();
    let last = bars
        .last()
        .ok_or_else(|| format!("series `{}` has no bars", bs.id))?;
    let from = usize::try_from(fact.bar.saturating_sub(30))
        .unwrap_or(0)
        .min(bars.len() - 1);
    let mut candles = Layer::new(Mark::Candle, 0);
    candles.series = Some(bs.id.clone());
    let mut layers = vec![candles];
    let mut panels = 1;
    if fact.series != bs.id {
        panels = 2;
        let mut line = Layer::new(Mark::Line, 1);
        line.series = Some(fact.series.clone());
        layers.push(line);
        if let Some(level) = fact.evidence.get("level") {
            let mut rule = Layer::new(Mark::Rule, 1);
            rule.encoding.y = Some(qp_schema::grammar::Position::Name(num(*level)));
            layers.push(rule);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let domain = [bars[from].t as f64, last.t as f64];
    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        x: Scale {
            domain: Some(domain),
            ..Scale::new(ScaleKind::Time)
        },
        y: (0..panels).map(|_| Scale::new(ScaleKind::Linear)).collect(),
        layers,
        theme: None,
        panels: Vec::new(),
        emphasis: Vec::new(),
    })
}

/// Presents a finding for a profile.
pub(crate) fn present(
    doc: &Document,
    cache: &mut Cache,
    finding: &Finding,
    surface: SurfaceKind,
    profile: &Profile,
) -> Presentation {
    let capsule = doc.capsules.iter().find(|c| c.finding == finding.id);
    let depth = if rank(profile.depth) > rank(Depth::Chart) {
        Depth::Chart
    } else {
        profile.depth
    };
    let mut p = Presentation {
        finding: finding.id.clone(),
        depth,
        surface,
        text: capsule.map_or_else(|| finding.claim.clone(), |c| c.headline.clone()),
        state: capsule.map(|c| c.state),
        glyphs: Vec::new(),
        confidence: None,
        spark: None,
        view: None,
        facts: Vec::new(),
    };
    if rank(depth) >= rank(Depth::Capsule) {
        if let Some(c) = capsule {
            p.glyphs = c
                .glyphs
                .iter()
                .take(profile.max_glyphs as usize)
                .cloned()
                .collect();
        }
        p.confidence = Some(finding.confidence);
        if profile.series_data && profile.max_points > 0 {
            p.spark = spark(doc, cache, finding, profile.max_points as usize);
        }
    }
    if rank(depth) >= rank(Depth::Chart) {
        p.view = capsule
            .and_then(|c| c.view.as_ref())
            .and_then(|v| doc.views.iter().find(|x| &x.id == v).cloned())
            .or_else(|| presenter_view(doc, finding, &format!("present-{}", finding.id)).ok());
        p.facts = finding
            .facts
            .iter()
            .filter_map(|f| doc.facts.iter().find(|x| &x.id == f).cloned())
            .collect();
    }
    p
}

/// The last `max_points` values of the main output of the finding's first fact's series.
fn spark(
    doc: &Document,
    cache: &mut Cache,
    finding: &Finding,
    max_points: usize,
) -> Option<SeriesValues> {
    let fact = doc
        .facts
        .iter()
        .find(|f| Some(&f.id) == finding.facts.first())?;
    let v = values(doc, cache, &fact.series).ok()?;
    let n = v.len();
    let from = n.saturating_sub(max_points);
    let main = v.main_values();
    Some(SeriesValues {
        series: fact.series.clone(),
        from_bar: from as u64,
        t: v.t[from..].to_vec(),
        outputs: [(v.main.clone(), main[from..].to_vec())]
            .into_iter()
            .collect(),
    })
}
