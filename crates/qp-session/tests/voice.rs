//! R5/m17-int (`docs/LANGUAGE.md`): the capsule voice a connection chooses.
//!
//! - `style {voice}`: `r4` (the default, today's text), `marla`, `selvi`; an unknown name
//!   is `invalid` with the known ones. The view state remembers it.
//! - Under a voice every listed capsule carries the voice's four surfaces in the reader's
//!   language (`CapsuleItem.voice`): headline, detail, ledger sentence, glossary, and the
//!   text face of a reader without a face scene. Length limits hold; no internal term; a
//!   redacted reader's texts carry no digit (the instrument label aside; A32, K33).
//! - The detail is built from the face's pieces (A33): the headline and the detail never
//!   contradict the item's validity.
//! - The face scene speaks from the voice (headline and ledger), its glossary terms are
//!   marked (`SceneDesc.terms`, underlined); a tap on a term answers with the term
//!   (`hit.term`) and opens nothing; a tap elsewhere opens the capsule (K49).
//! - Under `r4` nothing changes: no `voice` on the items, no terms in the scenes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::{CapsuleItem, OutboundLine, ReplayInit, ReplaySourceKind, ViewState};
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The demo world (synthetic, as `qp demo`), small; `visitor` reads without data.
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
    let none = std::env::temp_dir().join("qp-voice-no-local-data");
    ReplayHost::with_local_dir(init, &root(), &none).expect("demo host")
}

fn send(h: &mut ReplayHost, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    h.handle_raw(conn, &msg.to_string())
}

/// The `result` data answering request `id` on `conn` among `out`.
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

fn style(h: &mut ReplayHost, conn: &str, g: Value) -> Value {
    let mut g = g;
    g["kind"] = json!("style");
    let out = send(h, conn, &json!({"type":"input","id":"st","gesture":g}));
    result(&out, conn, "st")
}

fn capsules(h: &mut ReplayHost, conn: &str) -> Vec<CapsuleItem> {
    let out = send(
        h,
        conn,
        &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
    );
    let m = result(&out, conn, "q");
    serde_json::from_value(m["data"]["capsules"].clone()).unwrap_or_else(|e| panic!("{e}: {m}"))
}

fn digits(s: &str) -> usize {
    s.bytes().filter(u8::is_ascii_digit).count()
}

#[test]
fn style_voice_is_validated_and_remembered() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    let m = style(&mut h, "a", json!({"voice":"nope"}));
    assert_eq!(m["type"], "error", "{m}");
    let msg = m.to_string();
    for n in ["r4", "marla", "selvi"] {
        assert!(msg.contains(n), "{msg}");
    }
    assert!(!msg.contains("tunca"), "{msg}");
    let m = style(&mut h, "a", json!({"voice":"marla"}));
    assert_eq!(m["data"]["interaction"]["state"]["voice"], "marla", "{m}");
    // The default is not written (bare view states stay as they were).
    let m = style(&mut h, "a", json!({"voice":"r4"}));
    assert!(
        m["data"]["interaction"]["state"].get("voice").is_none(),
        "{m}"
    );
}

#[test]
fn every_listed_capsule_speaks_in_the_chosen_voice() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    hello(&mut h, "e", "arkin", "en");
    hello(&mut h, "v", "visitor", "tr");
    for _ in 0..60 {
        assert!(h.take_step().is_some());
    }
    // r4 (the default): no voice anywhere.
    for c in ["a", "e", "v"] {
        assert!(capsules(&mut h, c).iter().all(|i| i.voice.is_none()));
    }
    for voice in ["marla", "selvi"] {
        for c in ["a", "e", "v"] {
            style(&mut h, c, json!({ "voice": voice }));
        }
        let (tr, en, vis) = (
            capsules(&mut h, "a"),
            capsules(&mut h, "e"),
            capsules(&mut h, "v"),
        );
        assert!(!tr.is_empty() && tr.len() == en.len());
        let mut redacted = 0;
        for (lang, items) in [("tr", &tr), ("en", &en), ("tr", &vis)] {
            for it in items {
                let v = it
                    .voice
                    .as_ref()
                    .unwrap_or_else(|| panic!("{voice}: {}", it.id));
                assert_eq!(v.voice, voice);
                let n = |s: &str| s.chars().count();
                assert!(n(&v.headline) <= 80, "{}", v.headline);
                assert!(n(&v.detail) <= 200, "{}", v.detail);
                assert!(n(&v.ledger) <= 70, "{}", v.ledger);
                assert_eq!(
                    v.face,
                    format!("{}\n{}\n{}", v.headline, v.ledger, v.detail)
                );
                let all = format!("{} {} {}", v.headline, v.detail, v.ledger).to_lowercase();
                for bad in [" bar ", " bars ", "aile", "teyit", "family", "sinyal"] {
                    assert!(!all.contains(bad), "{voice} {lang}: {bad} in {all}");
                }
                // A33: the headline says what the item's validity says.
                let h = v.headline.to_lowercase();
                match (it.validity, lang) {
                    (qp_schema::capsule::Validity::Valid, "tr") => {
                        assert!(!h.contains("artık"), "{h}");
                    }
                    (qp_schema::capsule::Validity::NoLonger, "tr") => {
                        assert!(h.contains("artık"), "{h}");
                    }
                    (qp_schema::capsule::Validity::Closed, "tr") => {
                        assert!(!h.contains("hâlâ geçerli") && !h.contains("artık"), "{h}");
                    }
                    (qp_schema::capsule::Validity::Valid, _) => {
                        assert!(!h.contains("no longer"), "{h}");
                    }
                    (qp_schema::capsule::Validity::NoLonger, _) => {
                        assert!(h.contains("no longer"), "{h}");
                    }
                    (qp_schema::capsule::Validity::Closed, _) => {
                        assert!(!h.contains("still holds"), "{h}");
                    }
                }
                if it.redacted {
                    redacted += 1;
                    let label = digits(&it.instrument);
                    for t in [&v.headline, &v.detail, &v.ledger] {
                        assert!(
                            digits(t) <= label,
                            "{voice}: digits for a redacted reader: {t}"
                        );
                        assert_eq!(digits(&t.replace(&it.instrument, "")), 0, "{t}");
                    }
                    for g in &v.glossary {
                        assert_eq!(digits(&g.text), 0, "{}", g.text);
                    }
                }
            }
        }
        assert!(redacted > 0, "{voice}: no redacted reader's capsule");
        // Deterministic.
        assert_eq!(capsules(&mut h, "a"), tr);
    }
}

