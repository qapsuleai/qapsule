//! R5/S: the integration's presentation defaults (M16 look + companion face, M17 voice,
//! M18 story chart, M20 open capsule design), the look → face rule, and the viewer's feed
//! text following a voice change (the gallery agent's question).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::{parity_host, pump_viewer};
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

/// A replica page on `host`; `setup` runs right after the core is made (as `start`).
fn page(host: &mut ReplayHost, setup: impl FnOnce(&mut ShellCore)) -> ShellCore {
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    setup(&mut core);
    for _ in 0..200 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw("w", &m.to_string()) {
                if l.conn == "w" {
                    let msg = serde_json::to_value(&l.msg).expect("msg");
                    core.on_frame(&json!({ "msg": msg }).to_string())
                        .expect("frame");
                }
            }
        }
    }
    core
}

fn look(core: &ShellCore) -> Value {
    serde_json::to_value(core.look()).unwrap()
}

#[test]
fn the_presentation_asks_for_the_r5_picks() {
    let l = look(&page(&mut parity_host(), |_| {}));
    assert_eq!(l["look"], qp_grammar::look::PRESENTED_LOOK, "{l}");
    assert_eq!(l["look"], "oltan");
    assert_eq!(l["face"], "oltan", "{l}");
    assert_eq!(l["voice"], "marla", "{l}");
    assert_eq!(l["story"], qp_grammar::stories::PRESENTED_STORY, "{l}");
    // (R6/T0, K57) Arkin's pick.
    assert_eq!(l["story"], "korda");
    assert_eq!(l["open"], qp_grammar::open::PRESENTED_OPEN, "{l}");
    assert_eq!(l["open"], "tesir");
    // (R5/S) M21's provisional pick opens with its own transition.
    assert_eq!(l["transition"], "surek", "{l}");
}

#[test]
fn a_look_brings_its_companion_face_unless_a_face_is_named() {
    for (choice, face) in [
        (json!({"look": "zemre"}), Some("zemre")),
        (json!({"look": "zemre", "face": "vadrel"}), Some("vadrel")),
        (json!({"look": "zemre", "face": ""}), Some("zemre")),
        // `r4`'s companion is R4's presented face.
        (json!({"look": "r4"}), Some("vadrel")),
        // A look without a companion keeps the face (the presented one).
        (json!({"look": "ornek"}), Some("oltan")),
        // The protocol default face (`r3`) is absent from the view state.
        (json!({"look": "oltan", "face": "r3"}), None),
    ] {
        let l = look(&page(&mut parity_host(), |c| c.set_look(&choice)));
        assert_eq!(l["face"].as_str(), face, "{choice}: {l}");
    }
}

/// A viewer core on `host`, at step `step`, with the presentation's look and `choice`.
fn viewer(host: ReplayHost, step: u32) -> (ShellCore, ReplayHost, Vec<qp_schema::data::FeedKey>) {
    let mut core = ShellCore::new_viewer("arkin", "tr", &[]).expect("core");
    let mut host = host;
    let mut served = Vec::new();
    pump_viewer(&mut core, &mut host, &mut served);
    core.seek(&step.to_string());
    pump_viewer(&mut core, &mut host, &mut served);
    (core, host, served)
}

fn feed_texts(core: &ShellCore) -> Vec<String> {
    core.feed()
        .iter()
        .map(|i| serde_json::to_string(i).unwrap())
        .collect()
}

#[test]
fn a_viewer_feed_follows_a_voice_change() {
    let (mut core, mut host, mut served) = viewer(parity_host(), 120);
    let marla = feed_texts(&core);
    assert!(!marla.is_empty(), "feed");
    core.set_look(&json!({"voice": "selvi"}));
    pump_viewer(&mut core, &mut host, &mut served);
    let selvi = feed_texts(&core);
    assert_eq!(look(&core)["voice"], "selvi");
    assert_ne!(marla, selvi, "the viewer's feed did not follow the voice");
    core.set_look(&json!({"voice": "marla"}));
    pump_viewer(&mut core, &mut host, &mut served);
    assert_eq!(feed_texts(&core), marla, "back to marla");
}
