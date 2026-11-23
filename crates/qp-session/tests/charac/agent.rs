//! The agent allow-list (§25.2–§25.3) and the role checks: one representative op per
//! kind × writer (proposer agent, editor agent, owner, viewer) × mode, under policy
//! `r5b`, on a marked agent connection without the policy, and with neither.

use serde_json::{json, Value};

use super::opcase::record;
use super::rig::{init_of, Rig};
use super::snap::Book;

const LAST_T: i64 = 1_759_881_600_000;

/// One valid op of every kind on the `thyao` document (a proposal `prop-1` and an
/// intent `i1` exist when they run).
fn samples() -> Vec<(&'static str, Value)> {
    let bar = json!({"t":LAST_T + 86_400_000,"o":1.0,"h":2.0,"l":0.5,"c":1.5,"v":10.0});
    let view = json!({"id":"v2","x":{"kind":"time"},"y":[{"kind":"linear"}],
        "layers":[{"mark":"candle","series":"px","panel":0}]});
    let cond = json!({"kind":"threshold","series":"px","op":"lt","value":100.0});
    vec![
        ("put_series", json!({"op":"put_series","series":{"id":"e9","source":{"kind":"study","study":"ema","inputs":["px"]}}})),
        ("add_study", json!({"op":"add_study","study":"ema","inputs":["px"]})),
        ("add_study_existing", json!({"op":"add_study","id":"rsi14","study":"rsi","inputs":["px"]})),
        ("set_params", json!({"op":"set_params","series":"rsi14","params":{"period":9}})),
        ("put_view", json!({"op":"put_view","view":view})),
        ("add_layer", json!({"op":"add_layer","view":"main","layer":{"mark":"line","series":"px","panel":0}})),
        ("remove_layer", json!({"op":"remove_layer","view":"main","index":4})),
        ("set_view_range", json!({"op":"set_view_range","view":"main","from_bar":150,"to_bar":199})),
        ("add_fact", json!({"op":"add_fact","kind":"note","series":"px","bar":3,"text":"t"})),
        ("add_finding", json!({"op":"add_finding","claim":"c","facts":["fact-rsi-cross70"],"confidence":0.5,"invalidation":cond})),
        ("update_finding", json!({"op":"update_finding","finding":"fnd-overbought","confidence":0.7})),
        ("approve_finding", json!({"op":"approve_finding","finding":"fnd-overbought"})),
        ("reject_finding", json!({"op":"reject_finding","finding":"fnd-overbought"})),
        ("add_intent", json!({"op":"add_intent","text":"t"})),
        ("set_intent_active", json!({"op":"set_intent_active","intent":"i1","active":false})),
        ("add_watch", json!({"op":"add_watch","text":"t","match":{"instrument":"THYAO"}})),
        ("add_comment", json!({"op":"add_comment","about":{"capsule":"cap-overbought"},"lang":"tr","text":"RSI yüksek","cites":["fnd-overbought"]})),
        ("add_narrative", json!({"op":"add_narrative","about":"fnd-overbought","steps":[{"at_ms":0,"target":{"view":"main"},"say":"s"}]})),
        ("show_finding", json!({"op":"show_finding","finding":"fnd-overbought"})),
        ("insert_layer", json!({"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":0,"encoding":{"y":"c"}}})),
        ("delete_layer", json!({"op":"delete_layer","view":"main","layer":"l1"})),
        ("split_layer", json!({"op":"split_layer","view":"main","layer":"l1"})),
        ("move_layer", json!({"op":"move_layer","view":"main","layer":"l4","before":"l0"})),
        ("set_encoding", json!({"op":"set_encoding","view":"main","layer":"l1","encoding":{"width":2}})),
        ("insert_panel", json!({"op":"insert_panel","view":"main","panel":{"y":{"kind":"linear"}}})),
        ("delete_panel", json!({"op":"delete_panel","view":"main","panel":2})),
        ("move_panel", json!({"op":"move_panel","view":"main","panel":2,"before":0})),
        ("set_panel", json!({"op":"set_panel","view":"main","panel":1,"weight":2})),
        ("set_scale", json!({"op":"set_scale","view":"main","scale":"y0","kind":"log"})),
        ("set_emphasis", json!({"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l2"},"kind":"focus"}]})),
        ("remove", json!({"op":"remove","id":"macd"})),
        ("append_bars", json!({"op":"append_bars","series":"px","bars":[bar]})),
        ("update_last_bar", json!({"op":"update_last_bar","series":"px","bar":{"t":LAST_T,"o":125.26,"h":127.0,"l":124.47,"c":126.5,"v":1.0}})),
        ("undo", json!({"op":"undo"})),
        ("accept_proposal", json!({"op":"accept_proposal","proposal":"prop-1"})),
        ("reject_proposal", json!({"op":"reject_proposal","proposal":"prop-1"})),
        ("set_audience", json!({"op":"set_audience","audience":{"group":"future"}})),
        ("set_sensitivity", json!({"op":"set_sensitivity","sensitivity":"all"})),
    ]
}

