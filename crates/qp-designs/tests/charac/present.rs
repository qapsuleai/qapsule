//! The presenter's depth rules (sentence / capsule / chart, per face), the depth transition
//! designs' frames, the motion designs (change frames, stamps, live marks) and the feed
//! layouts.

use qp_designs::faces;
use qp_designs::feeds::{self, FeedCtx, LocalStamp};
use qp_designs::motion::{self, ChangeKind};
use qp_designs::transitions::{self, nabiz, Side};
use qp_grammar::{depth, MemProvider};
use qp_look::GrammarError;
use qp_schema::capsule::Direction;
use qp_schema::protocol::{Language, OpenDepth, Profile};
use qp_schema::scene::SceneDesc;
use qp_schema::Depth;

use crate::designs::face_scene;
use crate::fixture::{self, compile_in, look, looks, shape};
use crate::snap::{debug_digest, err_line, scene_digest, Book};

const DEPTHS: [Depth; 4] = [
    Depth::Sentence,
    Depth::Capsule,
    Depth::Chart,
    Depth::Workspace,
];
const OPEN_DEPTHS: [OpenDepth; 4] = [
    OpenDepth::Sentence,
    OpenDepth::Capsule,
    OpenDepth::Chart,
    OpenDepth::Evidence,
];

fn profile(depth: Depth, series: bool, points: u32) -> Profile {
    Profile {
        depth,
        series_data: series,
        max_points: points,
        max_glyphs: 4,
        speech: !series,
    }
}

fn capsule_scene(
    p: &MemProvider,
    l: &str,
    f: &str,
    face: Option<&str>,
) -> Result<SceneDesc, GrammarError> {
    let v = depth::capsule_view_in(look(l), f, 120, p, &format!("capsule-{f}"))?;
    compile_in(&v, p, l, 390, 240, face)
}

fn chart_scene(
    p: &MemProvider,
    l: &str,
    f: &str,
    face: Option<&str>,
) -> Result<SceneDesc, GrammarError> {
    let v = depth::chart_view_in(look(l), f, p, &format!("present-{f}"))?;
    compile_in(&v, p, l, 900, 520, face)
}

fn scene_line(b: &mut Book, case: &str, r: Result<SceneDesc, GrammarError>) {
    match r {
        Ok(s) => b.put(case, &scene_digest(&s), &shape(&s)),
        Err(e) => b.note(case, &err_line(&e)),
    }
}

