//! RP3/T4: the shell core's own records stay bounded on a long session
//! (`rounds/RP3/experiments/T4.md`).
#![cfg(feature = "replica")]

use qp_wasm::drive::parity_host;
use qp_wasm::shell::{ShellCore, SENT_KEEP};
use serde_json::json;

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