const WRITERS: [(&str, &str); 4] = [
    ("s", "scout"),
    ("e", "editor"),
    ("a", "arkin"),
    ("v", "viewer"),
];

/// A fresh rig of `init` with the four writers, a pending proposal `prop-1` and an
/// intent `i1` (both by the owner), and the owner's last entry undoable.
fn rig(init: &Value, mark: bool) -> Rig {
    let mut r = Rig::json(init.clone());
    if mark {
        r.session_mut().mark_agent_connection("s");
        r.session_mut().mark_agent_connection("e");
    }
    for (c, who) in WRITERS {
        r.hello(c, who, "headless");
    }
    r.sub("v", "ch", json!({"kind":"changes"}));
    let p = r.send("a", &json!({"type":"command","id":"p","mode":"propose","ops":[{"op":"add_intent","text":"öneri"}]}));
    assert_eq!(p[0]["msg"]["proposal"], "prop-1", "{p:?}");
    let i = r.cmd("a", "i", json!([{"op":"add_intent","id":"i1","text":"t"}]));
    assert_eq!(i[0]["msg"]["status"], "applied", "{i:?}");
    r
}

fn matrix(book: &mut Book, init: &Value, mark: bool, writers: &[(&str, &str)]) {
    for (name, op) in samples() {
        for (c, who) in writers {
            for mode in ["auto", "direct", "propose"] {
                let mut r = rig(init, mark);
                let before = r.log_len();
                let out = r.send(c, &json!({"type":"command","id":"c","mode":mode,"ops":[op.clone()]}));
                record(book, &format!("{name}.{who}.{mode}"), &r, before, &out);
            }
        }
    }
}

fn thyao_init() -> Value {
    let p = super::rig::root().join("fixtures/protocol/init/thyao.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn charac_session_agent_policy_r5b() {
    let mut book = Book::new("agent-r5b");
    let init = init_of("66-agent-cannot-write");
    assert_eq!(init["agent_policy"], "r5b");
    matrix(&mut book, &init, false, &WRITERS);
    // Mixed classes in one command; a direct op beside a proposal-only one.
    for (name, ops) in [
        ("mixed.watch_view", json!([{"op":"add_watch","text":"t","match":{"instrument":"THYAO"}},
            {"op":"set_view_range","view":"main","from_bar":150,"to_bar":199}])),
        ("mixed.view_fact", json!([{"op":"set_view_range","view":"main","from_bar":150,"to_bar":199},
            {"op":"add_fact","kind":"note","series":"px","bar":3,"text":"t"}])),
        ("mixed.views", json!([{"op":"set_view_range","view":"main","from_bar":150,"to_bar":199},
            {"op":"set_scale","view":"main","scale":"y0","kind":"log"}])),
    ] {
        let mut r = rig(&init, false);
        let before = r.log_len();
        let out = r.cmd("e", "c", ops);
        record(&mut book, name, &r, before, &out);
    }
    book.check();
}

#[test]
fn charac_session_agent_marked_connection() {
    let mut book = Book::new("agent-marked");
    let init = thyao_init();
    // Marked connections, no policy: the agent rule holds on `s` and `e` only.
    matrix(&mut book, &init, true, &WRITERS[..2]);
    // Neither: plain roles.
    let mut plain = Book::new("agent-none");
    matrix(&mut plain, &init, false, &WRITERS[..2]);
    // An owner on a marked connection: its decisions are refused.
    let mut r = rig(&init, false);
    r.session_mut().mark_agent_connection("a");
    let before = r.log_len();
    let out = r.cmd("a", "c", json!([{"op":"accept_proposal","proposal":"prop-1"}]));
    record(&mut book, "owner_marked.accept", &r, before, &out);
    book.note(
        "marks",
        &format!(
            "a={} s={} policy={}",
            r.session().is_agent_connection("a"),
            r.session().is_agent_connection("s"),
            r.session().agent_policy()
        ),
    );
    book.check();
    plain.check();
}
