//! R5/M18 `korda` (lifelines, `qp_grammar::stories::korda`) on a synthetic instrument with
//! 5 / 50 / 200 stories in view: every story accounted for (a lane or a cluster count),
//! lanes that never overlap, the selected story first, closed stories hidden on request,
//! the point query naming every element it is drawn for, determinism and contrast.

mod support;

use qp_grammar::stories::korda::{self, StoryHit, CLUSTER_LAYER, STORY_LAYER, TOGGLE_LAYER};
use qp_grammar::stories::StoryInput;
use qp_grammar::{CompileOptions, MemProvider};
use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
use qp_schema::grammar::{Layer, Mark, Panel, Scale, ScaleKind, View};
use qp_schema::protocol::{FeedHidden, HitItem, Language, StoryFace, StoryOutcome, StoryRow};
use qp_schema::scene::{PixelRect, Prim, SceneDesc, ScenePanel};
use qp_schema::Bar;

const BARS: u64 = 300;

fn bars() -> Vec<Bar> {
    let t0 = 1_735_776_000_000_i64;
    (0..BARS)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let x = i as f64;
            let c = 100.0 + 10.0 * ((x / 17.0).sin()) + x / 10.0;
            Bar {
                t: t0 + i64::try_from(i).unwrap() * 86_400_000,
                o: c - 0.5,
                h: c + 1.0,
                l: c - 1.0,
                c,
                v: 1000.0,
            }
        })
        .collect()
}

fn provider() -> MemProvider {
    MemProvider::new().with_bars("px", bars(), Some("SYN"), Some("1d"))
}

fn view() -> View {
    let mut candles = Layer::new(Mark::Candle, 0);
    candles.id = Some("candles".to_owned());
    candles.series = Some("px".to_owned());
    View {
        id: "present-f0".to_owned(),
        title: None,
        display_tz: None,
        theme: None,
        x: Scale::new(ScaleKind::Time),
        y: Vec::new(),
        panels: vec![Panel {
            id: Some(0),
            name: None,
            weight: None,
            y: Scale::new(ScaleKind::Linear),
        }],
        layers: vec![candles],
        emphasis: Vec::new(),
    }
}

/// `n` stories spread over the bars, deterministic: lives of 3–27 bars, outcomes in
/// turn, the last ones open, every 7th a coin flip, every 3rd open one "no longer".
fn rows(n: u64) -> Vec<StoryRow> {
    let kinds = [
        MeaningKind::Breakout,
        MeaningKind::Reversal,
        MeaningKind::Continuation,
        MeaningKind::Extreme,
        MeaningKind::Squeeze,
    ];
    let outcomes = [
        Stage::Realized,
        Stage::Invalidated,
        Stage::Expired,
        Stage::Realized,
        Stage::Retracted,
    ];
    (0..n)
        .map(|i| {
            let born = (i * 283 / n.max(1)) % (BARS - 5);
            let len = 3 + (i * 7) % 25;
            let open = born + len >= BARS - 1 || i + 3 >= n;
            let stage = if open {
                Stage::Born
            } else {
                outcomes[usize::try_from(i % 5).unwrap()]
            };
            let dir = match i % 3 {
                0 => Direction::Up,
                1 => Direction::Down,
                _ => Direction::Neutral,
            };
            let last = if open { born + len / 2 } else { born + len };
            let t = |b: u64| 1_735_776_000_000_i64 + i64::try_from(b).unwrap() * 86_400_000;
            StoryRow {
                id: format!("cap.SYN.{i}"),
                finding: format!("f{i}"),
                series: "px".to_owned(),
                kind: kinds[usize::try_from(i % 5).unwrap()],
                direction: dir,
                tier: u8::try_from(1 + i % 3).unwrap(),
                born_bar: born,
                born_t: t(born),
                last_bar: last,
                last_t: t(last),
                closed_bar: (!open).then_some(last),
                closed_t: (!open).then(|| t(last)),
                stage,
                outcome: StoryOutcome::of(stage),
                validity: if !open {
                    Validity::Closed
                } else if i % 3 == 0 {
                    Validity::NoLonger
                } else {
                    Validity::Valid
                },
                in_feed: i % 7 != 0,
                hidden: (i % 7 == 0).then_some(FeedHidden::CoinFlip),
                coin_flip: i % 7 == 0,
                face: StoryFace {
                    event: "yukarı kırılım".to_owned(),
                    direction_word: "yukarı".to_owned(),
                    when: "dün".to_owned(),
                    moved: "dün".to_owned(),
                    validity_phrase: if open {
                        "hâlâ geçerli"
                    } else {
                        "gerçekleşti"
                    }
                    .to_owned(),
                    ledger_short: "7'de 3 tuttu".to_owned(),
                    few_cases: false,
                    ledger_n: Some(7),
                },
                redacted: false,
            }
        })
        .collect()
}

