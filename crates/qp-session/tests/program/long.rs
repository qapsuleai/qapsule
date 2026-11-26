//! A long log and a command of many program ops: the log's program bytes are counted
//! once, not once per op (the memo in `qp-program`'s budget).

use std::time::{Duration, Instant};

use serde_json::json;

use crate::rig::{H, SMA_TEXT};

#[test]
fn many_program_ops_over_a_long_log_stay_fast() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    for k in 0..600 {
        let r = h.cmd(
            "a",
            json!([{"op":"set_program_params","program":"p1","params":{"len": 2 + k % 40}}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
    }
    let ir = h.ir(SMA_TEXT);
    let ops: Vec<_> = (0..30)
        .map(|k| json!({"op":"add_program","id":format!("m{k}"),"program":ir,"inputs":["px"]}))
        .collect();
    let t = Instant::now();
    let r = h.cmd("a", json!(ops));
    let took = t.elapsed();
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(
        h.s.document()
            .series
            .iter()
            .filter(|s| s.id.starts_with('m') && s.id.contains('.'))
            .count(),
        60
    );
    eprintln!("30 program ops over a 600-entry log: {took:?}");
    // Measured in a debug build: ~0.17 s with the memo, ~1.7 s rescanning per op (and the
    // 600 set-ups ~25 s instead of ~1 s). The bound sits between, with room.
    assert!(took < Duration::from_secs(1), "{took:?}");
}
