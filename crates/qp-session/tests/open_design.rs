//! R5/M20: the open capsule design a connection chooses (`style {open}`).
//!
//! - `r4` (the default, today's card) and `tesir`; an unknown name is `invalid` with the
//!   known ones; the view state remembers it; the default is not written.
//! - Under `tesir` the capsule depth is the design's column at its own frame height; the
//!   authority's scene and the shell's (`viewer::open_scene_with`, same reader, same
//!   language) are the same scene, so a tap is read where it was drawn.
//! - A tap on the trust block answers with its explanation (`hit.term`) and opens
//!   nothing; a tap on the claim opens the chart.
//! - The device's check changes the trust row's words only (same layout, same terms).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::{
    CapsuleItem, Language, Opening, OutboundLine, ReplayInit, ReplaySourceKind, ViewState,
};
use qp_session::viewer::{open_scene_with, OpenExtra, TrustState};
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn host() -> ReplayHost {
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    let init = demo_init(ReplayInit {
        source: ReplaySourceKind::Synthetic,
        instruments: Some(8),
        days: Some(260),
        history: Some(120),
        fine: Some("1h".to_owned()),
        entitlements: ent,
        ..ReplayInit::default()
    });
    let none = std::env::temp_dir().join("qp-open-no-local-data");
    ReplayHost::with_local_dir(init, &root(), &none).expect("demo host")
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

fn input(h: &mut ReplayHost, g: Value) -> Value {
    let out = send(h, "a", &json!({"type":"input","id":"i","gesture":g}));
    result(&out, "a", "i")
}

fn capsules(h: &mut ReplayHost) -> Vec<CapsuleItem> {
    let out = send(
        h,
        "a",
        &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
    );
    let m = result(&out, "a", "q");
    serde_json::from_value(m["data"]["capsules"].clone()).unwrap()
}

#[test]
fn style_open_is_validated_and_remembered() {
    let mut h = host();
    send(
        &mut h,
        "a",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone","lang":"tr"}}),
    );
    let m = input(&mut h, json!({"kind":"style","open":"nope"}));
    assert_eq!(m["type"], "error", "{m}");
    assert!(m.to_string().contains("tesir"), "{m}");
    let m = input(&mut h, json!({"kind":"style","open":"tesir"}));
    assert_eq!(m["data"]["interaction"]["state"]["open"], "tesir", "{m}");
    let m = input(&mut h, json!({"kind":"style","open":"r4"}));
    assert!(
        m["data"]["interaction"]["state"].get("open").is_none(),
        "{m}"
    );
}

#[test]
fn the_column_is_one_scene_on_both_sides_and_its_trust_block_explains() {
    let mut h = host();
    send(
        &mut h,
        "a",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone","lang":"tr"}}),
    );
    for _ in 0..60 {
        assert!(h.take_step().is_some());
    }
    input(
        &mut h,
        json!({"kind":"style","open":"tesir","voice":"marla"}),
    );
    let items = capsules(&mut h);
    let it = items
        .iter()
        .find(|i| i.voice.is_some() && !i.redacted)
        .expect("a voiced capsule")
        .clone();
    let fh = qp_grammar::open::height_in(&qp_grammar::look::R4, Some("tesir")).unwrap();
    let m = input(
        &mut h,
        json!({"kind":"open","capsule":it.id,"to":"capsule","width":390,"height":fh}),
    );
    let o: Opening = serde_json::from_value(m["data"]["interaction"]["opening"].clone())
        .unwrap_or_else(|e| panic!("{e}: {m}"));
    assert_eq!(o.frame, Some([390, fh]));
    let look: ViewState =
        serde_json::from_value(m["data"]["interaction"]["state"].clone()).unwrap();
    let extra = OpenExtra {
        lang: Some(Language::Tr),
        check: None,
    };
    let shell = open_scene_with(
        h.session_mut(),
        &o,
        Some(&it.sentence.text),
        it.voice.as_ref(),
        &look,
        120,
        extra,
    )
    .expect("the shell's scene");
    let names: Vec<_> = shell.panels.iter().filter_map(|p| p.name.clone()).collect();
    assert_eq!(
        names,
        ["claim", "basis", "evidence", "why", "trust", "record"]
    );
    let trust = shell
        .terms
        .iter()
        .find(|t| t.layer == "open-trust")
        .expect("the trust term")
        .clone();
    // A tap in the middle of the trust block: the authority answers with the same term.
    let (x, y) = (
        f64::from(trust.hit.left + trust.hit.right) / 2.0,
        f64::from(trust.hit.top + trust.hit.bottom) / 2.0,
    );
    let m = input(
        &mut h,
        json!({"kind":"tap_at","target":{"capsule":it.id},"x":x,"y":y,"width":390,"height":fh}),
    );
    let i = &m["data"]["interaction"];
    assert_eq!(i["hit"]["term"]["term"], json!(trust.term), "{m}");
    assert_eq!(
        i["hit"]["term"]["hit"],
        serde_json::to_value(trust.hit).unwrap()
    );
    assert!(i.get("opening").is_none(), "{m}");
    // The device's check: other words, the same layout and terms.
    let checked = open_scene_with(
        h.session_mut(),
        &o,
        Some(&it.sentence.text),
        it.voice.as_ref(),
        &look,
        120,
        OpenExtra {
            check: Some(TrustState::Device),
            ..extra
        },
    )
    .unwrap();
    assert_eq!(checked.terms, shell.terms);
    let rects =
        |s: &qp_schema::scene::SceneDesc| s.panels.iter().map(|p| p.rect).collect::<Vec<_>>();
    assert_eq!(rects(&checked), rects(&shell));
    assert_ne!(
        qp_session::viewer::scene_digest(&checked),
        qp_session::viewer::scene_digest(&shell)
    );
    // A tap on the claim (top left): the chart opens.
    let m = input(
        &mut h,
        json!({"kind":"tap_at","target":{"capsule":it.id},"x":12.0,"y":44.0,"width":390,"height":fh}),
    );
    assert_eq!(m["data"]["interaction"]["opening"]["to"], "chart", "{m}");
}