#[test]
fn charac_depth_rules() {
    let mut b = Book::new("depth");
    let p = fixture::provider(Language::Tr);
    let profiles = [
        ("desktop", profile(Depth::Workspace, true, 300)),
        ("phone", profile(Depth::Capsule, true, 120)),
        ("nodata", profile(Depth::Chart, false, 0)),
        ("voice", profile(Depth::Sentence, false, 0)),
    ];
    for (pn, pr) in &profiles {
        for d in DEPTHS {
            b.note(
                &format!("effective.{pn}.{d:?}"),
                &format!("{:?}", depth::effective_depth(d, pr)),
            );
        }
    }
    let mut findings = fixture::capsule_findings();
    findings.extend(["fnd-rsi", "fnd-two", "fnd-missing", "nope"]);
    for f in &findings {
        b.note(
            &format!("sentence.{f}"),
            &depth::sentence(f, &p).unwrap_or_else(|e| err_line(&e)),
        );
        for (pn, pr) in &profiles {
            for d in DEPTHS {
                let case = format!("present.{f}.{pn}.{d:?}");
                match depth::present(f, d, pr, &p) {
                    Ok(x) => b.put(
                        &case,
                        &debug_digest(&x),
                        &format!(
                            "{:?} conf={:?} view={} text={}",
                            x.depth,
                            x.confidence,
                            x.view.as_ref().map_or(0, |v| v.layers.len()),
                            x.text
                        ),
                    ),
                    Err(e) => b.note(&case, &err_line(&e)),
                }
            }
        }
    }
    // The three depths' scenes per look, and per face (the face's headline template
    // reaches every capsule text of the scene).
    for l in looks() {
        for f in &findings {
            if !matches!(
                *f,
                "fnd-brk" | "fnd-sq" | "fnd-div" | "fnd-lock" | "fnd-rsi"
            ) && l != "r4"
            {
                continue;
            }
            scene_line(
                &mut b,
                &format!("capsule.{l}.{f}"),
                capsule_scene(&p, l, f, None),
            );
            scene_line(
                &mut b,
                &format!("chart.{l}.{f}"),
                chart_scene(&p, l, f, None),
            );
            if let Ok(v) = depth::capsule_card_view_in(look(l), f, 120, &p, "card") {
                b.put(
                    &format!("card_view.{l}.{f}"),
                    &debug_digest(&v),
                    &format!("{} layers", v.layers.len()),
                );
            }
        }
    }
    for face in faces::names() {
        for f in ["fnd-brk", "fnd-parts", "fnd-sq"] {
            scene_line(
                &mut b,
                &format!("face.{face}.{f}.sentence"),
                face_scene(&p, face, "r4", f),
            );
            scene_line(
                &mut b,
                &format!("face.{face}.{f}.capsule"),
                capsule_scene(&p, "r4", f, Some(face)),
            );
            scene_line(
                &mut b,
                &format!("face.{face}.{f}.chart"),
                chart_scene(&p, "r4", f, Some(face)),
            );
        }
    }
    for d in OPEN_DEPTHS {
        b.note(
            &format!("open.{d:?}"),
            &format!(
                "deeper={:?} shallower={:?} presenter={:?} tap={:?}",
                depth::deeper(d),
                depth::shallower(d),
                depth::presenter_depth(d),
                depth::tap_opens(d, None)
            ),
        );
    }
    // Anchors of each depth's scene and the shared elements between them.
    let sc = face_scene(&p, "oltan", "r4", "fnd-brk").unwrap();
    let ca = capsule_scene(&p, "r4", "fnd-brk", None).unwrap();
    let ch = chart_scene(&p, "r4", "fnd-brk", None).unwrap();
    let anchors: Vec<_> = [&sc, &ca, &ch]
        .iter()
        .map(|s| depth::transition_anchors(s, Some("px"), Some(180)))
        .collect();
    for (n, a) in ["sentence", "capsule", "chart"].iter().zip(&anchors) {
        b.put(
            &format!("anchors.{n}"),
            &debug_digest(a),
            &a.keys().cloned().collect::<Vec<_>>().join(","),
        );
    }
    for (i, j) in [(0, 1), (1, 2), (2, 1), (0, 2)] {
        let s = depth::shared_elements(Some(&anchors[i]), Some(&anchors[j]));
        b.put(
            &format!("shared.{i}{j}"),
            &debug_digest(&s),
            &format!("{} elements", s.len()),
        );
    }
    let s = depth::shared_elements(None, Some(&anchors[1]));
    b.put(
        "shared.none1",
        &debug_digest(&s),
        &format!("{} elements", s.len()),
    );
    b.check();
}

