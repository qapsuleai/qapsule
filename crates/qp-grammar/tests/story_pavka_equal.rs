//! (R5/m18-pavka-perf) The story chart `pavka`'s performance change does not change what
//! it draws: for 5, 50 and 200 stories in view (and a 533-story list zoomed from a few
//! bars to all of them), in every look and both named themes (and the look's default and
//! an unknown name), in both languages, with the closed stories shown and hidden, the
//! focus on a live, a closed, an out-of-range or no story, at three frame heights (so
//! with and without clusters, and without a story part), the current `pavka::extend`
//! gives a scene equal, and byte-equal as JSON, to the one the code before the change
//! gave (kept verbatim in `tests/pavka_ref/`).

mod pavka_ref;

use qp_grammar::look::{self, Look, R4};
use qp_grammar::stories::{pavka, StoryInput};
use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
use qp_schema::grammar::ScaleKind;
use qp_schema::protocol::{Language, StoryFace, StoryOutcome, StoryRow};
use qp_schema::scene::{PixelRect, SceneDesc, ScenePanel, XMap, YMap};

const W: u32 = 390;
const H: u32 = 640;
const BARS: u64 = 1000;

/// The chart part as the compiler would leave it: one price panel over `[from, to]`.
fn chart(from: u64, to: u64) -> SceneDesc {
    let ph = (pavka::DESIGN.price_height)(&R4, H);
    let [l, r, t, b] = R4.space.plot;
    #[allow(clippy::cast_precision_loss)]
    let slot = f64::from(W.cast_signed() - r - l) / (to - from + 1) as f64;
    SceneDesc {
        view: "present-x".to_owned(),
        width: W,
        height: ph,
        background: R4.dark.bg,
        x: XMap {
            from_bar: from,
            to_bar: to,
            left: f64::from(l),
            slot,
        },
        bars: BARS,
        lod: None,
        panels: vec![ScenePanel {
            id: 0,
            name: None,
            rect: PixelRect {
                left: l,
                top: t,
                right: W.cast_signed() - r,
                bottom: ph.cast_signed() - b,
            },
            y: YMap {
                kind: ScaleKind::Linear,
                min: 0.0,
                max: 1.0,
                top: f64::from(t),
                bottom: f64::from(ph.cast_signed() - b),
            },
            layers: Vec::new(),
        }],
        emphasis: Vec::new(),
        under: Vec::new(),
        over: Vec::new(),
        terms: Vec::new(),
    }
}

/// `n` stories over the last `span` bars, lives up to `max_life` bars: a mix of live,
/// realized, invalidated, expired, retracted, coin-flip and no-longer-valid ones
/// (deterministic; as `story_pavka.rs`).
fn rows_life(n: usize, span: u64, max_life: u64) -> Vec<StoryRow> {
    let mut v = Vec::new();
    let n64 = n as u64;
    for i in 0..n64 {
        let born = BARS - span + i * span / n64.max(1);
        let life = 5 + (i * 7) % max_life;
        let k = i % 9;
        let (closed, stage, outcome) = match k {
            0 | 5 if born + life >= BARS - 1 => (None, Stage::Born, None),
            1 | 6 => (
                Some(born + life),
                Stage::Realized,
                Some(StoryOutcome::Realized),
            ),
            2 | 7 => (
                Some(born + life),
                Stage::Invalidated,
                Some(StoryOutcome::Invalidated),
            ),
            3 => (
                Some(born + life),
                Stage::Expired,
                Some(StoryOutcome::Expired),
            ),
            4 => (
                Some(born + life),
                Stage::Retracted,
                Some(StoryOutcome::Retracted),
            ),
            _ => (
                Some(born + life),
                Stage::Realized,
                Some(StoryOutcome::Realized),
            ),
        };
        let closed = closed.map(|c: u64| c.min(BARS - 1));
        let live = closed.is_none();
        v.push(StoryRow {
            id: format!("cap.{i}"),
            finding: format!("fnd.{i}"),
            series: "px".to_owned(),
            kind: MeaningKind::Breakout,
            direction: match i % 3 {
                0 => Direction::Up,
                1 => Direction::Down,
                _ => Direction::Neutral,
            },
            tier: 1,
            born_bar: born,
            born_t: i64::try_from(born).unwrap() * 3_600_000,
            last_bar: closed.unwrap_or(born),
            last_t: 0,
            closed_bar: closed,
            closed_t: closed.map(|c| i64::try_from(c).unwrap() * 3_600_000),
            stage,
            outcome,
            validity: if live {
                if i % 2 == 0 {
                    Validity::Valid
                } else {
                    Validity::NoLonger
                }
            } else {
                Validity::Closed
            },
            in_feed: i % 4 != 3,
            hidden: None,
            coin_flip: i % 4 == 3,
            face: StoryFace {
                event: "yukarı kırılım".to_owned(),
                direction_word: "yukarı".to_owned(),
                when: "dün".to_owned(),
                moved: "dün".to_owned(),
                validity_phrase: if live { "geçerli" } else { "kapandı" }.to_owned(),
                ledger_short: "7'de 3 tuttu".to_owned(),
                few_cases: false,
                ledger_n: Some(7),
            },
            redacted: false,
        });
    }
    v
}

