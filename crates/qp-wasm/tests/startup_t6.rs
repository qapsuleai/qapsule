//! R4/T6: the faster opening paths give the same results as the general ones.
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
use serde_json::json;

/// Opens a page on `host` the way `qp demo` does; `frame` writes each message frame.
fn open(host: &mut ReplayHost, frame: impl Fn(&serde_json::Value) -> String) -> serde_json::Value {
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    for _ in 0..100 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw("w", &m.to_string()) {
                if l.conn == "w" {
                    let msg = serde_json::to_value(&l.msg).expect("msg");
                    core.on_frame(&frame(&msg)).expect("frame");
                }
            }
        }
    }
    assert!(!core.feed().is_empty(), "the page has a feed");
    core.digests()
}

/// The direct parse of a host frame (`{"msg":…}`) and the general path (a frame the
/// direct parse does not take) give the same document and scenes.
#[test]
fn direct_message_frames_equal_the_general_path() {
    let fast = open(&mut parity_host(), |m| json!({"msg": m}).to_string());
    let general = open(&mut parity_host(), |m| format!("{{ \"msg\" : {m} }}"));
    assert_eq!(fast, general);
}