fn draw(rows: &[StoryRow], selected: &str, show_closed: bool, lang: Language) -> SceneDesc {
    let opts = CompileOptions::new(390, 300);
    korda::scene(
        &view(),
        &provider(),
        &opts,
        &StoryInput {
            rows,
            focus: Some(selected),
            hide_closed: !show_closed,
            lang,
            fact_of: &|_| None,
            stamp_of: None,
        },
    )
    .expect("scene")
}

fn panel(s: &SceneDesc) -> &ScenePanel {
    s.panels
        .iter()
        .find(|p| p.name.as_deref() == Some("stories"))
        .expect("stories panel")
}

/// The point query's rule (`qp_session::hit_scene`): the panel under the pixel, then the
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

fn visible(rows: &[StoryRow], s: &SceneDesc) -> Vec<String> {
    rows.iter()
        .filter(|r| {
            let end = r.closed_bar.unwrap_or_else(|| r.last_bar.max(BARS - 1));
            r.born_bar <= s.x.to_bar && end >= s.x.from_bar
        })
        .map(|r| r.id.clone())
        .collect()
}

fn lanes(s: &SceneDesc) -> Vec<&qp_schema::scene::SceneLayer> {
    panel(s)
        .layers
        .iter()
        .filter(|l| l.id.starts_with(STORY_LAYER))
        .collect()
}

fn clusters(s: &SceneDesc) -> Vec<&qp_schema::scene::SceneLayer> {
    panel(s)
        .layers
        .iter()
        .filter(|l| l.id.starts_with(CLUSTER_LAYER))
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn accounted(s: &SceneDesc) -> f64 {
    lanes(s).len() as f64
        + clusters(s)
            .iter()
            .map(|l| l.items[0].values["count"])
            .sum::<f64>()
}

fn center(r: &PixelRect) -> (f64, f64) {
    (
        f64::from(r.left + r.right - 1) / 2.0,
        f64::from(r.top + r.bottom - 1) / 2.0,
    )
}

#[test]
fn five_fifty_two_hundred_every_story_accounted_for() {
    // 5: lanes only; 50: lanes or a summary (the phone frame's room decides); 200: a
    // summary.
    for (n, want_clusters) in [(5u64, Some(false)), (50, None), (200, Some(true))] {
        let rs = rows(n);
        let s = draw(&rs, "f1", true, Language::Tr);
        let vis = visible(&rs, &s);
        #[allow(clippy::cast_precision_loss)]
        let want = vis.len() as f64;
        assert!(
            (accounted(&s) - want).abs() < 0.5,
            "n={n}: {} of {want}",
            accounted(&s)
        );
        if let Some(w) = want_clusters {
            assert_eq!(!clusters(&s).is_empty(), w, "n={n}");
        }
        // Lanes on the same row never overlap.
        let ls = lanes(&s);
        for a in &ls {
            for b in &ls {
                let (ha, hb) = (a.items[0].hit, b.items[0].hit);
                if a.id != b.id && ha.top == hb.top {
                    assert!(
                        ha.right <= hb.left || hb.right <= ha.left,
                        "n={n}: {} and {} overlap",
                        a.id,
                        b.id
                    );
                }
            }
        }
        // The selected story's lane is the first under the header.
        let sel = ls
            .iter()
            .find(|l| l.id == "story:cap.SYN.1")
            .expect("selected lane");
        assert_eq!(sel.items[0].hit.top, panel(&s).rect.top + korda::HEADER_PX);
        assert!(ls
            .iter()
            .all(|l| l.items[0].hit.top >= sel.items[0].hit.top));
    }
}

#[test]
fn five_stories_have_labels_and_marks_by_shape() {
    let rs = rows(5);
    let s = draw(&rs, "f1", true, Language::Tr);
    let texts: Vec<&str> = panel(&s)
        .layers
        .iter()
        .flat_map(|l| &l.prims)
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("canlı ")), "{texts:?}");
    assert!(texts.contains(&"gizle"), "{texts:?}");
    assert!(
        texts
            .iter()
            .filter(|t| t.contains("yukarı kırılım"))
            .count()
            >= 3
    );
    // A coin-flip story (K53) is drawn and says so.
    let coin = lanes(&s)
        .into_iter()
        .find(|l| l.id == "story:cap.SYN.0")
        .expect("coin flip lane");
    assert!(coin.label.as_deref().unwrap().contains("yazı tura"));
}