/// Every other finding has a first fact (the rest: unknown to the device yet).
fn fact_of(f: &str) -> Option<String> {
    f.len().is_multiple_of(2).then(|| format!("fact-of-{f}"))
}

/// A list of rows (named) and the visible ranges it is drawn over.
type Set = (&'static str, Vec<StoryRow>, Vec<(u64, u64)>);

struct Case<'a> {
    look: &'a Look,
    theme: Option<&'a str>,
    rows: &'a [StoryRow],
    focus: Option<&'a str>,
    hide: bool,
    lang: Language,
    range: (u64, u64),
    h: u32,
}

fn both(c: &Case<'_>) -> (SceneDesc, SceneDesc) {
    let inp = StoryInput {
        rows: c.rows,
        focus: c.focus,
        hide_closed: c.hide,
        lang: c.lang,
        fact_of: &fact_of,
        stamp_of: None,
    };
    let old = pavka_ref::extend(c.look, c.theme, chart(c.range.0, c.range.1), c.h, &inp);
    let new = pavka::extend(c.look, c.theme, chart(c.range.0, c.range.1), c.h, &inp);
    (old, new)
}

#[test]
fn extend_draws_byte_for_byte_what_it_drew_before() {
    let sets: Vec<Set> = vec![
        (
            "5",
            rows_life(5, 200, 40),
            vec![(BARS - 200, BARS - 1), (BARS - 120, BARS - 60)],
        ),
        (
            "50",
            rows_life(50, 600, 40),
            vec![(BARS - 600, BARS - 1), (BARS - 300, BARS - 100)],
        ),
        (
            "200",
            rows_life(200, 950, 40),
            vec![(BARS - 950, BARS - 1), (BARS - 500, BARS - 200)],
        ),
        (
            "200 long",
            rows_life(200, 950, 300),
            vec![(BARS - 950, BARS - 1), (BARS - 80, BARS - 1)],
        ),
        (
            "533",
            rows_life(533, 990, 30),
            vec![
                (BARS - 12, BARS - 1),
                (BARS - 95, BARS - 1),
                (BARS - 380, BARS - 1),
                (BARS - 990, BARS - 1),
                (BARS - 400, BARS - 300),
            ],
        ),
    ];
    // (R5/S) `r4` only: the design now measures its words with the compiler's measure
    // (`FontAsset::width`) and keeps the "closed" checkbox's word whole by moving the box
    // left when the margin is too narrow for it (it was cut at the right edge under
    // `oltan`, and ended in "…" under `ornek`) — a deliberate change for the other looks.
    let looks: Vec<&Look> = look::names()
        .into_iter()
        .map(|n| look::by_name(Some(n)).unwrap())
        .filter(|l| l.name == "r4")
        .collect();
    assert!(looks.iter().any(|l| l.name == "r4"));
    let mut n = 0;
    let (mut clustered, mut hidden, mut early) = (0, 0, 0);
    for (name, rows, ranges) in &sets {
        for &range in ranges {
            let vis: Vec<&StoryRow> = rows
                .iter()
                .filter(|r| r.born_bar <= range.1 && r.closed_bar.unwrap_or(BARS - 1) >= range.0)
                .collect();
            let live = vis
                .iter()
                .find(|r| r.closed_bar.is_none())
                .map(|r| r.finding.as_str());
            let closed = vis
                .iter()
                .rev()
                .find(|r| r.closed_bar.is_some())
                .map(|r| r.finding.as_str());
            let mid = vis.get(vis.len() / 2).map(|r| r.finding.as_str());
            let out = rows
                .iter()
                .find(|r| r.closed_bar.is_some_and(|c| c < range.0))
                .map(|r| r.finding.as_str());
            let focuses = [mid, live, closed, out, None, Some("fnd.nope")];
            for lk in &looks {
                for theme in [None, Some("dark"), Some("light"), Some("nope")] {
                    for focus in focuses {
                        for hide in [false, true] {
                            for lang in [Language::Tr, Language::En] {
                                for h in [H, 400, 200] {
                                    let c = Case {
                                        look: lk,
                                        theme,
                                        rows,
                                        focus,
                                        hide,
                                        lang,
                                        range,
                                        h,
                                    };
                                    let (old, new) = both(&c);
                                    let (a, b) = (
                                        serde_json::to_vec(&old).unwrap(),
                                        serde_json::to_vec(&new).unwrap(),
                                    );
                                    assert!(
                                        a == b,
                                        "{name} {range:?} look {} theme {theme:?} focus {focus:?} hide {hide} {lang:?} h {h}",
                                        lk.name
                                    );
                                    assert_eq!(old, new);
                                    n += 1;
                                    if new.panels.len() == 1 {
                                        early += 1;
                                    }
                                    if new
                                        .panels
                                        .iter()
                                        .flat_map(|p| &p.layers)
                                        .any(|l| l.id.starts_with(pavka::CLUSTER_LAYER))
                                    {
                                        clustered += 1;
                                    }
                                    if hide {
                                        hidden += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("{n} scenes equal ({clustered} with clusters, {hidden} hiding closed, {early} without a story part)");
    assert!(
        clustered > 100 && early > 0,
        "the cases reach clusters and the early return"
    );
}