/// The face scene of the first capsule with a glossary term drawn, under `voice`
/// (connection `a`'s look), from the shared scene function.
fn voiced_face(
    h: &mut ReplayHost,
    items: &[CapsuleItem],
    voice: &str,
) -> Option<(CapsuleItem, qp_schema::scene::SceneDesc, u32)> {
    let look = ViewState {
        face: Some("vadrel".into()),
        voice: Some(voice.into()),
        ..ViewState::default()
    };
    let fh = qp_grammar::faces::height_for(&qp_grammar::look::R4, Some("vadrel"), true);
    for it in items {
        let s = qp_session::viewer::face_scene(h.session_mut(), it, &look, 60, 390, fh)?;
        if !s.terms.is_empty() {
            return Some((it.clone(), s, fh));
        }
    }
    None
}

#[test]
fn glossary_terms_are_marked_and_a_tap_opens_the_explanation() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    for _ in 0..60 {
        assert!(h.take_step().is_some());
    }
    style(&mut h, "a", json!({"face":"vadrel","voice":"marla"}));
    let items = capsules(&mut h, "a");
    let (it, scene, fh) = voiced_face(&mut h, &items, "marla").expect("a face with a term");
    let v = it.voice.as_ref().unwrap();
    // The face speaks the voice: its headline and its ledger sentence are drawn.
    let texts: Vec<String> = scene
        .prims()
        .filter_map(|p| match p {
            qp_schema::scene::Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    let drawn = texts.join(" ");
    let first = v
        .headline
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(drawn.contains(&first), "{drawn} / {}", v.headline);
    let lw = v.ledger.split_whitespace().next().unwrap();
    assert!(drawn.contains(lw), "{drawn} / {}", v.ledger);
    // Every marked term is in the voice's glossary, inside the frame, underlined.
    for t in &scene.terms {
        assert!(v
            .glossary
            .iter()
            .any(|g| g.term == t.term && g.text == t.text));
        assert!(t.hit.left >= 0 && t.hit.right <= 390 && t.hit.top >= 0);
        assert!(t.hit.bottom <= i32::try_from(fh).unwrap() && t.hit.left < t.hit.right);
    }
    // A tap on the term: the authority answers with it and opens nothing.
    let t = &scene.terms[0];
    let (x, y) = (
        f64::from(t.hit.left + t.hit.right) / 2.0,
        f64::from(t.hit.top + t.hit.bottom) / 2.0,
    );
    let out = send(
        &mut h,
        "a",
        &json!({"type":"input","id":"t","gesture":{"kind":"tap_at","target":{"capsule":it.id},
                "x":x,"y":y,"width":390,"height":fh}}),
    );
    let m = result(&out, "a", "t");
    let i = &m["data"]["interaction"];
    assert_eq!(i["hit"]["term"]["term"], json!(t.term), "{m}");
    assert!(i.get("opening").is_none(), "{m}");
    // Deterministic: the same tap, the same answer.
    let again = send(
        &mut h,
        "a",
        &json!({"type":"input","id":"t","gesture":{"kind":"tap_at","target":{"capsule":it.id},
                "x":x,"y":y,"width":390,"height":fh}}),
    );
    assert_eq!(result(&again, "a", "t"), m);
    // A tap on the glyph (top left): the capsule opens, and the open card speaks the
    // voice's detail.
    let out = send(
        &mut h,
        "a",
        &json!({"type":"input","id":"o","gesture":{"kind":"tap_at","target":{"capsule":it.id},
                "x":12.0,"y":44.0,"width":390,"height":fh}}),
    );
    let m = result(&out, "a", "o");
    let i = &m["data"]["interaction"];
    assert!(i["hit"].get("term").is_none(), "{m}");
    assert_eq!(i["opening"]["to"], "capsule", "{m}");
}

#[test]
fn under_r4_scenes_carry_no_terms() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    for _ in 0..60 {
        assert!(h.take_step().is_some());
    }
    let items = capsules(&mut h, "a");
    let look = ViewState {
        face: Some("vadrel".into()),
        ..ViewState::default()
    };
    let fh = qp_grammar::faces::height_for(&qp_grammar::look::R4, Some("vadrel"), false);
    assert_eq!(fh, qp_grammar::faces::height(Some("vadrel")));
    let mut seen = 0;
    for it in &items {
        if let Some(s) = qp_session::viewer::face_scene(h.session_mut(), it, &look, 60, 390, fh) {
            seen += 1;
            assert!(s.terms.is_empty());
            let j = serde_json::to_string(&s).unwrap();
            assert!(!j.contains("\"terms\""), "{j}");
        }
    }
    assert!(seen > 0);
}
