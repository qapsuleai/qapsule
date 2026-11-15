//! Generator of the R3 capsule view fixtures (`docs/GRAMMAR.md` §18, R3/T7 criteria 7–8).
//!
//! Shared by `examples/make_capsule_views.rs` (writes the files) and
//! `tests/capsule_views.rs` (checks that the committed files are exactly what this
//! generator produces, then renders them). Nothing is written by hand:
//!
//! - **Bars:** `qp_source::synthetic_market(4, 300, seed)` (regime market, XIST calendar),
//!   one instrument at a time.
//! - **Facts and stories:** the real detectors (`qp_facts::default_detectors`) and the real
//!   capsule engine (`qp_capsule::simulate_engine`) with fixture-local rules (below; the
//!   chosen M3–M5 rules are not in the product crates yet at R3/T7): theme = kind +
//!   direction, a directional flow fact on the same or next bar confirming an open
//!   breakout/reversal/continuation of the same direction (the M4 `kinddir` exception);
//!   outcome = a 4 % close move from the birth close within 20 bars (realized in the
//!   story's direction, invalidated against it; either way for a neutral story); constant
//!   rank.
//! - **Selection:** for each sample the first (seed, instrument, story) in a fixed search
//!   order whose story is born at bar ≥ 80 and is still open 6 bars later; the document
//!   holds the bars up to that bar (`cut = born + 6`), re-simulated on exactly those bars,
//!   and only the selected story's facts, finding and capsule. Breakout: kind `breakout`,
//!   up, confirmed by the volume family. Squeeze: kind `squeeze`. Divergence:
//!   `reversal.rsi_divergence`.
//! - **Sentence:** the product sentence: `qp_capsule::sentence::sentences_at` with
//!   `qp_capsule::realize::Template`, Turkish, an entitled reader, as of the cut bar.
//!
//! Transcripts: `c1-sentence` (watch: the sentence only, checked by `qp conform`),
//! `c2-capsule` (phone, 390x300: the capsule card), `c3-chart` (desktop: `show_finding`
//! then `present`, the chart with the breakout level), `c4-band` (squeeze band),
//! `c5-points` (divergence swing points).

#![allow(dead_code)]

use qp_capsule::{
    pipeline::simulate_engine, EngineConfig, InstrumentInput, OpenStory, OutcomeRule, RankCtx,
    Ranker, ThemeDecision, ThemeRule, Verdict,
};
use qp_schema::capsule::{Direction, Edge, EvidenceFamily, MeaningKind, TimeKind};
use qp_schema::{Bar, Fact};
use serde_json::{json, Value};

/// Depth views: (transcript stem, finding sample, surface, width, height).
pub const DEPTHS: &[(&str, Sample, &str, u32, u32)] = &[
    ("c2-capsule", Sample::Breakout, "phone", 390, 300),
    ("c3-chart", Sample::Breakout, "desktop", 1200, 600),
    ("c4-band", Sample::Squeeze, "desktop", 1200, 600),
    ("c5-points", Sample::Divergence, "desktop", 1200, 600),
];

/// The three sample stories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sample {
    /// Range breakout up, confirmed by a volume surge (Level geometry).
    Breakout,
    /// Bollinger squeeze (Band geometry).
    Squeeze,
    /// RSI divergence (Points geometry).
    Divergence,
}