#[test]
fn charac_transition_frames() {
    let mut b = Book::new("transitions");
    let p = fixture::provider(Language::Tr);
    let sc = face_scene(&p, "oltan", "r4", "fnd-brk").unwrap();
    let ca = capsule_scene(&p, "r4", "fnd-brk", None).unwrap();
    let ch = chart_scene(&p, "r4", "fnd-brk", None).unwrap();
    let side = |depth, scene| Side {
        depth,
        scene,
        price_series: Some("px"),
        marked_bar: Some(180),
    };
    let pairs = [
        (
            "sentence-capsule",
            side(OpenDepth::Sentence, Some(&sc)),
            side(OpenDepth::Capsule, Some(&ca)),
        ),
        (
            "capsule-chart",
            side(OpenDepth::Capsule, Some(&ca)),
            side(OpenDepth::Chart, Some(&ch)),
        ),
        (
            "chart-capsule",
            side(OpenDepth::Chart, Some(&ch)),
            side(OpenDepth::Capsule, Some(&ca)),
        ),
        (
            "capsule-sentence",
            side(OpenDepth::Capsule, Some(&ca)),
            side(OpenDepth::Sentence, Some(&sc)),
        ),
        (
            "bare-capsule",
            side(OpenDepth::Sentence, None),
            side(OpenDepth::Capsule, Some(&ca)),
        ),
    ];
    for t in transitions::TRANSITIONS {
        for (n, before, after) in &pairs {
            let shared = (t.shared)(before, after);
            let ids: Vec<&str> = shared.iter().map(|s| s.id.as_str()).collect();
            b.put(
                &format!("{}.{n}.shared", t.name),
                &debug_digest(&shared),
                &ids.join(","),
            );
            for pr in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let fr = (t.frame)(&shared, (0.0, 120.0), (0.0, 0.0), pr);
                b.put(
                    &format!("{}.{n}.p{pr}", t.name),
                    &debug_digest(&fr),
                    &format!(
                        "{} els scene_opacity={}",
                        fr.elements.len(),
                        fr.scene_opacity
                    ),
                );
            }
        }
        for l in looks() {
            b.note(
                &format!("{}.timing.{l}", t.name),
                &format!(
                    "dur={} p(0)={} p(100)={} p(1000)={}",
                    transitions::duration_in(look(l), Some(t.name)),
                    transitions::progress_in(look(l), Some(t.name), 0.0, 0.0),
                    transitions::progress_in(look(l), Some(t.name), 0.0, 100.0),
                    transitions::progress_in(look(l), Some(t.name), 0.0, 1000.0)
                ),
            );
        }
        b.note(
            &format!("{}.progress", t.name),
            &format!("{}", transitions::progress(Some(t.name), 0.0, 130.0)),
        );
    }
    let a = transitions::anchors(&pairs[1].1);
    b.put("anchors.capsule", &debug_digest(&a), "-");
    b.note(
        "registry",
        &format!(
            "{:?} default={} presented={} unknown={}",
            transitions::names(),
            transitions::DEFAULT_TRANSITION,
            transitions::PRESENTED_TRANSITION,
            transitions::by_name(Some("nope")).name
        ),
    );
    b.check();
}

#[test]
fn charac_motion_designs() {
    let mut b = Book::new("motion");
    let p = fixture::provider(Language::Tr);
    for m in motion::MOTIONS {
        b.note(
            &format!("design.{}", m.name),
            &format!(
                "transition={} marks={} live={}",
                m.transition,
                m.marks,
                m.live.is_some()
            ),
        );
    }
    let before = face_scene(&p, "oltan", "r4", "fnd-parts").unwrap();
    let after = face_scene(&p, "oltan", "r4", "fnd-brk").unwrap();
    for k in ChangeKind::ALL {
        for (n, bf) in [("before", Some(&before)), ("none", None)] {
            let ch = motion::face_change(k, bf, &after);
            b.put(&format!("change.{}.{n}", k.name()), &debug_digest(&ch), "-");
            for pr in [0.0, 0.5, 1.0] {
                let fr = motion::change_frame(&ch, 10.0, 20.0, pr);
                b.put(
                    &format!("change.{}.{n}.p{pr}", k.name()),
                    &debug_digest(&fr),
                    &format!("{} els", fr.elements.len()),
                );
            }
        }
        for l in looks() {
            let mut s = face_scene(&p, "oltan", l, "fnd-brk").unwrap();
            motion::stamp(&mut s, k, Language::Tr, look(l));
            b.put(
                &format!("stamp.{}.{l}", k.name()),
                &scene_digest(&s),
                motion::label(k, Language::En),
            );
        }
    }
    for l in looks() {
        b.note(
            &format!("timing.{l}"),
            &format!(
                "dur={} p(100)={}",
                motion::change_duration(look(l)),
                motion::change_progress(look(l), 0.0, 100.0)
            ),
        );
    }
    let items = fixture::feed_items();
    for (i, old) in items.iter().enumerate() {
        for new in &items {
            let k = motion::classify(Some(old), new, Some(0));
            if k.is_some() || i == 0 {
                b.note(
                    &format!("classify.{}.{}", old.finding, new.finding),
                    &format!("{k:?}"),
                );
            }
        }
        b.note(
            &format!("classify.none.{}", old.finding),
            &format!("{:?}", motion::classify(None, old, Some(0))),
        );
    }
    // nabiz's live marks: still marks into each face, overlay boxes at a few clocks.
    let live = motion::by_name(Some("nabiz")).live.unwrap();
    for l in ["r4", "tavek", "oltan"] {
        let lk = look(l);
        let base = face_scene(&p, "oltan", l, "fnd-brk").unwrap();
        let places = (live.places)(&base, lk);
        b.put(
            &format!("nabiz.places.{l}"),
            &debug_digest(&places),
            &format!("{}", places.is_some()),
        );
        for change in [
            None,
            Some(nabiz::Change::Born),
            Some(nabiz::Change::Strengthened),
            Some(nabiz::Change::Weakened),
            Some(nabiz::Change::Invalid),
            Some(nabiz::Change::Closed),
        ] {
            for (lv, word) in [(true, None), (true, Some("canlı")), (false, None)] {
                let mut s = base.clone();
                let marks = nabiz::FaceMarks {
                    live: lv,
                    label: change.map(|_| "az önce"),
                    change,
                    direction: Direction::Up,
                    live_word: word,
                };
                (live.draw_face)(&mut s, &marks, lk, &lk.dark);
                b.put(
                    &format!("nabiz.face.{l}.{change:?}.{lv}.{word:?}"),
                    &scene_digest(&s),
                    &format!("over{}", s.over.len()),
                );
            }
            if let Some(pl) = places {
                for t in [0.0, 400.0, 1500.0] {
                    let fm = nabiz::FaceMotion {
                        places: pl,
                        live: true,
                        direction: Direction::Down,
                        beat: change.map(|c| nabiz::Beat {
                            change: c,
                            age: t / 2.0,
                        }),
                        label_w: 40,
                        t,
                    };
                    let o = (live.overlay)(&fm, lk, &lk.dark);
                    b.put(
                        &format!("nabiz.overlay.{l}.{change:?}.{t}"),
                        &debug_digest(&o),
                        &format!("{} boxes", o.len()),
                    );
                }
            }
        }
    }
    b.check();
}

