//! R5/M20 `nobra` (question cards, `qp_designs::open::nobra`): the open capsule design a
//! connection chooses with `style {open}`.
//!
//! - `style {open}`: `r4` (the default, today's card) and `nobra`; an unknown name is
//!   `invalid` with the known ones; the view state remembers it.
//! - At the capsule depth every question card answers in one line (no tap); the device's
//!   scene (`viewer::open_scene_in`) and the authority's (the one `tap_at` reads) have the
//!   same cards in the same places: a tap on a card opens its detail (`state.ask`), a second
//!   tap closes it, a tap on the evidence chart opens the chart; nothing else opens.
//! - "Who verified it?" says what the device's check found (K48), in the same box.
//! - A reader whose evidence values are withheld gets "Why can't I see numbers?" and no
//!   digit in any question text (K33/K34, A32).
//! - Under `r4` the capsule depth is today's card, byte for byte.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_designs::open::Check;
use qp_schema::protocol::{
    CapsuleItem, Language, Opening, OutboundLine, ReplayInit, ReplaySourceKind, ViewState,
};
use qp_schema::scene::{Prim, SceneDesc};
use qp_session::viewer::{open_scene, open_scene_with, OpenExtra, TrustState};
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

/// (R5/S) The reader `nobra` was measured with: its language and the device's check.
struct OpenReader {
    lang: Language,
    check: Option<Check>,
}

impl Default for OpenReader {
    fn default() -> Self {
        Self {
            lang: Language::Tr,
            check: None,
        }
    }
}

/// (R5/S) The device's scene for `r` (`viewer::open_scene_with` with its `OpenExtra`).
fn open_scene_in(
    s: &mut qp_session::Session,
    o: &qp_schema::protocol::Opening,
    headline: Option<&str>,
    voice: Option<&qp_schema::protocol::CapsuleVoice>,
    look: &ViewState,
    max_points: u32,
    r: &OpenReader,
) -> Option<SceneDesc> {
    let check = match r.check {
        None => TrustState::Server,
        Some(Check::Checking) => TrustState::Checking,
        Some(Check::Device) => TrustState::Device,
        Some(Check::Differs) => TrustState::Differs,
    };
    let extra = OpenExtra {
        lang: Some(r.lang),
        check: Some(check),
    };
    open_scene_with(
        s,
        o,
        headline,
        voice,
        look,
        max_points,
        (&[], Language::Tr),
        extra,
    )
}

const W: u32 = 390;
const H: u32 = 620;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn host() -> ReplayHost {
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    ent.insert("visitor".to_owned(), Vec::new());
    let mut init = demo_init(ReplayInit {
        source: ReplaySourceKind::Synthetic,
        instruments: Some(8),
        days: Some(260),
        history: Some(120),
        fine: Some("1h".to_owned()),
        entitlements: ent,
        ..ReplayInit::default()
    });
    init.principals.push(
        serde_json::from_value(json!(
            {"id": "visitor", "author": {"kind": "human", "name": "visitor"}, "role": "owner"}))
        .expect("visitor"),
    );
    let none = std::env::temp_dir().join("qp-nobra-no-local-data");
    let mut h = ReplayHost::with_local_dir(init, &root(), &none).expect("demo host");
    for _ in 0..80 {
        assert!(h.take_step().is_some());
    }
    h
}

fn send(h: &mut ReplayHost, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    h.handle_raw(conn, &msg.to_string())
}

fn result(out: &[OutboundLine], conn: &str, id: &str) -> Value {
    for l in out.iter().filter(|l| l.conn == conn) {
        let m = serde_json::to_value(&l.msg).unwrap();
        if m["id"] == id {
            return m;
        }
    }
    panic!("no answer to {id}: {out:?}");
}

fn hello(h: &mut ReplayHost, conn: &str, who: &str, lang: &str) {
    send(
        h,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,
                "surface":{"kind":"phone","lang":lang}}),
    );
}

fn input(h: &mut ReplayHost, conn: &str, g: Value) -> Value {
    let out = send(h, conn, &json!({"type":"input","id":"g","gesture":g}));
    result(&out, conn, "g")
}

fn capsule_ids(h: &mut ReplayHost, conn: &str) -> Vec<(String, String, bool)> {
    let out = send(
        h,
        conn,
        &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
    );
    let m = result(&out, conn, "q");
    m["data"]["capsules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["id"].as_str().unwrap().to_owned(),
                c["finding"].as_str().unwrap().to_owned(),
                c["redacted"].as_bool().unwrap_or(false),
            )
        })
        .collect()
}

/// Opens capsule `cap` to its capsule depth; the opening and the view state.
fn open(h: &mut ReplayHost, conn: &str, cap: &str) -> (Opening, ViewState) {
    let m = input(
        h,
        conn,
        json!({"kind":"open","capsule":cap,"to":"capsule","width":W,"height":H}),
    );
    let i = &m["data"]["interaction"];
    (
        serde_json::from_value(i["opening"].clone()).unwrap_or_else(|e| panic!("{e}: {m}")),
        serde_json::from_value(i["state"].clone()).unwrap(),
    )
}

