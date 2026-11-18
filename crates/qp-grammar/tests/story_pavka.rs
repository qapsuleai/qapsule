//! R5/M18 story chart `pavka` (K50, K54, K53): every story of the visible range drawn,
//! the selected one emphasised, live ones on their own lanes, closed ones faint with their
//! outcome, clusters when the lanes run out, closed stories hideable — at 5, 50 and 200
//! stories; the scene's story elements and the point query name the same stories.

use std::collections::{BTreeMap, BTreeSet};

use qp_grammar::look::{Look, R4};
use qp_grammar::stories::{self, pavka, StoryInput, StoryTap};
use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
use qp_schema::grammar::ScaleKind;
use qp_schema::protocol::{HitItem, Language, StoryFace, StoryOutcome, StoryRow};
use qp_schema::scene::{DataSource, PixelRect, Prim, SceneDesc, ScenePanel, XMap, YMap};

const W: u32 = 390;
const H: u32 = 640;
const BARS: u64 = 1000;

/// The chart part as the compiler would leave it: one price panel over `[from, to]`.
fn chart(from: u64, to: u64) -> SceneDesc {
    let ph = (pavka::DESIGN.price_height)(H);
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

/// `n` stories over the last `span` bars: a mix of live, realized, invalidated, expired,
/// retracted, coin-flip and no-longer-valid ones (deterministic).
fn rows(n: usize, span: u64) -> Vec<StoryRow> {
    rows_life(n, span, 40)
}

/// [`rows`] with lives up to `max_life` bars.
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

fn fact_of(f: &str) -> Option<String> {
    Some(format!("fact-of-{f}"))
}

fn draw(
    look: &Look,
    theme: Option<&str>,
    r: &[StoryRow],
    focus: &str,
    hide: bool,
    range: (u64, u64),
) -> SceneDesc {
    let inp = StoryInput {
        rows: r,
        focus: Some(focus),
        hide_closed: hide,
        lang: Language::Tr,
        fact_of: &fact_of,
    };
    (pavka::DESIGN.extend)(look, theme, chart(range.0, range.1), H, &inp)
}

fn story_panel(s: &SceneDesc) -> &ScenePanel {
    s.panels
        .iter()
        .find(|p| p.name.as_deref() == Some(pavka::PANEL_NAME))
        .expect("story panel")
}

/// The point query's rule (`qp_session::hit_scene`): the panel under the point, then the
/// last item whose box holds it.
fn hit(s: &SceneDesc, x: f64, y: f64) -> Option<HitItem> {
    let inside = |r: &PixelRect| {
        x >= f64::from(r.left)
            && x < f64::from(r.right)
            && y >= f64::from(r.top)
            && y < f64::from(r.bottom)
    };
    let p = s.panels.iter().find(|p| inside(&p.rect))?;
    let mut out = None;
    for l in &p.layers {
        for it in &l.items {
            if inside(&it.hit) {
                out = Some(HitItem {
                    layer: l.id.clone(),
                    mark: l.mark,
                    data: l.data.clone(),
                    bar: it.bar,
                    t: it.t,
                    values: it.values.clone(),
                    constants: l.constants.clone(),
                });
            }
        }
    }
    out
}

fn visible(r: &[StoryRow], range: (u64, u64)) -> Vec<&StoryRow> {
    r.iter()
        .filter(|x| x.born_bar <= range.1 && x.closed_bar.unwrap_or(BARS - 1) >= range.0)
        .collect()
}

/// Stories drawn on a lane, by finding; stories inside clusters, by capsule id.
fn drawn(s: &SceneDesc) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut lanes = BTreeSet::new();
    let mut clustered = BTreeSet::new();
    for l in &story_panel(s).layers {
        if l.id.starts_with(pavka::STORY_LAYER) {
            if let DataSource::Finding { finding } = &l.data {
                assert!(lanes.insert(finding.clone()), "one lane layer per story");
            }
        } else if l.id.starts_with(pavka::CLUSTER_LAYER) {
            let n = l.items[0].values["stories"];
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = n as usize;
            assert_eq!(
                l.constants.len(),
                n,
                "{}: one origin per clustered story",
                l.id
            );
            for k in l.constants.keys() {
                clustered.insert(k.trim_start_matches("/stories/").to_owned());
            }
        }
    }
    (lanes, clustered)
}

fn check(n: usize, span: u64, range: (u64, u64)) -> SceneDesc {
    check_rows(&rows(n, span), range)
}