impl Sample {
    /// Document path (relative to the repository root).
    #[must_use]
    pub fn doc(self) -> &'static str {
        match self {
            Sample::Breakout => "fixtures/views/docs/r3-breakout.json",
            Sample::Squeeze => "fixtures/views/docs/r3-squeeze.json",
            Sample::Divergence => "fixtures/views/docs/r3-divergence.json",
        }
    }

    fn matches(self, s: &OpenStory) -> bool {
        let lead = &s.facts[0];
        let det = lead.meaning.as_ref().map(|m| m.detector.as_str());
        match self {
            Sample::Breakout => {
                s.story.kind == MeaningKind::Breakout
                    && s.story.direction == Direction::Up
                    && s.story.families.contains(&EvidenceFamily::Volume)
            }
            Sample::Squeeze => s.story.kind == MeaningKind::Squeeze,
            Sample::Divergence => det == Some("reversal.rsi_divergence"),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture-local engine rules
// ---------------------------------------------------------------------------

/// Theme = kind + direction; a directional flow fact on the same or the next bar joins an
/// open breakout/reversal/continuation story of its direction (confirmation); an exit
/// joins the story holding the enter of its detector.
struct KindDir;

impl ThemeRule for KindDir {
    fn name(&self) -> &'static str {
        "fixture.kind_dir"
    }

    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };
        if m.edge == Edge::Exit {
            let own = open.iter().find(|s| {
                s.facts.iter().any(|g| {
                    g.meaning
                        .as_ref()
                        .is_some_and(|gm| gm.edge == Edge::Enter && gm.detector == m.detector)
                })
            });
            return own.map_or(ThemeDecision::Ignore, |s| ThemeDecision::Join(s.id.clone()));
        }
        if m.kind == MeaningKind::Flow && m.direction != Direction::Neutral {
            let confirms = open.iter().find(|s| {
                matches!(
                    s.story.kind,
                    MeaningKind::Breakout | MeaningKind::Reversal | MeaningKind::Continuation
                ) && s.story.direction == m.direction
                    && fact.bar <= s.story.last_bar + 1
            });
            if let Some(s) = confirms {
                return ThemeDecision::Join(s.id.clone());
            }
        }
        match open
            .iter()
            .find(|s| s.story.kind == m.kind && s.story.direction == m.direction)
        {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None => ThemeDecision::New {
                theme: format!("{}-{}", m.kind.name(), dir_name(m.direction)),
            },
        }
    }
}

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// A 4 % close move from the birth close within 20 bars.
struct Move;

impl OutcomeRule for Move {
    fn name(&self) -> &'static str {
        "fixture.move4"
    }

    fn horizon(&self, _kind: MeaningKind, _time: TimeKind) -> u32 {
        20
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now = usize::try_from(now).ok()?;
        if now <= born {
            return None;
        }
        let r = bars.get(now)?.c / bars.get(born)?.c - 1.0;
        let (hit, miss) = match s.story.direction {
            Direction::Up => (r >= 0.04, r <= -0.04),
            Direction::Down => (r <= -0.04, r >= 0.04),
            Direction::Neutral => (r.abs() >= 0.04, false),
        };
        if hit {
            Some((Verdict::Realized, "move_hit"))
        } else if miss {
            Some((Verdict::Invalidated, "move_miss"))
        } else {
            None
        }
    }
}

struct Flat;

impl Ranker for Flat {
    fn name(&self) -> &'static str {
        "fixture.flat"
    }

    fn rank(&self, _s: &OpenStory, _ctx: &RankCtx) -> (f64, u8) {
        (1.0, 1)
    }
}

fn config() -> EngineConfig {
    EngineConfig {
        theme: Box::new(KindDir),
        outcome: Box::new(Move),
        ranker: Box::new(Flat),
    }
}

fn run(symbol: &str, bars: &[Bar]) -> qp_capsule::Engine {
    let input = InstrumentInput {
        symbol: symbol.to_owned(),
        series: "px".to_owned(),
        bars: bars.to_vec(),
        flow: None,
    };
    simulate_engine(&[input], &qp_facts::default_detectors, config()).0
}

/// Bars kept after the story's birth.
pub const AFTER_BIRTH: u64 = 6;

/// The selected (symbol, bars up to the cut, open story) for a sample.
///
/// # Panics
/// No sample found in the search range (the generator is broken).
#[must_use]
pub fn select(sample: Sample) -> (String, Vec<Bar>, OpenStory) {
    for seed in 1..=64_u64 {
        for inst in qp_source::synthetic_market(4, 300, seed) {
            let full = run(&inst.symbol, &inst.bars);
            let mut births: Vec<u64> = full
                .open_stories()
                .iter()
                .chain(full.closed_stories())
                .map(|s| s.story.born_bar)
                .filter(|b| *b >= 80 && b + AFTER_BIRTH < inst.bars.len() as u64)
                .collect();
            births.sort_unstable();
            births.dedup();
            for b in births {
                let cut = usize::try_from(b + AFTER_BIRTH + 1).unwrap_or(0);
                let bars = &inst.bars[..cut];
                let eng = run(&inst.symbol, bars);
                if let Some(s) = eng
                    .open_stories()
                    .iter()
                    .find(|s| s.story.born_bar == b && sample.matches(s))
                {
                    return (inst.symbol.clone(), bars.to_vec(), s.clone());
                }
            }
        }
    }
    panic!("no {sample:?} sample in the search range");
}