fn texts(s: &SceneDesc) -> Vec<String> {
    s.prims()
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn panel_center(s: &SceneDesc, name: &str) -> (f64, f64) {
    let p = s
        .panels
        .iter()
        .find(|p| p.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no panel {name}"));
    (
        f64::from(p.rect.left + p.rect.right) / 2.0,
        f64::from(p.rect.top + p.rect.bottom) / 2.0,
    )
}

fn tap(h: &mut ReplayHost, conn: &str, cap: &str, (x, y): (f64, f64)) -> Value {
    let m = input(
        h,
        conn,
        json!({"kind":"tap_at","target":{"capsule":cap},"x":x,"y":y,"width":W,"height":H}),
    );
    m["data"]["interaction"].clone()
}

fn digits(s: &str) -> usize {
    s.bytes().filter(u8::is_ascii_digit).count()
}

#[test]
fn style_open_is_validated_and_remembered() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    let m = input(&mut h, "a", json!({"kind":"style","open":"nope"}));
    assert_eq!(m["type"], "error", "{m}");
    let msg = m.to_string();
    assert!(msg.contains("r4") && msg.contains("nobra"), "{msg}");
    let m = input(&mut h, "a", json!({"kind":"style","open":"nobra"}));
    assert_eq!(m["data"]["interaction"]["state"]["open"], "nobra", "{m}");
    let m = input(&mut h, "a", json!({"kind":"style","open":"r4"}));
    assert!(
        m["data"]["interaction"]["state"].get("open").is_none(),
        "{m}"
    );
}

#[test]
fn question_cards_answer_and_open_on_touch() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    input(&mut h, "a", json!({"kind":"style","open":"nobra"}));
    let (cap, _, _) = capsule_ids(&mut h, "a")
        .into_iter()
        .next()
        .expect("a capsule");
    let (o, state) = open(&mut h, "a", &cap);
    // The design states its height (560 px in `r4`); the width is the surface's.
    assert_eq!(
        o.frame,
        Some([W, qp_designs::open::nobra::height(&qp_look::look::R4)])
    );
    assert_eq!(o.frame, Some([W, 560]));
    let reader = OpenReader::default();
    let scene =
        open_scene_in(h.session_mut(), &o, None, None, &state, 60, &reader).expect("a scene");
    let names: Vec<&str> = scene
        .panels
        .iter()
        .filter_map(|p| p.name.as_deref())
        .collect();
    assert_eq!(
        names,
        [
            "ask:what",
            "ask:who",
            "ask:before",
            "ask:basis",
            "ask:chart"
        ]
    );
    let drawn = texts(&scene).join(" | ");
    for q in [
        "Ne oldu?",
        "Kim doğruladı?",
        "Daha önce kaç kez?",
        "Neye dayanıyor?",
        "Sunucu doğruladı.",
    ] {
        assert!(drawn.contains(q), "{q}: {drawn}");
    }
    assert!(!drawn.contains("Neden rakam"), "{drawn}");
    // Every card stays inside the frame; the chart is the last panel.
    for p in &scene.panels {
        assert!(p.rect.top >= 0 && p.rect.bottom <= i32::try_from(H).unwrap());
        assert!(p.rect.bottom > p.rect.top);
    }
    // Deterministic.
    let again = open_scene_in(h.session_mut(), &o, None, None, &state, 60, &reader).unwrap();
    assert_eq!(again, scene);

    // A tap on "who": its detail opens (the authority's scene has the card in the same
    // place), nothing opens.
    let i = tap(&mut h, "a", &cap, panel_center(&scene, "ask:who"));
    assert!(i.get("opening").is_none(), "{i}");
    assert_eq!(i["state"]["ask"], "who", "{i}");
    let state: ViewState = serde_json::from_value(i["state"].clone()).unwrap();
    let opened = open_scene_in(h.session_mut(), &o, None, None, &state, 60, &reader).unwrap();
    let drawn = texts(&opened).join(" ");
    assert!(drawn.contains("bu cihaz aynı kuralı"), "{drawn}");
    // A tap on another card moves the detail there; a second tap closes it.
    let i = tap(&mut h, "a", &cap, panel_center(&opened, "ask:before"));
    assert_eq!(i["state"]["ask"], "before", "{i}");
    let state: ViewState = serde_json::from_value(i["state"].clone()).unwrap();
    let opened = open_scene_in(h.session_mut(), &o, None, None, &state, 60, &reader).unwrap();
    let i = tap(&mut h, "a", &cap, panel_center(&opened, "ask:before"));
    assert!(i["state"].get("ask").is_none(), "{i}");
    // The title row opens nothing.
    let i = tap(&mut h, "a", &cap, (100.0, 10.0));
    assert!(i.get("opening").is_none(), "{i}");
    // A tap on the evidence chart opens the chart.
    let i = tap(&mut h, "a", &cap, panel_center(&scene, "ask:chart"));
    assert_eq!(i["opening"]["to"], "chart", "{i}");
    // Closing to the feed forgets the opened question.
    let m = input(&mut h, "a", json!({"kind":"close","to":"sentence"}));
    assert!(m["data"]["interaction"]["state"].get("ask").is_none());
}

