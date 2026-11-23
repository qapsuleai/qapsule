//! Audit additions (RF1/S1 audit B2–B4): a data event that meets a condition and triggers
//! an intent at once (full and incremental paths); a public-view owner whose decisions
//! reach the authority's world through the reverse translation; two watches matching two
//! story events of one engine look.

use serde_json::{json, Value};

use super::order::{data_steps, Run};
use super::rig::{init_of, Rig};
use super::snap::Book;

/// B2: transcript 69's world; the host adds an intent and every reader a condition
/// subscription on the same threshold, so one data event emits both `condition_met`
/// and `intent_triggered`.
fn both(name: &'static str, incremental: bool) {
    let mut book = Book::new(name);
    let mut rig = Rig::of("69-watch");
    rig.session_mut().set_incremental(incremental);
    let mut r = Run::new(&mut book, "both", rig);
    r.hello("a", "ana", "phone");
    r.hello("e", "eve", "headless");
    r.hello("h", "host", "headless");
    let cond = json!({"kind":"threshold","series":"px","op":"gt","value":111.5});
    for c in ["a", "e", "h"] {
        r.sub(c, "co", json!({"kind":"condition","condition":cond}));
        r.sub(c, "in", json!({"kind":"intents"}));
        r.sub(c, "cl", json!({"kind":"claims"}));
    }
    r.cmd(
        "intent",
        "h",
        json!([{"op":"add_intent","id":"i-up","text":"yukarı","trigger":cond}]),
    );
    for (i, ev) in data_steps("69-watch").iter().enumerate() {
        r.data(&format!("bar{i}"), ev);
    }
    r.finish();
    book.check();
}

#[test]
fn charac_session_order_both() {
    both("order-both", true);
}

#[test]
fn charac_session_order_both_full() {
    both("order-both-full", false);
}

/// B3: transcript 39's world with the owner entitled to the bars only: a public-view
/// writer with the decision right. Its own finding with a capsule, approved, rejected
/// and undone in the public world, reaches the authority's world through the reverse
/// translation (`SetCapsuleState`, `PushLedger`); ana (entitled) reads the authority.
#[test]
fn charac_session_worlds_public_owner() {
    let mut book = Book::new("worlds-public-owner");
    let mut v: Value = init_of("39-capsule-entitlement");
    v["data"]["entitlements"]["arkin"] = json!(["bist/THYAO"]);
    let mut r = Run::new(&mut book, "po", Rig::json(v));
    r.hello("o", "arkin", "headless");
    r.hello("a", "ana", "headless");
    r.hello("c", "can", "headless");
    for c in ["o", "a", "c"] {
        r.sub(c, "ch", json!({"kind":"changes"}));
        r.sub(c, "ka", json!({"kind":"capsules","all":true}));
    }
    for (i, ev) in data_steps("39-capsule-entitlement").iter().enumerate() {
        r.data(&format!("bar{i}"), ev);
    }
    r.cmd(
        "fact",
        "o",
        json!([{"op":"add_fact","id":"pf","kind":"note","series":"px","bar":10,"text":"t"}]),
    );
    r.cmd(
        "finding",
        "o",
        json!([{"op":"add_finding","id":"pfnd","claim":"c","facts":["pf"],"confidence":0.5,
            "invalidation":{"kind":"threshold","series":"px","op":"lt","value":1.0},"capsule":"Başlık"}]),
    );
    for c in ["o", "a"] {
        r.sub(
            c,
            &format!("fi_{c}"),
            json!({"kind":"finding","finding":"pfnd"}),
        );
    }
    r.cmd(
        "approve",
        "o",
        json!([{"op":"approve_finding","finding":"pfnd","note":"iyi"}]),
    );
    r.cmd(
        "reject",
        "o",
        json!([{"op":"reject_finding","finding":"pfnd"}]),
    );
    r.cmd("undo", "o", json!([{"op":"undo"}]));
    for (c, l) in [("o", "q_log_o"), ("a", "q_log_a"), ("c", "q_log_c")] {
        r.query(l, c, json!({"kind":"log"}));
    }
    for (c, l) in [("o", "q_get_o"), ("a", "q_get_a")] {
        r.query(l, c, json!({"kind":"get","id":"pfnd"}));
    }
    // The authority's own document: the decisions arrived there.
    let caps: Vec<String> = r
        .rig
        .session()
        .document()
        .capsules
        .iter()
        .filter(|c| c.finding == "pfnd")
        .map(|c| format!("{}={:?}", c.id, c.state))
        .collect();
    let ledger = r
        .rig
        .session()
        .document()
        .ledger
        .entries
        .iter()
        .filter(|e| e.finding == "pfnd")
        .count();
    r.book.note(
        "po.authority_doc",
        &format!("capsules [{}] ledger_rows={ledger}", caps.join(",")),
    );
    r.finish();
    book.check();
}

/// B4: transcript 59's replay host (two instruments, the engine on both): the owner
/// leaves two watches on every instrument; replay steps whose engine look makes several
/// story events, so `intent_matched` lines cross events and watches.
#[test]
fn charac_session_watch_cross_order() {
    let mut book = Book::new("watch-cross");
    let mut r = Run::new(&mut book, "x", Rig::of("59-viewer"));
    r.hello("a", "arkin", "headless");
    r.sub("a", "in", json!({"kind":"intents"}));
    r.cmd(
        "w1",
        "a",
        json!([{"op":"add_watch","text":"hepsi","match":{}}]),
    );
    r.cmd(
        "w2",
        "a",
        json!([{"op":"add_watch","text":"hepsi 2","match":{},"max_matches":40}]),
    );
    for i in 0..60 {
        r.send(
            &format!("step{i}"),
            "a",
            json!({"type":"replay","id":format!("r{i}"),"control":{"kind":"step","n":1}}),
        );
    }
    r.finish();
    book.check();
}