fn check_rows(r: &[StoryRow], range: (u64, u64)) -> SceneDesc {
    let n = r.len();
    let vis = visible(r, range);
    let focus = vis[vis.len() / 2].finding.clone();
    let s = draw(&R4, None, r, &focus, false, range);
    assert_eq!(s.height, H);
    let (lanes, clustered) = drawn(&s);
    // Every visible story is drawn: on a lane or in a cluster, never twice.
    assert_eq!(lanes.len() + clustered.len(), vis.len(), "n={n}");
    for x in &vis {
        let on_lane = lanes.contains(&x.finding);
        assert!(on_lane ^ clustered.contains(&x.id), "{} drawn once", x.id);
        // Live and selected stories always keep a lane.
        if x.closed_bar.is_none() || x.finding == focus {
            assert!(on_lane, "{} (live or selected) has a lane", x.id);
        }
    }
    // The selected story is drawn last (on top, wins the point query) in the focus colour.
    let sp = story_panel(&s);
    let last = sp.layers.last().unwrap();
    assert_eq!(
        last.data,
        DataSource::Finding {
            finding: focus.clone()
        }
    );
    assert!(last
        .prims
        .iter()
        .any(|p| matches!(p, Prim::Rect { color, .. } if *color == R4.dark.focus)));
    // `hit` names what the scene draws: the centre of every lane's box hits that story.
    for l in sp
        .layers
        .iter()
        .filter(|l| l.id.starts_with(pavka::STORY_LAYER) || l.id.starts_with(pavka::CLUSTER_LAYER))
    {
        let b = l.items[0].hit;
        let (cx, cy) = (
            f64::from(b.left + b.right) / 2.0,
            f64::from(b.top + b.bottom) / 2.0,
        );
        let h = hit(&s, cx, cy).expect("hit");
        // A lane's centre may sit under a later story's widened box only if both are the
        // same lane; the selected story wins its own row.
        if h.layer != l.id {
            let other = sp.layers.iter().find(|x| x.id == h.layer).unwrap();
            assert_eq!(
                other.items[0].values.get("lane"),
                l.items[0].values.get("lane"),
                "{} vs {}",
                l.id,
                h.layer
            );
            continue;
        }
        assert_eq!(h.data, l.data);
    }
    // Everything inside the frame.
    for p in s.prims() {
        if let Prim::Rect { x, y, w, h, .. } = p {
            assert!(*x >= 0 && *y >= 0, "{p:?}");
            assert!(
                x + w.cast_signed() <= W.cast_signed() && y + h.cast_signed() <= H.cast_signed(),
                "{p:?}"
            );
        }
    }
    // Deterministic.
    assert_eq!(s, draw(&R4, None, r, &focus, false, range));
    s
}

#[test]
fn five_stories() {
    let s = check(5, 200, (BARS - 200, BARS - 1));
    let (_, clustered) = drawn(&s);
    assert!(clustered.is_empty(), "5 stories: no cluster");
}

#[test]
fn fifty_stories() {
    check(50, 600, (BARS - 600, BARS - 1));
}

#[test]
fn two_hundred_stories_cluster_when_zoomed_out() {
    // Short lives pack: 200 stories over 950 bars mostly share lanes.
    let s = check(200, 950, (BARS - 950, BARS - 1));
    let short = drawn(&s).1.len();
    // Long lives (up to 300 bars) do not fit: lanes thin out, then cluster.
    let r = rows_life(200, 950, 300);
    let s = check_rows(&r, (BARS - 950, BARS - 1));
    let (lanes, clustered) = drawn(&s);
    assert!(
        !clustered.is_empty(),
        "long-lived: cluster ({} on lanes)",
        lanes.len()
    );
    assert!(
        clustered.len() > short,
        "long lives cluster more: {} > {short}",
        clustered.len()
    );
    // Zoomed in, the same list needs fewer clusters.
    let near = (BARS - 80, BARS - 1);
    let vis = visible(&r, near);
    let s2 = draw(&R4, None, &r, &vis[0].finding, false, near);
    let (_, c2) = drawn(&s2);
    assert!(
        c2.len() < clustered.len(),
        "zoomed in: {} < {}",
        c2.len(),
        clustered.len()
    );
}

#[test]
fn closed_stories_hide_and_the_selected_stays() {
    let r = rows(50, 600);
    let range = (BARS - 600, BARS - 1);
    let focus = r
        .iter()
        .find(|x| x.closed_bar.is_some())
        .unwrap()
        .finding
        .clone();
    let s = draw(&R4, None, &r, &focus, true, range);
    let (lanes, clustered) = drawn(&s);
    assert!(clustered.is_empty());
    for x in visible(&r, range) {
        let want = x.closed_bar.is_none() || x.finding == focus;
        assert_eq!(lanes.contains(&x.finding), want, "{}", x.id);
    }
    // The checkbox says so (its value) and its tap toggles.
    let t = story_panel(&s)
        .layers
        .iter()
        .find(|l| l.id == pavka::TOGGLE_LAYER)
        .unwrap();
    assert_eq!(t.items[0].values["hide_closed"], 1.0);
    let b = t.items[0].hit;
    let h = hit(&s, f64::from(b.left + 2), f64::from(b.top + 4));
    assert_eq!(
        (pavka::DESIGN.tap)(h.as_ref(), Some(&focus)),
        Some(StoryTap::ToggleClosed)
    );
}

