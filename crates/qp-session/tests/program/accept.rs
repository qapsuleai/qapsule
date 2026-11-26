//! Acceptance and undo go through the same rules as a direct command: the guards on a
//! proposal's ops, the live budget of what an undo restores, one owner identity.

use serde_json::json;

use crate::rig::{reason, H, SMA_TEXT};

/// Adds programs `b0`, `b1`, … by `arkin` until the budget refuses; how many fit.
fn fill(h: &mut H) -> usize {
    let ir = h.ir(SMA_TEXT);
    for k in 0..40 {
        let r = h.cmd(
            "a",
            json!([{"op":"add_program","id":format!("b{k}"),"program":ir,"inputs":["px"]}]),
        );
        if r["type"] == "error" {
            assert_eq!(reason(&r), "E_BUDGET", "{r}");
            return k;
        }
    }
    panic!("the budget never refused");
}

#[test]
fn the_guards_hold_when_a_proposal_is_accepted() {
    let mut h = H::new();
    // Plain series named like the outputs a program will take, so the proposals are
    // valid when made.
    let r = h.cmd(
        "a",
        json!([{"op":"add_study","id":"p1.up","study":"rsi","inputs":["px"]}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let cmp = json!({"kind":"compare","left":"rsi14","op":"gt","right":"p1.up"});
    let r = h.cmd_mode(
        "a",
        "propose",
        json!([{"op":"add_intent","text":"x","trigger":cmp}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    let intent = r["proposal"].as_str().unwrap().to_owned();
    let mut forged = h.series("rsi14");
    forged["id"] = json!("p1.avg");
    let r = h.cmd_mode("a", "propose", json!([{"op":"put_series","series":forged}]));
    assert_eq!(r["status"], "proposed", "{r}");
    let put = r["proposal"].as_str().unwrap().to_owned();
    // Now the names are a program's outputs (`up` an event series).
    assert_eq!(
        h.cmd("a", json!([{"op":"remove","id":"p1.up"}]))["status"],
        "applied"
    );
    assert_eq!(h.add("p1", SMA_TEXT)["status"], "applied");
    for (pid, sub) in [(intent, "E_EVENT_OPERAND"), (put, "E_PROGRAM_SERIES")] {
        let r = h.cmd("a", json!([{"op":"accept_proposal","proposal":pid}]));
        assert_eq!(reason(&r), sub, "{r}");
    }
    assert!(h
        .s
        .document()
        .proposals
        .iter()
        .all(|p| { serde_json::to_value(p.state).unwrap() == "pending" }));
}

#[test]
fn undo_cannot_restore_past_the_budget() {
    let mut h = H::new();
    let k = fill(&mut h);
    let r = h.cmd("a", json!([{"op":"remove_program","program":"b0"}]));
    assert_eq!(r["status"], "applied", "{r}");
    let removed = r["seq"].as_u64().unwrap();
    let ir = h.ir(SMA_TEXT);
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"refill","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    // Undoing the removal would make k + 1 instances.
    let r = h.cmd("a", json!([{"op":"undo","seq":removed}]));
    assert_eq!(reason(&r), "E_BUDGET", "{r}");
    let n =
        h.s.document()
            .series
            .iter()
            .filter(|s| s.id.ends_with(".avg"))
            .count();
    assert_eq!(n, k);
    // With room again, the same undo goes through.
    assert_eq!(
        h.cmd("a", json!([{"op":"remove_program","program":"refill"}]))["status"],
        "applied"
    );
    let r = h.cmd("a", json!([{"op":"undo","seq":removed}]));
    assert_eq!(r["status"], "applied", "{r}");
    h.assert_replay();
}

#[test]
fn a_direct_and_an_accepted_program_count_for_the_same_owner() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    let r = h.cmd_mode(
        "s",
        "auto",
        json!([{"op":"add_program","id":"viaagent","program":ir,"inputs":["px"]}]),
    );
    let pid = r["proposal"].as_str().unwrap().to_owned();
    let r = h.cmd("a", json!([{"op":"accept_proposal","proposal":pid}]));
    assert_eq!(r["status"], "applied", "{r}");
    let direct = h.add("direct", SMA_TEXT);
    assert_eq!(direct["status"], "applied", "{direct}");
    let owner = |h: &H, id: &str| h.series(id)["source"]["program"]["owner"].clone();
    assert_eq!(owner(&h, "viaagent.avg"), owner(&h, "direct.avg"));
    // Both count against one budget: 2 fit before, so 2 fewer fit now.
    let mut fresh = H::new();
    let all = fill(&mut fresh);
    assert_eq!(fill(&mut h), all - 2);
}