#[test]
fn every_element_is_named_by_the_point_query() {
    for n in [5u64, 50, 200] {
        let rs = rows(n);
        let s = draw(&rs, "f1", true, Language::Tr);
        let mut seen = 0;
        for l in &panel(&s).layers {
            for it in &l.items {
                let (x, y) = center(&it.hit);
                let h = hit(&s, x, y).unwrap_or_else(|| panic!("n={n}: nothing at {}", l.id));
                assert_eq!(h.layer, l.id, "n={n}");
                let meant = korda::story_hit(&h).expect("a story chart element");
                match meant {
                    StoryHit::Focus { capsule, finding } => {
                        let r = rs.iter().find(|r| r.id == capsule).expect("row");
                        assert_eq!(r.finding, finding);
                    }
                    StoryHit::Zoom { from, to } => assert!(from <= to),
                    StoryHit::ToggleClosed => assert_eq!(l.id, TOGGLE_LAYER),
                }
                seen += 1;
            }
        }
        assert!(seen > 1, "n={n}");
    }
}

#[test]
fn closed_stories_can_be_hidden() {
    let rs = rows(50);
    let shown = draw(&rs, "f1", true, Language::Tr);
    let hidden = draw(&rs, "f1", false, Language::Tr);
    let closed_ids: Vec<&str> = rs
        .iter()
        .filter(|r| r.closed_bar.is_some() && r.finding != "f1")
        .map(|r| r.id.as_str())
        .collect();
    for l in lanes(&hidden) {
        let cap = l.id.trim_start_matches(STORY_LAYER);
        assert!(!closed_ids.contains(&cap), "{cap} is closed");
    }
    assert!(clusters(&hidden).is_empty());
    assert!(accounted(&hidden) < accounted(&shown));
    let hdr = |s: &SceneDesc| {
        panel(s)
            .layers
            .iter()
            .find(|l| l.id == TOGGLE_LAYER)
            .unwrap()
            .items[0]
            .values
            .clone()
    };
    assert_eq!(hdr(&shown)["shown"], 1.0);
    assert_eq!(hdr(&hidden)["shown"], 0.0);
    assert_eq!(hdr(&hidden)["closed"], hdr(&shown)["closed"]);
}

#[test]
fn range_limits_the_stories_and_english_words() {
    let rs = rows(200);
    let mut opts = CompileOptions::new(390, 300);
    opts.range = Some((200, 239));
    let s = korda::scene(
        &view(),
        &provider(),
        &opts,
        &StoryInput {
            rows: &rs,
            focus: Some("f150"),
            hide_closed: false,
            lang: Language::En,
            fact_of: &|_| None,
            stamp_of: None,
        },
    )
    .expect("scene");
    assert_eq!((s.x.from_bar, s.x.to_bar), (200, 239));
    let vis = visible(&rs, &s);
    assert!(vis.len() < 60);
    #[allow(clippy::cast_precision_loss)]
    let want = vis.len() as f64;
    assert!((accounted(&s) - want).abs() < 0.5);
    let texts: Vec<&str> = panel(&s)
        .layers
        .iter()
        .flat_map(|l| &l.prims)
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("live ")), "{texts:?}");
}

#[test]
fn no_rows_is_the_plain_chart_and_scenes_are_deterministic() {
    let plain = draw(&[], "f1", true, Language::Tr);
    let base = qp_grammar::compile(&view(), &provider(), &CompileOptions::new(390, 300)).unwrap();
    assert_eq!(plain, base);
    let rs = rows(200);
    let a = serde_json::to_string(&draw(&rs, "f3", true, Language::Tr)).unwrap();
    let b = serde_json::to_string(&draw(&rs, "f3", true, Language::Tr)).unwrap();
    assert_eq!(a, b);
    // Row order does not matter.
    let mut rev = rs.clone();
    rev.reverse();
    let c = serde_json::to_string(&draw(&rev, "f3", true, Language::Tr)).unwrap();
    assert_eq!(a, c);
}

/// WCAG relative luminance and contrast.
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

fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn closed_marks_and_lane_text_keep_their_contrast() {
    for look in qp_grammar::look::LOOKS {
        for t in [look.dark, look.light] {
            // Marks and lines (WCAG 1.4.11): ≥ 3:1. The closed colour is drawn at full
            // opacity (faded by weight, not by alpha). Every look's closed colour; the
            // other roles in the reference look (a trial look's own roles are its own).
            let k = contrast(t.closed, t.bg);
            assert!(k >= 3.0, "{} {}: closed {k:.2}", look.name, t.name);
            if look.name != "r4" {
                continue;
            }
            for (what, c) in [
                ("closed", t.closed),
                ("up", t.up),
                ("down", t.down),
                ("neutral", t.neutral),
                ("focus", t.focus),
                ("valid", t.valid),
                ("invalid", t.invalid),
                ("muted (control)", t.muted),
            ] {
                let k = contrast(c, t.bg);
                assert!(k >= 3.0, "{} {}: {what} {k:.2}", look.name, t.name);
            }
            // Lane labels and the header (WCAG 1.4.3, small text): ≥ 4.5:1.
            for (what, c) in [
                ("closed text", t.closed),
                ("text", t.text),
                ("header", t.muted),
            ] {
                let k = contrast(c, t.bg);
                assert!(k >= 4.5, "{} {}: {what} {k:.2}", look.name, t.name);
            }
        }
    }
}

/// Cost of the story chart over the plain compile (release: `cargo test --release -p
/// qp-grammar --test stories_korda -- --ignored --nocapture`).
#[test]
#[ignore = "timing probe"]
fn timing_probe() {
    let p = provider();
    let v = view();
    let opts = CompileOptions::new(390, 300);
    let k = 300;
    for n in [0u64, 5, 50, 200] {
        let rs = rows(n);
        let t0 = std::time::Instant::now();
        for _ in 0..k {
            std::hint::black_box(qp_grammar::compile(&v, &p, &opts).unwrap());
        }
        let t1 = std::time::Instant::now();
        for _ in 0..k {
            std::hint::black_box(draw(&rs, "f1", true, Language::Tr));
        }
        let t2 = std::time::Instant::now();
        eprintln!(
            "n={n}: compile {:?} korda {:?}",
            (t1 - t0) / k,
            (t2 - t1) / k
        );
    }
}

/// (RP4/T7) With the rows' time (`StoryInput::stamp_of`) every lane's label carries its
/// stamp (after what happened) and nothing else changes; without it the scene is byte-identical to before.
#[test]
fn rows_carry_their_time_only_when_asked() {
    let rs = rows(5);
    let plain = draw(&rs, "f1", true, Language::Tr);
    let stamp = |r: &StoryRow| Some(format!("{:02}:30", 10 + r.born_bar % 8));
    let s = korda::scene(
        &view(),
        &provider(),
        &CompileOptions::new(390, 300),
        &StoryInput {
            rows: &rs,
            focus: Some("f1"),
            hide_closed: false,
            lang: Language::Tr,
            fact_of: &|_| None,
            stamp_of: Some(&stamp),
        },
    )
    .expect("scene");
    let (a, b) = (lanes(&plain), lanes(&s));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        let r = rs.iter().find(|r| x.id.ends_with(&r.id)).expect("row");
        let plain = x.label.as_deref().unwrap();
        let t = stamp(r).unwrap();
        // After what happened and how it stands, before the coin-flip mark.
        let want = match plain.strip_suffix(" · yazı tura") {
            Some(head) => format!("{head} · {t} · yazı tura"),
            None => format!("{plain} · {t}"),
        };
        assert_eq!(y.label.as_deref(), Some(want.as_str()));
        assert_eq!(x.items, y.items, "the lane's geometry is the same");
    }
    // The drawn labels carry it too (where the room allows).
    let texts: Vec<&str> = panel(&s)
        .layers
        .iter()
        .flat_map(|l| &l.prims)
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.ends_with(":30")), "{texts:?}");
    // No stamp for a row (a redacted one): its label as before.
    let none = |_: &StoryRow| None;
    let s2 = korda::scene(
        &view(),
        &provider(),
        &CompileOptions::new(390, 300),
        &StoryInput {
            rows: &rs,
            focus: Some("f1"),
            hide_closed: false,
            lang: Language::Tr,
            fact_of: &|_| None,
            stamp_of: Some(&none),
        },
    )
    .expect("scene");
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        serde_json::to_string(&s2).unwrap()
    );
}