#[test]
fn a_tap_on_another_story_moves_the_focus_and_a_cluster_zooms() {
    let r = rows_life(200, 950, 300);
    let range = (BARS - 950, BARS - 1);
    let vis = visible(&r, range);
    let focus = vis[3].finding.clone();
    let s = draw(&R4, None, &r, &focus, false, range);
    let sp = story_panel(&s);
    let mut moved = 0;
    for l in sp
        .layers
        .iter()
        .filter(|l| l.id.starts_with(pavka::STORY_LAYER))
    {
        let b = l.items[0].hit;
        let h = hit(
            &s,
            f64::from(b.left + b.right) / 2.0,
            f64::from(b.top + b.bottom) / 2.0,
        );
        let t = (pavka::DESIGN.tap)(h.as_ref(), Some(&focus));
        match &l.data {
            DataSource::Finding { finding } if *finding == focus => {
                assert_eq!(t, None, "own lane: evidence")
            }
            DataSource::Finding { finding } if h.as_ref().is_some_and(|h| h.layer == l.id) => {
                assert_eq!(t, Some(StoryTap::Focus(finding.clone())));
                moved += 1;
            }
            _ => {}
        }
    }
    assert!(moved > 10);
    let c = sp
        .layers
        .iter()
        .find(|l| l.id.starts_with(pavka::CLUSTER_LAYER))
        .expect("a cluster");
    let b = c.items[0].hit;
    let h = hit(
        &s,
        f64::from(b.left + b.right) / 2.0,
        f64::from(b.top + b.bottom) / 2.0,
    );
    match (pavka::DESIGN.tap)(h.as_ref(), Some(&focus)) {
        Some(StoryTap::Zoom(a, z)) => assert!(a <= z && a >= range.0 && z <= range.1),
        t => panic!("cluster tap: {t:?}"),
    }
}

#[test]
fn shapes_carry_the_outcome_and_closed_marks_keep_contrast() {
    // Closed marks ≥ 3:1 against the background, both schemes (A31).
    fn lum(c: [u8; 4]) -> f64 {
        let ch = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
    }
    for th in [R4.dark, R4.light] {
        let (a, b) = (lum(th.closed), lum(th.bg));
        let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
        assert!(ratio >= 3.0, "{}: closed {ratio:.2}:1", th.name);
    }
    // Each outcome has its own shape kind at the lane's end (not colour alone).
    let r = rows(9, 200);
    let range = (BARS - 200, BARS - 1);
    let s = draw(&R4, Some("light"), &r, &r[0].finding, false, range);
    let mut kinds: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    for l in story_panel(&s)
        .layers
        .iter()
        .filter(|l| l.id.starts_with(pavka::STORY_LAYER))
    {
        let ks = l
            .prims
            .iter()
            .map(|p| match p {
                Prim::Rect { .. } => "rect",
                Prim::Fill { .. } => "fill",
                Prim::Circle { .. } => "circle",
                Prim::Polyline { .. } => "line",
                Prim::Text { .. } => "text",
            })
            .collect();
        kinds.insert(l.id.clone(), ks);
    }
    let of = |id: &str| kinds[&format!("{}{id}", pavka::STORY_LAYER)].clone();
    assert!(of("cap.1").contains(&"circle"), "realized: a disc");
    assert!(of("cap.2").contains(&"line"), "invalidated: a cross");
    assert_eq!(
        of("cap.4").iter().filter(|k| **k == "circle").count(),
        2,
        "retracted: a ring"
    );
    // Coin flip (K53): a dashed lane (several rects).
    assert!(
        of("cap.3").iter().filter(|k| **k == "rect").count() > 2,
        "coin flip: dashed"
    );
}

#[test]
fn registry_and_price_height() {
    assert_eq!(stories::names(), vec!["r4", "pavka", "korda"]);
    assert!(stories::by_name(Some("r4")).is_none());
    assert!(stories::by_name(None).is_none());
    assert!(stories::is_story("pavka"));
    for h in [120, 240, 400, 640, 844, 2000] {
        let p = (pavka::DESIGN.price_height)(h);
        assert!(p >= qp_grammar::compile::MIN_HEIGHT && p <= h, "{h} -> {p}");
    }
}

#[test]
#[ignore = "timing helper"]
fn extend_timing() {
    let r = rows_life(200, 950, 300);
    let range = (BARS - 950, BARS - 1);
    let f = r[100].finding.clone();
    let t = std::time::Instant::now();
    for _ in 0..200 {
        let _ = draw(&R4, None, &r, &f, false, range);
    }
    eprintln!("extend 200 long: {:?}", t.elapsed() / 200);
    let r = rows(200, 950);
    let t = std::time::Instant::now();
    for _ in 0..200 {
        let _ = draw(&R4, None, &r, &f, false, range);
    }
    eprintln!("extend 200 short: {:?}", t.elapsed() / 200);
}
