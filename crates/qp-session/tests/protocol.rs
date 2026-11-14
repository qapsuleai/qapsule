//! Boundary, errors, roles, undo, proposals, subscriptions and ordering.

mod common;

use common::{code, init, H};
use serde_json::{json, Value};

#[test]
fn boundary_errors() {
    let mut h = H::new();
    let r = h.raw("a", "{not json");
    assert_eq!(code(&r[0]["msg"]), "parse");
    assert!(r[0]["msg"].get("id").is_none());
    let big = format!(
        "{{\"type\":\"clock\",\"now\":1,\"x\":\"{}\"}}",
        "a".repeat(1_100_000)
    );
    assert_eq!(code(&h.raw("a", &big)[0]["msg"]), "limit");
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    assert_eq!(code(&h.raw("a", &deep)[0]["msg"]), "limit");
    let r = h.raw("a", r#"{"type":"comand","id":"x"}"#);
    let m = &r[0]["msg"];
    assert_eq!(code(m), "schema");
    assert_eq!(m["id"], "x");
    assert_eq!(m["error"]["path"], "/type");
    assert_eq!(m["error"]["did_you_mean"], json!(["command"]));
    let r = h.raw("a", r#"{"type":"query","id":"q","query":{"kind":"study"}}"#);
    assert_eq!(r[0]["msg"]["error"]["path"], "/query");
    assert_eq!(r[0]["msg"]["error"]["expected"], json!(["series"]));
    let r = h.raw(
        "a",
        r#"{"type":"command","id":"c","ops":[{"op":"set_params","series":"macd","params":{"fast":"x"}}]}"#,
    );
    assert_eq!(r[0]["msg"]["error"]["path"], "/ops/0/params/fast");
    let r = h.raw(
        "a",
        r#"{"type":"input","id":"i","gesture":{"kind":"tap","view":"main","panel":-1,"bar":1}}"#,
    );
    assert_eq!(r[0]["msg"]["error"]["path"], "/gesture/panel");
}

#[test]
fn handshake_and_limits() {
    let mut h = H::new();
    let r = h.one(
        "a",
        json!({"type":"query","id":"q","query":{"kind":"describe"}}),
    );
    assert_eq!(code(&r), "handshake");
    assert_eq!(r["id"], "q");
    let r = h.one(
        "a",
        json!({"type":"hello","protocol":"qp/0","principal":"arkn","surface":{"kind":"desktop"}}),
    );
    assert_eq!(r["error"]["did_you_mean"], json!(["arkin"]));
    let w = h.one(
        "a",
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop","max_depth":"sentence"},"features":["narrative","bogus"]}),
    );
    assert_eq!(w["profile"]["depth"], "sentence");
    assert_eq!(w["features"], json!(["narrative"]));
    let ops: Vec<Value> = (0..65)
        .map(|_| json!({"op":"add_intent","text":"x"}))
        .collect();
    let r = h.cmd("a", "c", Value::Array(ops));
    assert_eq!(code(&r), "limit");
    assert_eq!(r["error"]["expected"], json!(["64"]));
    let r = h.cmd(
        "a",
        "c",
        json!([{"op":"add_intent","text":"x".repeat(5000)}]),
    );
    assert_eq!(code(&r), "limit");
    assert_eq!(r["error"]["path"], "/ops/0/text");
    for i in 0..32 {
        let r = h.one(
            "a",
            json!({"type":"subscribe","id":format!("s{i}"),"topic":{"kind":"proposals"}}),
        );
        assert_eq!(r["status"], "subscribed");
    }
    let r = h.one(
        "a",
        json!({"type":"subscribe","id":"s99","topic":{"kind":"proposals"}}),
    );
    assert_eq!(code(&r), "limit");
    let r = h.one(
        "a",
        json!({"type":"subscribe","id":"s0","topic":{"kind":"proposals"}}),
    );
    assert_eq!(code(&r), "duplicate");
    let r = h.one("a", json!({"type":"unsubscribe","id":"u","sub":"s0"}));
    assert_eq!(r["status"], "unsubscribed");
    let r = h.one("a", json!({"type":"unsubscribe","id":"u","sub":"s0"}));
    assert_eq!(code(&r), "not_found");
}

#[test]
fn roles() {
    let mut h = H::new();
    h.hello("v", "viewer", "phone");
    h.hello("f", "feed", "headless");
    h.hello("s", "scout", "headless");
    h.hello("e", "editor", "headless");
    h.hello("a", "arkin", "desktop");
    assert_eq!(
        code(&h.cmd("v", "c", json!([{"op":"add_intent","text":"x"}]))),
        "forbidden"
    );
    assert_eq!(code(&h.query("f", json!({"kind":"describe"}))), "forbidden");
    assert_eq!(
        code(&h.one("a", json!({"type":"clock","now":2_000_000_000_000_i64}))),
        "forbidden"
    );
    let r = h.one(
        "s",
        json!({"type":"command","id":"c","mode":"propose","ops":[{"op":"undo"}]}),
    );
    assert_eq!(code(&r), "forbidden");
    let r = h.cmd(
        "e",
        "c",
        json!([{"op":"approve_finding","finding":"fnd-overbought"}]),
    );
    assert_eq!(
        r["status"], "proposed",
        "an editor proposes what it may not decide"
    );
    let r = h.one(
        "a",
        json!({"type":"command","id":"c","mode":"propose","ops":[{"op":"add_intent","text":"x"}]}),
    );
    assert_eq!(r["status"], "proposed", "an owner may choose to propose");
    let r = h.cmd(
        "v",
        "c",
        json!([{"op":"accept_proposal","proposal":"prop-1"}]),
    );
    assert_eq!(code(&r), "forbidden");
    let r = h.one(
        "v",
        json!({"type":"attend","id":"a","directive":{"to":"arkin","steps":[{"target":{}}]}}),
    );
    assert_eq!(code(&r), "forbidden");
    let r = h.cmd(
        "a",
        "c",
        json!([{"op":"accept_proposal","proposal":"prop-1"},{"op":"add_intent","text":"x"}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn undo_rules() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.hello("e", "editor", "headless");
    assert_eq!(
        code(&h.cmd("a", "c", json!([{"op":"undo"}]))),
        "undo_conflict"
    );
    assert_eq!(
        code(&h.cmd("a", "c", json!([{"op":"undo","seq":9}]))),
        "not_found"
    );
    h.cmd("a", "c1", json!([{"op":"add_intent","id":"i","text":"x"}]));
    h.cmd(
        "e",
        "c2",
        json!([{"op":"set_intent_active","intent":"i","active":false}]),
    );
    let r = h.cmd("a", "c3", json!([{"op":"undo"}]));
    assert_eq!(code(&r), "undo_conflict");
    assert_eq!(r["error"]["conflicts"], json!([2]));
    // The editor undoes its own entry; then the owner's undo goes through.
    assert_eq!(h.cmd("e", "c4", json!([{"op":"undo"}]))["seq"], 3);
    assert_eq!(h.cmd("a", "c5", json!([{"op":"undo"}]))["seq"], 4);
    assert!(h.s.document().intents.is_empty());
    assert_eq!(
        code(&h.cmd("a", "c6", json!([{"op":"undo","seq":1}]))),
        "undo_conflict"
    );
    // Redo (undo the undo), then nothing left of arkin's to undo but the redo.
    assert_eq!(h.cmd("a", "c7", json!([{"op":"undo","seq":4}]))["seq"], 5);
    assert_eq!(h.s.document().intents.len(), 1);
    h.assert_replay();
}

#[test]
fn proposals_lifecycle_and_failed_accept_stays_pending() {
    let mut i = init();
    i.proposal_ttl_ms = Some(1000);
    let mut h = H::with(i);
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h.hello("h", "host", "headless");
    let r = h.cmd(
        "s",
        "c1",
        json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross70"],"confidence":0.5,"invalidation":{"kind":"text","text":"x"}}]),
    );
    assert_eq!(r["proposal"], "prop-1");
    let p = &h.s.document().proposals[0];
    assert_eq!(p.expires_at, Some(1_760_000_001_000));
    // The fact it rests on disappears: acceptance fails, the proposal stays pending.
    h.cmd("a", "c2", json!([{"op":"remove","id":"fnd-overbought"}]));
    assert_eq!(
        code(&h.cmd("a", "c3", json!([{"op":"remove","id":"cap-overbought"}]))),
        "<none>"
    );
    h.cmd("a", "c4", json!([{"op":"remove","id":"fnd-overbought"}]));
    h.cmd("a", "c5", json!([{"op":"remove","id":"fact-rsi-cross70"}]));
    let r = h.cmd(
        "a",
        "c6",
        json!([{"op":"accept_proposal","proposal":"prop-1"}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0");
    let st = serde_json::to_value(h.s.document().proposals[0].state).unwrap();
    assert_eq!(st, "pending");
    let out = h.send("h", json!({"type":"clock","now":1_760_000_001_000_i64}));
    assert!(out.is_empty());
    let last = h.s.log().last().unwrap();
    assert_eq!(last.author, "qp");
    let st = serde_json::to_value(h.s.document().proposals[0].state).unwrap();
    assert_eq!(st, "expired");
    h.assert_replay();
}

#[test]
fn base_conflicts_ignore_undone_entries() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.cmd(
        "a",
        "c1",
        json!([{"op":"set_params","series":"macd","params":{"fast":10}}]),
    );
    h.cmd("a", "c2", json!([{"op":"undo"}]));
    let r = h.one(
        "a",
        json!({"type":"command","id":"c3","base":0,"ops":[{"op":"set_params","series":"macd","params":{"fast":9}}]}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.one(
        "a",
        json!({"type":"command","id":"c4","base":2,"ops":[{"op":"set_params","series":"macd","params":{"fast":8}}]}),
    );
    assert_eq!(code(&r), "conflict");
    assert_eq!(r["error"]["conflicts"], json!([3]));
    assert_eq!(r["error"]["path"], "/ops/0");
}

#[test]
fn notification_order_and_intents() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h.hello("f", "feed", "headless");
    h.cmd(
        "a",
        "c1",
        json!([{"op":"add_intent","id":"up","text":"close above 130","trigger":{"kind":"threshold","series":"px","op":"gt","value":130.0}},
               {"op":"update_finding","finding":"fnd-overbought","invalidation":{"kind":"threshold","series":"px","op":"gt","value":130.0}}]),
    );
    h.one(
        "s",
        json!({"type":"subscribe","id":"i","topic":{"kind":"intents"}}),
    );
    h.one("s", json!({"type":"subscribe","id":"k","topic":{"kind":"condition","condition":{"kind":"threshold","series":"px","op":"gt","value":130.0}}}));
    h.one(
        "a",
        json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}),
    );
    h.one(
        "s",
        json!({"type":"subscribe","id":"fs","topic":{"kind":"finding","finding":"fnd-overbought"}}),
    );
    h.one(
        "a",
        json!({"type":"subscribe","id":"ch2","topic":{"kind":"changes","ids":["cap-overbought"]}}),
    );
    let t = h.s.document().series[0].bars().unwrap().last().unwrap().t + 86_400_000;
    let out = h.send(
        "f",
        json!({"type":"command","id":"b","ops":[{"op":"append_bars","series":"px","bars":[{"t":t,"o":125.0,"h":132.0,"l":124.0,"c":131.0,"v":1.0}]}]}),
    );
    let kinds: Vec<String> = out
        .iter()
        .map(|o| {
            format!(
                "{}:{}:{}",
                o["conn"].as_str().unwrap(),
                o["msg"]["type"].as_str().unwrap(),
                o["msg"]["event"]["kind"]
                    .as_str()
                    .or(o["msg"]["status"].as_str())
                    .unwrap()
            )
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "f:ack:applied",
            "a:notify:change",        // entry 3 (bars) to `ch`
            "a:notify:change",        // entry 4 (engine) to `ch`
            "a:notify:change",        // entry 4 to `ch2` (touches cap-overbought)
            "s:notify:finding_state", // entry 4
            "s:notify:condition_met",
            "s:notify:intent_triggered",
        ]
    );
    assert_eq!(out[5]["msg"]["event"]["seq"], 2);
    assert_eq!(out[6]["msg"]["event"]["intent"], "up");
    // Edge-triggered: another bar above 130 does not fire again.
    let out = h.send(
        "f",
        json!({"type":"command","id":"b2","ops":[{"op":"append_bars","series":"px","bars":[{"t":t + 86_400_000,"o":131.0,"h":133.0,"l":130.5,"c":132.0,"v":1.0}]}]}),
    );
    assert!(out
        .iter()
        .all(|o| o["msg"]["event"]["kind"] != "intent_triggered"));
    h.assert_replay();
}

#[test]
fn identical_inputs_give_identical_outputs() {
    let script = |h: &mut H| -> Vec<Value> {
        let mut all = Vec::new();
        all.extend(h.send("a", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}})));
        all.extend(h.send(
            "a",
            json!({"type":"subscribe","id":"s","topic":{"kind":"changes"}}),
        ));
        all.extend(h.send("a", json!({"type":"command","id":"c","ops":[{"op":"add_study","study":"macd","inputs":["px"],"show":{"view":"main"}}]})));
        all.extend(h.send(
            "a",
            json!({"type":"query","id":"q","query":{"kind":"present"}}),
        ));
        all.extend(h.send("a", json!({"type":"input","id":"i","gesture":{"kind":"zoom","view":"main","factor":3.0,"anchor_bar":170}})));
        all
    };
    let a = script(&mut H::new());
    let b = script(&mut H::new());
    assert_eq!(a, b);
}

#[test]
fn frames() {
    let mut h = H::new();
    let r = h.s.handle_frame(r#"{"conn":"z","msg":"{oops"}"#);
    assert_eq!(r[0].conn, "z");
    let r = serde_json::to_value(h.s.handle_frame("nope")).unwrap();
    assert_eq!(r[0]["conn"], "");
    assert_eq!(r[0]["msg"]["error"]["code"], "parse");
    let r = serde_json::to_value(h.s.handle_frame(r#"{"conn":"z","msg":{},"x":1}"#)).unwrap();
    assert_eq!(r[0]["msg"]["error"]["code"], "schema");
}

#[test]
fn delete_then_undo_is_byte_identical() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    let before = serde_json::to_vec(h.s.document()).unwrap();
    // A series in the middle of its collection, with a layer in the middle of a view.
    assert_eq!(
        h.cmd("a", "c1", json!([{"op":"remove","id":"bb20"}]))["seq"],
        1
    );
    assert_eq!(
        h.cmd("a", "c2", json!([{"op":"remove","id":"cap-overbought"}]))["seq"],
        2
    );
    assert_eq!(h.cmd("a", "c3", json!([{"op":"undo"}]))["seq"], 3);
    assert_eq!(h.cmd("a", "c4", json!([{"op":"undo"}]))["seq"], 4);
    assert_eq!(before, serde_json::to_vec(h.s.document()).unwrap());
    h.assert_replay();
}

#[test]
fn undo_authorization() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h.hello("e", "editor", "headless");
    // An editor's proposal accepted by the owner: the owner approved it, but did not
    // author it, so a bare `undo` does not pick it.
    let r = h.one("e", json!({"type":"command","id":"p","mode":"propose","reason":"why","ops":[{"op":"add_intent","id":"i","text":"x"}]}));
    assert_eq!(r["status"], "proposed");
    h.cmd(
        "a",
        "c",
        json!([{"op":"accept_proposal","proposal":"prop-1"}]),
    );
    assert_eq!(h.s.log()[0].reason.as_deref(), Some("why"));
    assert_eq!(h.s.log()[1].reason.as_deref(), Some("why"));
    assert_eq!(
        code(&h.cmd("a", "u", json!([{"op":"undo"}]))),
        "undo_conflict"
    );
    // The proposer has no write permission: undo is never proposable.
    assert_eq!(code(&h.cmd("s", "u", json!([{"op":"undo"}]))), "forbidden");
    // The editor authored the accepted entry, but it carries a proposal decision.
    let r = h.cmd("e", "u", json!([{"op":"undo","seq":2}]));
    assert_eq!(code(&r), "forbidden");
    // The owner may undo it (owner and decide).
    assert_eq!(
        h.cmd("a", "u", json!([{"op":"undo","seq":2}]))["status"],
        "applied"
    );
    // An editor rejecting nothing it may decide: approve via owner, editor cannot undo it.
    h.cmd(
        "a",
        "r",
        json!([{"op":"reject_finding","finding":"fnd-overbought","note":"n"}]),
    );
    assert_eq!(
        code(&h.cmd("e", "u", json!([{"op":"undo","seq":4}]))),
        "forbidden"
    );
    h.assert_replay();
}
