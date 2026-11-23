//! R5/M21 `surek`: the faces' own changes in the shell core — the same facts with motion
//! and under reduced motion (information loss 0), the label on the face under reduced
//! motion, a frame for every animated change, and today's motion (`r4`) unchanged.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use qp_schema::scene::{Prim, SceneDesc};
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

fn pump(core: &mut ShellCore, w: &mut ReplayHost) {
    for _ in 0..1000 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in w.handle_raw("s", &m.to_string()) {
                if l.conn == "s" {
                    core.on_message(&l.msg);
                }
            }
        }
    }
}

/// One replay step: the host's new events as data frames, then the feed's answer.
fn step(core: &mut ShellCore, w: &mut ReplayHost) {
    let before = w.recorded_len();
    core.replay_control(json!({"kind":"step","n":1}));
    pump(core, w);
    let evs: Vec<_> = w.recorded_since(before);
    for ev in &evs {
        core.data_event(ev);
    }
    pump(core, w);
}

fn page(motion: &str, reduced: bool) -> (ShellCore, ReplayHost) {
    let mut w = parity_host();
    w.set_record_events(true);
    let data = w.init().data.clone().expect("replay init has data");
    let evs = w.recorded_events();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    core.set_look(&json!({"motion": motion}));
    core.set_reduced_motion(reduced);
    core.set_audience(&json!({"group":"habituated","sensitivity":"all","watch":["SYN06"]}));
    pump(&mut core, &mut w);
    (core, w)
}

fn texts(sc: &SceneDesc) -> Vec<String> {
    let mut out = Vec::new();
    let mut take = |p: &Prim| {
        if let Prim::Text { text, .. } = p {
            out.push(text.clone());
        }
    };
    sc.under.iter().for_each(&mut take);
    for p in &sc.panels {
        for l in &p.layers {
            l.prims.iter().for_each(&mut take);
        }
    }
    sc.over.iter().for_each(&mut take);
    out
}

const STEPS: usize = 60;

#[test]
fn reduced_motion_carries_the_same_facts_as_motion() {
    let (mut anim, mut wa) = page("surek", false);
    let (mut red, mut wr) = page("surek", true);
    assert_eq!(anim.look_json()["motion"], "surek");
    assert_eq!(anim.look_json()["transition"], "surek");
    assert_eq!(red.look_json()["reduced_motion"], true);
    let mut kinds = std::collections::BTreeSet::new();
    let mut animated = 0usize;
    let mut labelled = 0usize;
    for _ in 0..STEPS {
        step(&mut anim, &mut wa);
        step(&mut red, &mut wr);
        // The same facts (which capsule changed, how) in both modes.
        let (fa, fr) = (anim.face_changes_json(), red.face_changes_json());
        assert_eq!(fa, fr);
        // Reduced motion: nothing moves; every change is said on its face.
        assert_eq!(red.take_face_motion(), json!([]));
        for c in fr.as_array().expect("changes") {
            let id = c["id"].as_str().expect("id");
            kinds.insert(c["kind"].as_str().expect("kind").to_owned());
            if let Some(sc) = red.face(id) {
                let t = texts(&sc);
                assert!(
                    t.iter().any(|x| x == c["label"].as_str().unwrap()),
                    "{id}: {t:?}"
                );
                labelled += 1;
            }
            // ... and never with motion (the label is the still stand-in).
            if let Some(sc) = anim.face(id) {
                assert!(!texts(&sc).iter().any(|x| x == c["label"].as_str().unwrap()));
            }
        }
        // Motion: every queued change has frames that move.
        for id in anim.take_face_motion().as_array().expect("ids") {
            let id = id.as_str().expect("id");
            let f0 = anim.face_change_frame(id, 0.0, 0.0, 0.0);
            let f5 = anim.face_change_frame(id, 0.5, 0.0, 0.0);
            assert!(!f5["elements"].as_array().expect("els").is_empty(), "{id}");
            assert_ne!(f0, f5, "{id}");
            assert_eq!(anim.face_change_frame(id, 1.0, 0.0, 0.0)["done"], true);
            animated += 1;
        }
        anim.mark_seen();
        red.mark_seen();
        pump(&mut anim, &mut wa);
        pump(&mut red, &mut wr);
        assert_eq!(red.face_changes_json(), json!([]));
    }
    assert!(
        animated > 0 && labelled > 0,
        "animated {animated}, labelled {labelled}"
    );
    // A state change, not only births.
    assert!(kinds.len() >= 3, "{kinds:?}");
}

#[test]
fn reduced_motion_opens_at_once() {
    let (mut red, _) = page("surek", true);
    assert!((red.transition_progress(0.0) - 1.0).abs() < 1e-12);
    let (mut anim, _) = page("surek", false);
    assert!(anim.transition_progress(0.0) < 1.0);
}

#[test]
fn todays_motion_marks_nothing() {
    let (mut core, mut w) = page("r4", false);
    let (mut red, mut wr) = page("r4", true);
    assert!(core.look_json().get("motion").is_none());
    assert_eq!(
        core.look_json()["transition"],
        qp_designs::transitions::PRESENTED_TRANSITION
    );
    for _ in 0..STEPS {
        step(&mut core, &mut w);
        step(&mut red, &mut wr);
        assert_eq!(core.face_changes_json(), json!([]));
        assert_eq!(core.take_face_motion(), json!([]));
        assert_eq!(red.face_changes_json(), Value::Array(Vec::new()));
    }
}
