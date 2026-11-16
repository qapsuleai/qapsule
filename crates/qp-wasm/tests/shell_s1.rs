//! R4/S1: the presentation's defaults (M9–M11 picks) and the human-test URL settings
//! (`?seen=`, `?watch=`/`?long=`), all in the shell core.
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

/// A page on `host`; `setup` runs right after the core is made (as `main.js`'s `start`).
fn page(host: &mut ReplayHost, setup: impl FnOnce(&mut ShellCore)) -> ShellCore {
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    setup(&mut core);
    pump(&mut core, host);
    core
}

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

fn sent_kind<'a>(core: &'a ShellCore, op: &str) -> Vec<&'a Value> {
    core.sent()
        .iter()
        .filter(|m| m["ops"][0]["op"] == op)
        .collect()
}

#[test]
fn the_presentation_asks_for_the_m9_m11_picks_and_the_page_overrides_them() {
    let core = page(&mut parity_host(), |_| {});
    let look = serde_json::to_value(core.look()).unwrap();
    assert_eq!(look["feed"], qp_grammar::feeds::PRESENTED_FEED, "{look}");
    assert_eq!(
        look["transition"],
        qp_grammar::transitions::PRESENTED_TRANSITION
    );
    assert_eq!(
        look["outcome"],
        qp_grammar::glyph::outcome::PRESENTED_OUTCOME
    );
    assert!(
        look.get("face").is_none(),
        "the face default is M8's: {look}"
    );
    // The page's own choice (URL) is sent after the defaults and wins; `r3` is the
    // protocol default, so the fields are absent again.
    let core = page(&mut parity_host(), |c| {
        c.set_look(&json!({"feed":"r3","transition":"r3","outcome":"r3"}));
    });
    let look = serde_json::to_value(core.look()).unwrap();
    for k in ["feed", "transition", "outcome"] {
        assert!(look.get(k).is_none(), "{k}: {look}");
    }
}

#[test]
fn url_audience_takes_comma_lists() {
    let core = page(&mut parity_host(), |c| {
        c.set_audience(&json!({"group":"","watch":"SYN01, SYN02","long":"SYN09","short":""}));
    });
    let a = sent_kind(&core, "set_audience");
    assert_eq!(a.len(), 1, "{:?}", core.sent());
    let aud = &a[0]["ops"][0]["audience"];
    assert_eq!(aud["group"], "habituated");
    assert_eq!(aud["watchlist"], json!(["SYN01", "SYN02"]));
    assert_eq!(aud["positions"], json!({"SYN09": 1.0}));
    assert_eq!(core.audience()["watchlist"], json!(["SYN01", "SYN02"]));
}

#[test]
fn url_seen_waits_for_its_step() {
    let mut host = parity_host();
    let mut core = page(&mut host, |c| c.seen_at("3"));
    // The replay stands at step 0 (paused): nothing marked yet.
    let look = serde_json::to_value(core.look()).unwrap();
    assert!(look.get("seen").is_none(), "{look}");
    core.seek("5");
    pump(&mut core, &mut host);
    // The host's data frame carries the new replay state (as `qp demo` sends it).
    let st = host.state();
    core.on_frame(&json!({ "replay": st }).to_string())
        .expect("frame");
    pump(&mut core, &mut host);
    let look = serde_json::to_value(core.look()).unwrap();
    assert_eq!(look["seen"], json!(st.t), "{look}");
    // `now` marks at the first known clock; garbage is ignored.
    let core = page(&mut parity_host(), |c| {
        c.seen_at("yesterday");
        c.seen_at("now");
    });
    let look = serde_json::to_value(core.look()).unwrap();
    assert!(look["seen"].is_i64(), "{look}");
}