#[test]
fn charac_feed_layouts() {
    let mut b = Book::new("feeds");
    let items = fixture::feed_items();
    let fresh: Vec<bool> = (0..items.len()).map(|i| i % 3 == 0).collect();
    let local: Vec<LocalStamp> = items
        .iter()
        .map(|i| LocalStamp::at_offset(i.t, 3 * 3_600_000))
        .collect();
    let now = items.last().unwrap().t + 86_400_000;
    for lang in [Language::Tr, Language::En] {
        for (n, seen) in [("unseen", None), ("seen", Some(items[4].t))] {
            let ctx = FeedCtx {
                items: &items,
                lang,
                seen,
                fresh: &fresh,
                now: Some(now),
                local: &local,
                now_local: Some(LocalStamp::at_offset(now, 3 * 3_600_000)),
                seen_local: seen.map(|t| LocalStamp::at_offset(t, 3 * 3_600_000)),
            };
            for f in feeds::LAYOUTS {
                let rows = feeds::layout(Some(f.name), &ctx);
                b.put(
                    &format!("layout.{}.{lang:?}.{n}", f.name),
                    &debug_digest(&rows),
                    &format!("{rows:?}"),
                );
            }
            for f in feeds::FEED_ORDERS {
                let rows = feeds::order(f, &ctx);
                b.put(
                    &format!("order.{f}.{lang:?}.{n}"),
                    &debug_digest(&rows),
                    &format!("{} rows", rows.map_or(0, |r| r.len())),
                );
            }
        }
        b.note(
            &format!("empty.{lang:?}"),
            &format!(
                "{:?} {:?}",
                feeds::empty_rows(lang, 0),
                feeds::empty_rows(lang, 3)
            ),
        );
    }
    b.note(
        "registry",
        &format!(
            "{:?} default={} presented={} order={}",
            feeds::names(),
            feeds::DEFAULT_FEED,
            feeds::PRESENTED_FEED,
            feeds::PRESENTED_FEED_ORDER
        ),
    );
    b.check();
}
