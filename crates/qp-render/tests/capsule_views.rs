//! R3 capsule views (criteria 7–8, `docs/GRAMMAR.md` §18): the sample documents and
//! transcripts are exactly what the generator (`support/capsule_gen.rs`) produces; each
//! depth view is built only from its transcript, rendered twice byte-identically, equal
//! to its golden (`fixtures/golden/views/<stem>.png`), and its explain text equals
//! `fixtures/views/<stem>.explain.txt`. The scene is made the way `qp render --transcript
//! <stem>.json --present <finding> --surface <s> --width <w> --height <h>` makes it (the
//! verify `views` loop runs that CLI on the same files). `c1-sentence` (sentence depth) is
//! checked by `qp conform fixtures/views` (the presenter's text) and here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

#[path = "support/capsule_gen.rs"]
mod capsule_gen;
#[path = "support/golden_check.rs"]
mod golden_check;

use golden_check::root;
use qp_session::conform::{load_init, read_transcript};
use qp_session::Session;
use serde_json::{json, Value};

#[test]
fn fixtures_are_generated() {
    let root = root();
    for (rel, content) in capsule_gen::files() {
        let have = std::fs::read_to_string(root.join(&rel)).unwrap_or_default();
        assert!(
            have == content,
            "{rel} differs from the generator; run `cargo run -p qp-render --example make_capsule_views -- --force`"
        );
    }
}

/// The session after the transcript's messages (as `qp render --transcript`).
fn session(stem: &str) -> (Session, String) {
    let root = root();
    let t = read_transcript(&root.join(format!("fixtures/views/{stem}.json"))).unwrap();
    let init = load_init(&t.init, &root).unwrap().init;
    let principal = init.principals[0].id.clone();
    let mut s = Session::new(init, &root).unwrap();
    for step in &t.steps {
        if let Some(line) = &step.send {
            s.handle_raw(&line.conn, &serde_json::to_string(&line.msg).unwrap());
        }
    }
    (s, principal)
}

fn ask(s: &mut Session, conn: &str, msg: &Value) -> Value {
    let out = s.handle_raw(conn, &msg.to_string());
    let o = out.into_iter().find(|o| o.conn == conn).expect("an answer");
    serde_json::to_value(&o.msg).unwrap()
}

/// The presentation of `finding` on a new `surface` connection.
fn present(s: &mut Session, principal: &str, finding: &str, surface: &str) -> Value {
    let conn = "qp-cli-present";
    let w = ask(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":surface}}),
    );
    assert_eq!(w["type"], "welcome", "{w}");
    let r = ask(
        s,
        conn,
        &json!({"type":"query","id":"present","query":{"kind":"present","finding":finding}}),
    );
    r["data"]["presentation"].clone()
}

#[test]
fn depth_views_match_goldens_and_explain() {
    let root = root();
    for (stem, sample, surface, w, h) in capsule_gen::DEPTHS {
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(sample.doc())).unwrap())
                .unwrap();
        let finding = capsule_gen::finding_of(&doc);
        let (mut s, principal) = session(stem);
        let p = present(&mut s, &principal, &finding, surface);
        let v = if p["capsule_view"].is_object() {
            &p["capsule_view"]
        } else {
            &p["view"]
        };
        let view: qp_schema::grammar::View = serde_json::from_value(v.clone()).unwrap();
        let opts = qp_designs::options(*w, *h);
        let scene = qp_grammar::compile(&view, &s.provider(), &opts).unwrap();
        let a = qp_render::render_scene_png(&scene).unwrap();
        let b =
            qp_render::render_scene_png(&qp_grammar::compile(&view, &s.provider(), &opts).unwrap())
                .unwrap();
        assert_eq!(a, b, "{stem}: two renders differ");
        golden_check::check_file(&format!("fixtures/golden/views/{stem}.png"), &a);
        // The point query on the glyph returns the capsule's finding.
        let glyph = scene
            .panels
            .iter()
            .flat_map(|p| &p.layers)
            .find(|l| l.mark == qp_schema::grammar::Mark::Glyph)
            .expect("a glyph layer");
        let hb = glyph.items[0].hit;
        let hit = qp_session::hit_scene(
            &scene,
            f64::from(hb.left + hb.right) / 2.0,
            f64::from(hb.top + hb.bottom) / 2.0,
            &[],
        );
        let item = hit.item.expect("glyph item hit");
        assert_eq!(item.mark, qp_schema::grammar::Mark::Glyph, "{stem}");
        assert_eq!(
            item.data,
            qp_schema::scene::DataSource::Finding {
                finding: finding.clone()
            },
            "{stem}"
        );
        let explain = qp_session::explain_scene(&view, &scene);
        golden_check::check_file(
            &format!("fixtures/views/{stem}.explain.txt"),
            explain.as_bytes(),
        );
    }
}

#[test]
fn sentence_depth_is_the_capsule_sentence() {
    let root = root();
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(capsule_gen::Sample::Breakout.doc())).unwrap(),
    )
    .unwrap();
    let finding = capsule_gen::finding_of(&doc);
    let (mut s, principal) = session("c1-sentence");
    let p = present(&mut s, &principal, &finding, "watch");
    assert_eq!(p["depth"], "sentence");
    assert_eq!(p["text"], doc["capsules"][0]["headline"]);
    assert!(p["view"].is_null() && p["capsule_view"].is_null());
}
