//! The deterministic presenter (`docs/PROTOCOL.md` §10.2, §10.3): the authority applies
//! the grammar's depth rules (`qp_grammar::depth`, `docs/GRAMMAR.md` §13).

use qp_grammar::depth;
use qp_schema::protocol::{Presentation, Profile, SeriesValues, SurfaceKind};
use qp_schema::{Depth, Document, Finding, View};

use crate::provider::DocProvider;
use qp_plane::values::{values, Cache};

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

/// The view the presenter builds for a finding: the grammar's `chart` depth rule
/// ([`qp_grammar::depth::chart_view`], `docs/GRAMMAR.md` §13): panel 0 candles of the
/// first fact's bar series; if the fact is on a study series, panel 1 its line and a rule
/// at `evidence.level` whose origin is the fact (`y` keeps the `qp/0` string form,
/// `"70"`); `x.domain = [t(max(0, bar - 30)), t(last bar)]`; a `highlight` emphasis on
/// the fact; linear value scales. Layer ids are the grammar's own names (`candles`,
/// `line`, `level`; `docs/GRAMMAR.md` §11, K25), as in the capsule view.
///
/// `doc` is the command's scratch document; a bound bar series' bars come from the data
/// plane's reading document (`reader`, §18), as for `add_fact` and `set_view_range`.
///
/// # Errors
/// The finding has no facts, or its first fact's series has no bars.
pub(crate) fn presenter_view(
    doc: &Document,
    reader: Option<qp_plane::plane::Reader<'_>>,
    finding: &Finding,
    id: &str,
) -> Result<View, String> {
    // The chart rule reads no study values: an empty cache is enough.
    let cache = Cache::default();
    let p = DocProvider::new(doc, &cache).with_reader(reader);
    depth::chart_view(&finding.id, &p, id).map_err(|e| e.reason)
}

/// Presents a finding for a profile with the grammar's depth rules
/// ([`qp_grammar::depth`]): `sentence` → text (+ capsule state); `capsule` → + glyphs,
/// confidence, spark and `capsule_view` (one panel; only if the profile allows series
/// data); `chart` → + the capsule's stored view or the presenter's view, and the facts.
pub fn present(
    doc: &Document,
    cache: &mut Cache,
    finding: &Finding,
    surface: SurfaceKind,
    profile: &Profile,
    headlines: &std::collections::BTreeMap<String, String>,
) -> Presentation {
    let capsule = doc.capsules.iter().find(|c| c.finding == finding.id);
    let depth = depth::effective_depth(profile.depth, profile);
    let series_data = profile.series_data && profile.max_points > 0;
    let spark = if rank(depth) >= rank(Depth::Capsule) && series_data {
        spark(doc, cache, finding, profile.max_points as usize)
    } else {
        None
    };
    let prov = DocProvider::new(doc, cache).with_headlines(headlines);
    let mut p = Presentation {
        finding: finding.id.clone(),
        depth,
        surface,
        text: depth::sentence(&finding.id, &prov).unwrap_or_else(|_| finding.claim.clone()),
        state: capsule.map(|c| c.state),
        glyphs: Vec::new(),
        confidence: None,
        spark,
        view: None,
        capsule_view: None,
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
    }
    match depth {
        Depth::Capsule if series_data => {
            p.capsule_view = depth::capsule_view(
                &finding.id,
                profile.max_points,
                &prov,
                &format!("capsule-{}", finding.id),
            )
            .ok()
            .map(Box::new);
        }
        Depth::Chart | Depth::Workspace => {
            p.view = capsule
                .and_then(|c| c.view.as_ref())
                .and_then(|v| doc.views.iter().find(|x| &x.id == v).cloned())
                .or_else(|| {
                    let id = format!("present-{}", finding.id);
                    depth::chart_view(&finding.id, &prov, &id).ok()
                });
            p.facts = finding
                .facts
                .iter()
                .filter_map(|f| doc.facts.iter().find(|x| &x.id == f).cloned())
                .collect();
        }
        Depth::Sentence | Depth::Capsule => {}
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
