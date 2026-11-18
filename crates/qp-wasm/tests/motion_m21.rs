//! R5/M21 motion design `nabiz` in the shell core: the still marks are the same with and
//! without reduce motion (no fact is lost), the default design changes nothing, a state
//! change the device sees gives one beat (and a change label until the next look), the
//! overlay is empty under reduce motion and every frame is a pure function of its time.
#![cfg(feature = "replica")]

use std::collections::BTreeSet;

use qp_schema::scene::{Prim, SceneDesc};
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

fn deliver(core: &mut ShellCore, lines: Vec<qp_schema::protocol::OutboundLine>) {
    for l in lines {
        if l.conn == "w" {
            let msg = serde_json::to_value(&l.msg).expect("msg");
            core.on_frame(&json!({ "msg": msg }).to_string())
                .expect("frame");
        }
    }
}

fn pump(core: &mut ShellCore, host: &mut ReplayHost) {
    for _ in 0..200 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            let lines = host.handle_raw("w", &m.to_string());
            deliver(core, lines);
        }
    }
}

/// One replay step: its data events as one data frame, then what the authority sent.
fn step(core: &mut ShellCore, host: &mut ReplayHost) {
    let from = host.recorded_events().len();
    let Some((_, lines)) = host.take_step() else {
        return;
    };
    let evs: Vec<Value> = host.recorded_events()[from..]
        .iter()
        .map(|e| serde_json::to_value(e).expect("event"))
        .collect();
    core.on_frame(&json!({"data": evs, "replay": host.state()}).to_string())
        .expect("data");
    deliver(core, lines);
    pump(core, host);
}

fn page(host: &mut ReplayHost, choice: &Value) -> ShellCore {
    host.set_record_events(true);
    for _ in 0..120 {
        let _ = host.take_step();
    }
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    core.set_look(choice);
    pump(&mut core, host);
    core
}

fn ids(core: &ShellCore) -> Vec<String> {
    core.feed().iter().map(|i| i.id.clone()).collect()
}

fn texts(s: &SceneDesc) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut add = |p: &Prim| {
        if let Prim::Text { text, .. } = p {
            out.insert(text.clone());
        }
    };
    s.under.iter().for_each(&mut add);
    s.over.iter().for_each(&mut add);
    for p in &s.panels {
        for l in &p.layers {
            l.prims.iter().for_each(&mut add);
        }
    }
    out
}

#[test]
fn the_choices_list_the_motion_designs_and_the_default_changes_nothing() {
    let ch = ShellCore::choices_json();
    assert_eq!(ch["motion"], json!(["r4", "nabiz"]), "{ch}");
    let mut a = page(&mut parity_host(), &json!({"feed_order": "live_new"}));
    let mut b = page(
        &mut parity_host(),
        &json!({"feed_order": "live_new", "motion": "r4"}),
    );
    assert!(b.look_json().get("motion").is_none());
    let list = ids(&a);
    assert!(!list.is_empty());
    for id in list.iter().take(6) {
        assert_eq!(a.face(id), b.face(id), "{id}: r4 decorates nothing");
    }
}

#[test]
fn nabiz_selects_its_transition_and_adds_only_still_marks() {
    let mut plain = page(&mut parity_host(), &json!({"feed_order": "live_new"}));
    let mut core = page(
        &mut parity_host(),
        &json!({"feed_order": "live_new", "motion": "nabiz"}),
    );
    assert_eq!(core.look_json()["motion"], "nabiz");
    assert_eq!(core.look().transition.as_deref(), Some("nabiz"));
    let mut live_dots = 0;
    for id in ids(&core).iter().take(10) {
        let (a, b) = (plain.face(id).expect("face"), core.face(id).expect("face"));
        // The same scene with primitives added after everything else; no hit box moves.
        assert_eq!(a.panels, b.panels, "{id}");
        assert_eq!(a.terms, b.terms, "{id}");
        assert_eq!(&b.over[..a.over.len()], &a.over[..], "{id}");
        let extra = &b.over[a.over.len()..];
        live_dots += extra
            .iter()
            .filter(|p| matches!(p, Prim::Circle { .. }))
            .count();
        let it = core
            .feed()
            .iter()
            .find(|i| &i.id == id)
            .expect("item")
            .clone();
        let live = qp_grammar::transitions::nabiz::is_live(it.stage, it.validity);
        assert_eq!(
            extra.iter().any(|p| matches!(p, Prim::Circle { .. })),
            live,
            "{id}: the live dot iff live"
        );
    }
    assert!(live_dots > 0, "some capsule in the feed is live");
}

#[test]
fn reduce_motion_loses_no_fact_and_paints_no_overlay() {
    let mut host = parity_host();
    let mut core = page(
        &mut host,
        &json!({"feed_order": "live_new", "motion": "nabiz"}),
    );
    for id in ids(&core) {
        core.visible(&id, true);
    }
    let _ = core.take_outbox();
    // Walk until the device sees a change (a beat).
    let mut beat = None;
    for _ in 0..200 {
        step(&mut core, &mut host);
        for id in ids(&core) {
            core.visible(&id, true);
        }
        let _ = core.take_outbox();
        // Faces are built (their mark places kept) as the page would paint them.
        for id in ids(&core) {
            let _ = core.face(&id);
        }
        let m = core.motion_marks(1_000.0).unwrap_or(Value::Null);
        if let Some(b) = m["marks"].as_array().and_then(|a| {
            a.iter()
                .find(|x| x["key"].as_str().is_some_and(|k| k.ends_with(":beat")))
        }) {
            beat = Some(b["face"].as_str().expect("face").to_owned());
            break;
        }
    }
    let id = beat.expect("a state change within 200 steps");
    // The change label is in the face (the still mark of the change).
    let with = core.face(&id).expect("face");
    let it = core
        .feed()
        .iter()
        .find(|i| i.id == id)
        .expect("item")
        .clone();
    let word = qp_grammar::transitions::nabiz::label(
        it.stage,
        it.validity,
        qp_schema::protocol::Language::Tr,
    )
    .expect("label");
    assert!(texts(&with).contains(word), "{id}: label `{word}`");
    // Frames are pure functions of time.
    let a = core.motion_marks(1_300.0);
    core.set_reduced_motion(false);
    assert_eq!(a, core.motion_marks(1_300.0), "deterministic");
    // Reduce motion: the overlay goes (once, empty), the faces stay byte-identical.
    core.set_reduced_motion(true);
    assert_eq!(core.motion_marks(1_400.0), Some(json!({"marks": []})));
    assert_eq!(core.motion_marks(1_500.0), None);
    for i in ids(&core) {
        core.set_reduced_motion(false);
        let a = core.face(&i);
        core.set_reduced_motion(true);
        let b = core.face(&i);
        assert_eq!(a, b, "{i}: the same facts with and without motion");
    }
    assert!((core.transition_progress(0.0) - 1.0).abs() < 1e-12);
    // The reader's next look takes the label away.
    core.set_reduced_motion(false);
    core.mark_seen();
    pump(&mut core, &mut host);
    let after = core.face(&id).expect("face");
    assert!(!texts(&after).contains(word), "{id}: seen");
}
