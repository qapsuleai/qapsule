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

/// The bytes of program series `id` as the log stores them.
fn bytes(h: &H, id: &str) -> u64 {
    serde_json::to_vec(&h.series(id)).unwrap().len() as u64
}

/// Repeated value changes fill the owner's log bytes (`program.live.doc.max_log_bytes`,
/// 4 MiB): refused at exactly the predicted change, counted by the slot over every
/// commit; an undo's restored series count as written.
#[test]
fn repeated_set_params_reach_the_log_byte_budget_exactly() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let set = |h: &mut H, len: u32| {
        h.cmd(
            "a",
            json!([{"op":"set_program_params","program":"p1","params":{"len": len}}]),
        )
    };
    // Same-width values: every entry writes the same bytes.
    assert_eq!(set(&mut h, 10)["status"], "applied");
    let per = bytes(&h, "p1.avg") + bytes(&h, "p1.up");
    let limit = 4_194_304_u64;
    // add_program wrote `per` bytes (as wide as these), and one set so far.
    let fits = limit / per; // entries that fit
    let mut applied = 2;
    loop {
        let r = set(&mut h, 10 + (applied % 2) as u32);
        if r["type"] == "error" {
            assert_eq!(r["error"]["reason"], "E_BUDGET", "{r}");
            assert_eq!(r["error"]["expected"], json!([limit.to_string()]), "{r}");
            break;
        }
        applied += 1;
        assert!(applied <= fits + 1, "not refused in time");
    }
    assert_eq!(
        applied, fits,
        "refused at exactly the first entry over the limit"
    );
    // Removing the instance frees no log bytes; its undo would write them again.
    let r = h.cmd("a", json!([{"op":"remove_program","program":"p1"}]));
    let seq = r["seq"].as_u64().unwrap();
    let r = h.cmd("a", json!([{"op":"undo","seq":seq}]));
    assert_eq!(r["error"]["reason"], "E_BUDGET", "{r}");
}