#[test]
fn the_device_check_is_written_in_the_same_box() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "en");
    input(&mut h, "a", json!({"kind":"style","open":"nobra"}));
    let (cap, _, _) = capsule_ids(&mut h, "a").into_iter().next().unwrap();
    let (o, _) = open(&mut h, "a", &cap);
    let i = tap(&mut h, "a", &cap, (195.0, 80.0));
    let mut state: ViewState = serde_json::from_value(i["state"].clone()).unwrap();
    state.ask = Some("who".into());
    let mut boxes = Vec::new();
    for (c, want) in [
        (None, "Verified by the server."),
        (Some(Check::Checking), "checking on this device"),
        (Some(Check::Device), "checked on this device too"),
        (Some(Check::Differs), "Sources differ"),
    ] {
        let r = OpenReader {
            lang: Language::En,
            check: c,
        };
        let s = open_scene_in(h.session_mut(), &o, None, None, &state, 60, &r).unwrap();
        let drawn = texts(&s).join(" ");
        assert!(drawn.contains(want), "{want}: {drawn}");
        assert!(drawn.contains("Who verified it?"), "{drawn}");
        boxes.push(
            s.panels
                .iter()
                .map(|p| (p.name.clone(), p.rect))
                .collect::<Vec<_>>(),
        );
    }
    assert!(boxes.windows(2).all(|w| w[0] == w[1]), "{boxes:?}");
}

#[test]
fn a_withheld_reader_is_told_why_and_sees_no_digit_in_the_questions() {
    for (lang, voice, why_q) in [
        ("tr", "r4", "Neden rakam göremiyorum?"),
        ("tr", "marla", "Neden rakam göremiyorum?"),
        ("en", "selvi", "Why can't I see numbers?"),
    ] {
        let mut h = host();
        hello(&mut h, "v", "visitor", lang);
        input(
            &mut h,
            "v",
            json!({"kind":"style","open":"nobra","voice":voice}),
        );
        let out = send(
            &mut h,
            "v",
            &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
        );
        let m = result(&out, "v", "q");
        let items: Vec<CapsuleItem> =
            serde_json::from_value(m["data"]["capsules"].clone()).unwrap();
        assert!(!items.is_empty());
        let l = if lang == "en" {
            Language::En
        } else {
            Language::Tr
        };
        let mut texted = 0;
        for it in items.iter().take(8) {
            let (o, _) = open(&mut h, "v", &it.id);
            // This reader has no data of the instrument: no scene; the page lists the
            // design's question cards as text (A32).
            assert!(o.frame.is_none(), "a scene for the visitor");
            let rows = qp_designs::open::text_rows(Some("nobra"), o.to, it, l);
            let keys: Vec<&str> = rows.iter().map(|r| r.key).collect();
            assert_eq!(keys, ["what", "who", "before", "why", "basis"]);
            assert_eq!(rows[3].q, why_q);
            let label = digits(&it.instrument);
            for r in &rows {
                for t in [&r.q, &r.a, &r.d] {
                    assert!(!t.is_empty());
                    assert_eq!(digits(&t.replace(&it.instrument, "")), 0, "{t}");
                    assert!(digits(t) <= label * 2, "{t}");
                }
            }
            // The reference design lists none.
            assert!(qp_designs::open::text_rows(None, o.to, it, l).is_empty());
            texted += 1;
            input(&mut h, "v", json!({"kind":"close","to":"sentence"}));
        }
        assert!(texted > 0);
    }
}

#[test]
fn under_r4_the_capsule_depth_is_todays_card() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    let (cap, _, _) = capsule_ids(&mut h, "a").into_iter().next().unwrap();
    let (o, state) = open(&mut h, "a", &cap);
    let a = open_scene(h.session_mut(), &o, None, None, &state, 60).unwrap();
    let b = open_scene_in(
        h.session_mut(),
        &o,
        None,
        None,
        &state,
        60,
        &OpenReader {
            lang: Language::En,
            check: Some(Check::Device),
        },
    )
    .unwrap();
    assert_eq!(a, b);
    assert!(a
        .panels
        .iter()
        .all(|p| p.name.as_deref().is_none_or(|n| !n.starts_with("ask:"))));
    // A tap anywhere opens the chart, as today.
    let i = tap(&mut h, "a", &cap, (195.0, 200.0));
    assert_eq!(i["opening"]["to"], "chart", "{i}");
}
