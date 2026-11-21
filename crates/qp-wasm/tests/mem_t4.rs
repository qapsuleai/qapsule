//! RP3/T4: the shell core's own records stay bounded on a long session, and the large
//! frames (start, data) parsed typed give the same page as the JSON-tree path
//! (`rounds/RP3/experiments/T4.md`).
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::{core_from_start, ShellCore, SENT_KEEP};
use serde_json::{json, Value};

/// Passes the core's messages to `host` and the answers back until it is idle.
fn pump(core: &mut ShellCore, host: &mut ReplayHost) {
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
}

/// The demo host after `n` replay steps, recording its events.
fn host_at(n: usize) -> ReplayHost {
    let mut h = parity_host();
    h.set_record_events(true);
    for _ in 0..n {
        let _ = h.take_step();
    }
    h
}

/// The next `n` steps of `h` as one data frame (as `qp demo` broadcasts them), with
/// `extra` events appended.
fn data_frame(h: &mut ReplayHost, n: usize, extra: &[Value]) -> String {
    let from = h.recorded_len();
    for _ in 0..n {
        let _ = h.take_step();
    }
    let mut evs: Vec<Value> = h.recorded_events()[from..]
        .iter()
        .map(|e| serde_json::to_value(e).expect("event"))
        .collect();
    evs.extend(extra.iter().cloned());
    json!({"data": evs, "replay": h.state()}).to_string()
}

#[test]
fn typed_frames_give_the_same_page_as_the_json_tree_path() {
    // A: typed start frame. B: a start frame the typed parse refuses (`lang` not text:
    // the tree path reads it as absent, `tr`), so it takes the tree path.
    let (mut ha, mut hb) = (host_at(300), host_at(300));
    let start = |h: &ReplayHost, lang: Value| {
        let mut data = h.init().data.clone().expect("data");
        data.entitlements.retain(|p, _| p == "arkin");
        json!({"conn":"c1","token":"t","principal":"arkin","lang":lang,
               "data":data,"events":h.recorded_events(),"info":{}})
        .to_string()
    };
    let mut a = core_from_start(&start(&ha, json!("tr"))).expect("a");
    let mut b = core_from_start(&start(&hb, json!(7))).expect("b");
    pump(&mut a, &mut ha);
    pump(&mut b, &mut hb);
    assert_eq!(a.lang(), "tr");
    assert_eq!(b.lang(), "tr");
    assert_eq!(a.digests(), b.digests());
    // A data frame: typed (A); with a malformed event appended (B), the tree path, which
    // skips that event.
    let fa = data_frame(&mut ha, 120, &[]);
    let fb = data_frame(&mut hb, 120, &[json!({"bogus": 1})]);
    let ca = a.on_frame(&fa).expect("a frame");
    let cb = b.on_frame(&fb).expect("b frame");
    assert_eq!(ca, cb);
    assert_eq!(ca, json!({"scene": true, "feed": true, "status": true}));
    pump(&mut a, &mut ha);
    pump(&mut b, &mut hb);
    assert_eq!(a.version(), b.version());
    assert_eq!(a.status_json(), b.status_json());
    assert_eq!(a.digests(), b.digests());
    // A frame without data or replay changes nothing, either way.
    assert_eq!(a.on_frame("{}").expect("empty"), json!({}));
    assert_eq!(
        a.on_frame(r#"{"data":null,"replay":null}"#).expect("nulls"),
        json!({})
    );
}

#[test]
fn the_sent_record_keeps_a_short_session_whole_and_a_long_one_bounded() {
    let host = parity_host();
    let data = host.init().data.clone().expect("data");
    let mut core = ShellCore::new("arkin", "tr", &data, &[]).expect("core");
    let first = core.take_outbox();
    assert!(!first.is_empty());
    // A short session: everything is kept, in order.
    for _ in 0..SENT_KEEP {
        core.replay_control(json!({"kind":"status"}));
        let _ = core.take_outbox();
    }
    assert_eq!(core.sent().len(), first.len() + SENT_KEEP);
    assert_eq!(core.sent()[0], first[0]);
    // A long one: at most twice the kept number, the latest message last.
    for i in 0..5 * SENT_KEEP {
        core.replay_control(json!({"kind":"status"}));
        let out = core.take_outbox();
        let n = core.sent().len();
        assert!((SENT_KEEP..=2 * SENT_KEEP).contains(&n), "{i}: {n}");
        assert_eq!(core.sent().last(), out.last());
    }
}