/// The product sentence (`qp_capsule::sentence::sentences_at` with the template realizer,
/// Turkish, an entitled reader) of the story as of `now`, from the engine re-run on
/// exactly `bars`.
fn headline(symbol: &str, bars: &[Bar], s: &OpenStory, now: u64) -> String {
    let eng = run(symbol, bars);
    qp_capsule::sentence::sentences_at(
        &eng,
        &qp_capsule::realize::Template,
        &s.id,
        bars,
        now,
        &qp_capsule::sentence::Reader::entitled(),
    )
    .expect("sentence of the selected story")
    .tr
}

fn bar_json(b: &Bar) -> Value {
    json!({"t": b.t, "o": b.o, "h": b.h, "l": b.l, "c": b.c, "v": b.v})
}

/// The sample's document (`schema_version` 0, one inline bar series `px`).
#[must_use]
pub fn document(sample: Sample) -> Value {
    let (symbol, bars, s) = select(sample);
    let now = bars.len() as u64 - 1;
    let finding_id = qp_capsule::engine::finding_id(&s.id);
    let text = headline(&symbol, &bars, &s, now);
    let facts: Vec<Value> = s
        .facts
        .iter()
        .map(|f| serde_json::to_value(f).expect("fact JSON"))
        .collect();
    let fact_ids: Vec<&str> = s.facts.iter().map(|f| f.id.as_str()).collect();
    let lab = s.story.ledger_at_birth;
    let confidence = if lab.n > 0 {
        f64::from(lab.realized) / f64::from(lab.n)
    } else {
        0.5
    };
    let prov = json!({"author": {"kind": "engine", "name": "qp-capsule", "version": qp_capsule::pipeline::ENGINE_VERSION},
                      "at": bars[usize::try_from(now).unwrap_or(0)].t, "based_on": fact_ids});
    json!({
        "schema_version": "0",
        "id": format!("doc-{}", sample.doc().trim_start_matches("fixtures/views/docs/").trim_end_matches(".json")),
        "title": format!("R3 sample: {symbol} ({})", s.story.ledger_key),
        "series": [{
            "id": "px",
            "instrument": {"symbol": symbol, "exchange": "BIST", "calendar": "XIST"},
            "interval": "1d",
            "source": {"kind": "inline"},
            "data": {"kind": "bars", "bars": bars.iter().map(bar_json).collect::<Vec<_>>()}
        }],
        "facts": facts,
        "findings": [{
            "id": finding_id,
            "claim": text,
            "facts": fact_ids,
            "confidence": confidence,
            "invalidation": {"kind": "text", "text": "the outcome rule closes the story"},
            "provenance": prov
        }],
        "capsules": [{
            "id": s.id,
            "finding": finding_id,
            "state": "live",
            "revision": 0,
            "headline": text,
            "glyphs": [],
            "depth": "capsule",
            "story": serde_json::to_value(&s.story).expect("story JSON"),
            "provenance": prov
        }]
    })
}

/// The finding id of a sample (from its document).
#[must_use]
pub fn finding_of(doc: &Value) -> String {
    doc["findings"][0]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn owner_init(doc: &str) -> Value {
    json!({
        "document": doc,
        "principals": [{"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"}],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0
    })
}

fn hello(surface: &str) -> Value {
    json!({
        "send": {"conn": "a", "msg": {"type": "hello", "protocol": "qp/0", "principal": "arkin",
                 "surface": {"kind": surface}}},
        "expect": [{"conn": "a", "msg": {"type": "welcome", "principal": "arkin", "role": "owner"}}]
    })
}

fn present(finding: &str, expect: &Value) -> Value {
    json!({
        "send": {"conn": "a", "msg": {"type": "query", "id": "q1",
                 "query": {"kind": "present", "finding": finding}}},
        "expect": [{"conn": "a", "msg": {"type": "result", "id": "q1",
                    "data": {"kind": "presentation", "presentation": expect}}}]
    })
}

fn transcript(
    name: &str,
    description: &str,
    doc: &str,
    steps: Vec<Value>,
    views: &[&str],
) -> Value {
    let views: Vec<Value> = views.iter().map(|v| json!({"id": v})).collect();
    let mut final_doc = json!({"views": views});
    if views.is_empty() {
        final_doc = json!({});
    }
    json!({
        "name": name,
        "description": description,
        "covers": ["presenter", "present", "depth", "capsule", "glyph"],
        "init": owner_init(doc),
        "steps": steps,
        "final": {"document": final_doc}
    })
}

fn chart_steps(finding: &str, view: &str) -> Vec<Value> {
    vec![
        hello("desktop"),
        json!({
            "send": {"conn": "a", "msg": {"type": "command", "id": "c1",
                     "ops": [{"op": "show_finding", "finding": finding, "view": view}]}},
            "expect": [{"conn": "a", "msg": {"type": "ack", "id": "c1", "status": "applied"}}]
        }),
        present(
            finding,
            &json!({"depth": "chart", "finding": finding, "surface": "desktop", "view": {"id": view}}),
        ),
    ]
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("JSON");
    s.push('\n');
    s
}

/// Every generated file: (path relative to the repository root, content).
#[must_use]
pub fn files() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut findings = Vec::new();
    for sample in [Sample::Breakout, Sample::Squeeze, Sample::Divergence] {
        let doc = document(sample);
        findings.push((
            sample,
            finding_of(&doc),
            doc["capsules"][0]["headline"].clone(),
        ));
        out.push((sample.doc().to_owned(), pretty(&doc)));
    }
    let f = |s: Sample| {
        findings
            .iter()
            .find(|x| x.0 == s)
            .map(|x| x.1.clone())
            .unwrap_or_default()
    };
    let breakout = f(Sample::Breakout);
    let headline = findings[0].2.clone();
    out.push((
        "fixtures/views/c1-sentence.json".to_owned(),
        pretty(&transcript(
            "c1-sentence",
            "C1: an R3 capsule (range breakout up, confirmed by volume) at sentence depth (watch): the presenter returns only the capsule's sentence. Same capsule as c2-capsule and c3-chart.",
            Sample::Breakout.doc(),
            vec![
                hello("watch"),
                present(&breakout, &json!({"depth": "sentence", "finding": breakout, "surface": "watch", "text": headline})),
            ],
            &[],
        )),
    ));
    out.push((
        "fixtures/views/c2-capsule.json".to_owned(),
        pretty(&transcript(
            "c2-capsule",
            "C2: the same R3 capsule at capsule depth (phone): the capsule card (meaning glyph, sentence, ledger line) over a mini price line with the breakout level and the fact point. Rendered with `qp render --present <finding> --surface phone --width 390 --height 300`.",
            Sample::Breakout.doc(),
            vec![
                hello("phone"),
                present(&breakout, &json!({"capsule_view": "$any", "depth": "capsule", "finding": breakout, "surface": "phone"})),
            ],
            &[],
        )),
    ));
    for (stem, sample, what) in [
        ("c3-chart", Sample::Breakout, "C3: the same R3 capsule at chart depth (desktop): show_finding builds the presenter's view — candles, the broken level drawn from the fact's evidence (a rule segment over the detector's 20-bar window, origin: the fact), the meaning glyph over the fact bar, highlight on the fact."),
        ("c4-band", Sample::Squeeze, "C4: an R3 squeeze capsule at chart depth (desktop): the Bollinger band of the squeeze drawn from the fact's evidence (upper/lower over the geometry's bars, origin: the fact) and the meaning glyph."),
        ("c5-points", Sample::Divergence, "C5: an R3 RSI-divergence capsule at chart depth (desktop): the two swing points from the fact's evidence and the segment joining them (origin: the fact), and the meaning glyph."),
    ] {
        let fid = f(sample);
        let view = format!("v-{stem}");
        out.push((
            format!("fixtures/views/{stem}.json"),
            pretty(&transcript(stem, &format!("{what} Rendered with `qp render --present <finding> --surface desktop`."), sample.doc(), chart_steps(&fid, &view), &[&view])),
        ));
    }
    out
}
