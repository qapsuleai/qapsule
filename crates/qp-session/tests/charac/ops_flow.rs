//! `expand_op`, grammar arms (`expand_grammar`), undo, proposals, audiences, comments and
//! watches (`expand_watch`), and the role checks in front of expansion.

use serde_json::{json, Value};

use super::opcase::{case, case_by, fresh, on, record};
use super::rig::Rig;
use super::snap::Book;

#[test]
fn charac_session_ops_grammar() {
    let mut b = Book::new("ops-grammar");
    let line = json!({"mark":"line","series":"bb20","panel":1,"encoding":{"y":"upper"}});
    case(
        &mut b,
        "insert_layer.ok.after",
        &[],
        json!([{"op":"insert_layer","view":"main","after":"l0",
        "layer":{"id":"close","mark":"line","series":"px","panel":0,"encoding":{"y":"c"}}}]),
    );
    case(
        &mut b,
        "insert_layer.ok.before",
        &[],
        json!([{"op":"insert_layer","view":"main","before":"l3","layer":line}]),
    );
    case(
        &mut b,
        "insert_layer.ok.origin",
        &[],
        json!([{"op":"insert_layer","view":"main","layer":{"id":"lvl","mark":"rule","panel":1,
        "encoding":{"y":70},"origin":{"/encoding/y":{"kind":"fact","fact":"fact-rsi-cross70","field":"level"}}}}]),
    );
    case(
        &mut b,
        "insert_layer.err.no_origin",
        &[],
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":0,"encoding":{"y":123.45}}}]),
    );
    case(
        &mut b,
        "insert_layer.err.duplicate",
        &[],
        json!([{"op":"insert_layer","view":"main","layer":{"id":"l0","mark":"line","series":"rsi14","panel":1}}]),
    );
    case(
        &mut b,
        "insert_layer.err.after",
        &[],
        json!([{"op":"insert_layer","view":"main","after":"nope","layer":line}]),
    );
    case(
        &mut b,
        "delete_layer.ok",
        &[],
        json!([{"op":"delete_layer","view":"main","layer":"l1"}]),
    );
    case(
        &mut b,
        "delete_layer.err.missing",
        &[],
        json!([{"op":"delete_layer","view":"main","layer":"l9"}]),
    );
    case(
        &mut b,
        "split_layer.ok",
        &[],
        json!([{"op":"split_layer","view":"main","layer":"l1"}]),
    );
    case(
        &mut b,
        "split_layer.err.single",
        &[],
        json!([{"op":"split_layer","view":"main","layer":"l0"}]),
    );
    case(
        &mut b,
        "split_layer.err.missing",
        &[],
        json!([{"op":"split_layer","view":"main","layer":"l7"}]),
    );
    case(
        &mut b,
        "move_layer.ok.panel",
        &[],
        json!([{"op":"move_layer","view":"main","layer":"l1","panel":2,"after":"l4"}]),
    );
    case(
        &mut b,
        "move_layer.ok.order",
        &[],
        json!([{"op":"move_layer","view":"main","layer":"l4","before":"l0"}]),
    );
    case(
        &mut b,
        "move_layer.err.panel",
        &[],
        json!([{"op":"move_layer","view":"main","layer":"l1","panel":9}]),
    );
    case(
        &mut b,
        "move_layer.err.missing",
        &[],
        json!([{"op":"move_layer","view":"main","layer":"lx","after":"l0"}]),
    );
    case(
        &mut b,
        "set_encoding.ok",
        &[],
        json!([{"op":"set_encoding","view":"main","layer":"l1","encoding":{"color":"down","width":2}}]),
    );
    case(
        &mut b,
        "set_encoding.ok.unset_label",
        &[],
        json!([{"op":"set_encoding","view":"main","layer":"l1","unset":["color"],"label":"BB"}]),
    );
    case(
        &mut b,
        "set_encoding.err.color",
        &[],
        json!([{"op":"set_encoding","view":"main","layer":"l1","encoding":{"color":"dwon"}}]),
    );
    case(
        &mut b,
        "set_encoding.err.opacity",
        &[],
        json!([{"op":"set_encoding","view":"main","layer":"l2","encoding":{"opacity":1.5}}]),
    );
    case(
        &mut b,
        "set_encoding.err.layer",
        &[],
        json!([{"op":"set_encoding","view":"main","layer":"l9","encoding":{"width":2}}]),
    );
    case(
        &mut b,
        "insert_panel.ok",
        &[],
        json!([{"op":"insert_panel","view":"main","after":0,"panel":{"name":"vol","y":{"kind":"linear"}},
        "layers":[{"mark":"bar","series":"px","encoding":{"y":"v"}}]}]),
    );
    case(
        &mut b,
        "insert_panel.err.after",
        &[],
        json!([{"op":"insert_panel","view":"main","after":9,"panel":{"y":{"kind":"linear"}}}]),
    );
    case(
        &mut b,
        "delete_panel.ok",
        &[],
        json!([{"op":"delete_panel","view":"main","panel":2}]),
    );
    case(
        &mut b,
        "delete_panel.err.last",
        &[],
        json!([{"op":"delete_panel","view":"main","panel":0},
        {"op":"delete_panel","view":"main","panel":1},{"op":"delete_panel","view":"main","panel":2}]),
    );
    case(
        &mut b,
        "delete_panel.err.missing",
        &[],
        json!([{"op":"delete_panel","view":"main","panel":5}]),
    );
    case(
        &mut b,
        "move_panel.ok",
        &[],
        json!([{"op":"move_panel","view":"main","panel":2,"before":0}]),
    );
    case(
        &mut b,
        "move_panel.err.after",
        &[],
        json!([{"op":"move_panel","view":"main","panel":2,"after":9}]),
    );
    case(
        &mut b,
        "set_panel.ok",
        &[],
        json!([{"op":"set_panel","view":"main","panel":1,"name":"rsi","weight":2}]),
    );
    case(
        &mut b,
        "set_panel.err.weight",
        &[],
        json!([{"op":"set_panel","view":"main","panel":1,"weight":0}]),
    );
    case(
        &mut b,
        "set_scale.ok.kind",
        &[],
        json!([{"op":"set_scale","view":"main","scale":"y0","kind":"log"}]),
    );
    case(
        &mut b,
        "set_scale.ok.domain",
        &[],
        json!([{"op":"set_scale","view":"main","scale":"y0","domain":[90,135]}]),
    );
    case(
        &mut b,
        "set_scale.ok.auto",
        &[],
        json!([{"op":"set_scale","view":"main","scale":"y1","auto_domain":true}]),
    );
    case(
        &mut b,
        "set_scale.err.kind",
        &[],
        json!([{"op":"set_scale","view":"main","scale":"x","kind":"log"}]),
    );
    case(
        &mut b,
        "set_scale.err.missing",
        &[],
        json!([{"op":"set_scale","view":"main","scale":"y9","kind":"log"}]),
    );
    case(
        &mut b,
        "set_emphasis.ok",
        &[],
        json!([{"op":"set_emphasis","view":"main","emphasis":[
        {"target":{"item":"fnd-overbought"},"kind":"highlight"},{"target":{"layer":"l2"},"kind":"focus"}]}]),
    );
    case(
        &mut b,
        "set_emphasis.ok.clear",
        &[],
        json!([{"op":"set_emphasis","view":"main","emphasis":[]}]),
    );
    case(
        &mut b,
        "set_emphasis.err.layer",
        &[],
        json!([{"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l9"},"kind":"dim"}]}]),
    );
    b.check();
}

fn intent(id: &str) -> Value {
    json!([{"op":"add_intent","id":id,"text":"t"}])
}

/// Scout proposes `ops`; the proposal's id.
fn propose(rig: &mut Rig, ops: Value) -> String {
    let out = rig.cmd("s", "pp", ops);
    out[0]["msg"]["proposal"]
        .as_str()
        .expect("proposed")
        .to_owned()
}

#[test]
fn charac_session_ops_undo_and_proposals() {
    let mut b = Book::new("ops-undo-proposals");
    // undo
    case(
        &mut b,
        "undo.ok.last",
        &[intent("i1")],
        json!([{"op":"undo"}]),
    );
    case(
        &mut b,
        "undo.ok.seq",
        &[intent("i1"), intent("i2")],
        json!([{"op":"undo","seq":1}]),
    );
    case(
        &mut b,
        "undo.ok.redo",
        &[intent("i1"), json!([{"op":"undo"}])],
        json!([{"op":"undo","seq":2}]),
    );
    case(
        &mut b,
        "undo.err.after_undo",
        &[intent("i1"), json!([{"op":"undo"}])],
        json!([{"op":"undo"}]),
    );
    case(&mut b, "undo.err.nothing", &[], json!([{"op":"undo"}]));
    case(
        &mut b,
        "undo.err.seq",
        &[intent("i1")],
        json!([{"op":"undo","seq":7}]),
    );
    case(
        &mut b,
        "undo.err.conflict",
        &[
            intent("i1"),
            json!([{"op":"set_intent_active","intent":"i1","active":false}]),
        ],
        json!([{"op":"undo","seq":1}]),
    );
    case_by(
        &mut b,
        "undo.err.other_author",
        &[intent("i1")],
        "e",
        json!([{"op":"undo"}]),
    );
    // proposals: scout proposes, arkin decides
    for (name, ops) in [
        ("propose.ok.intent", intent("pi")),
        (
            "propose.ok.study",
            json!([{"op":"add_study","study":"ema","inputs":["px"]}]),
        ),
        ("propose.err.invalid", json!([{"op":"remove","id":"nope"}])),
    ] {
        let mut rig = fresh();
        on(&mut b, name, &mut rig, &[], "s", ops);
    }
    {
        let mut rig = fresh();
        let pid = propose(&mut rig, intent("pi"));
        on(
            &mut b,
            "accept_proposal.ok",
            &mut rig,
            &[],
            "a",
            json!([{"op":"accept_proposal","proposal":pid,"note":"tamam"}]),
        );
    }
    {
        let mut rig = fresh();
        let pid = propose(&mut rig, intent("pi"));
        on(
            &mut b,
            "accept_proposal.err.not_alone",
            &mut rig,
            &[],
            "a",
            json!([{"op":"accept_proposal","proposal":pid},{"op":"add_intent","text":"x"}]),
        );
    }
    {
        let mut rig = fresh();
        propose(&mut rig, intent("pi"));
        on(
            &mut b,
            "accept_proposal.err.missing",
            &mut rig,
            &[],
            "a",
            json!([{"op":"accept_proposal","proposal":"prop-x"}]),
        );
    }
    {
        let mut rig = fresh();
        let pid = propose(&mut rig, intent("pi"));
        on(
            &mut b,
            "accept_proposal.err.proposer",
            &mut rig,
            &[],
            "s",
            json!([{"op":"accept_proposal","proposal":pid}]),
        );
    }
    {
        let mut rig = fresh();
        let pid = propose(&mut rig, intent("pi"));
        rig.cmd("a", "x", intent("pi"));
        on(
            &mut b,
            "accept_proposal.err.stale",
            &mut rig,
            &[],
            "a",
            json!([{"op":"accept_proposal","proposal":pid}]),
        );
    }
    {
        let mut rig = fresh();
        let pid = propose(&mut rig, intent("pi"));
        on(
            &mut b,
            "reject_proposal.ok",
            &mut rig,
            &[],
            "a",
            json!([{"op":"reject_proposal","proposal":pid,"note":"hayır"}]),
        );
        on(
            &mut b,
            "reject_proposal.err.decided",
            &mut rig,
            &[],
            "a",
            json!([{"op":"reject_proposal","proposal":pid}]),
        );
    }
    {
        let mut rig = fresh();
        on(
            &mut b,
            "reject_proposal.err.missing",
            &mut rig,
            &[],
            "a",
            json!([{"op":"reject_proposal","proposal":"nope"}]),
        );
    }
    // modes and bases
    {
        let mut rig = fresh();
        let before = rig.log_len();
        let out = rig.send(
            "a",
            &json!({"type":"command","id":"m","mode":"propose","ops":intent("mp")}),
        );
        record(&mut b, "mode.propose.owner", &rig, before, &out);
        let before = rig.log_len();
        let out = rig.send(
            "s",
            &json!({"type":"command","id":"m","mode":"direct","ops":intent("md")}),
        );
        record(&mut b, "mode.direct.proposer", &rig, before, &out);
        rig.cmd("a", "x", intent("i1"));
        let n = rig.log_len();
        let out = rig.send("a", &json!({"type":"command","id":"b","base":0,"ops":[{"op":"set_intent_active","intent":"i1","active":false}]}));
        record(&mut b, "base.conflict", &rig, n, &out);
        let out = rig.send(
            "a",
            &json!({"type":"command","id":"b","base":0,"ops":intent("i9")}),
        );
        record(&mut b, "base.disjoint", &rig, n, &out);
        let n = rig.log_len();
        let out = rig.send(
            "a",
            &json!({"type":"command","id":"r","reason":"neden","ops":intent("ir")}),
        );
        record(&mut b, "reason", &rig, n, &out);
    }
    b.check();
}

#[test]
fn charac_session_ops_people() {
    let mut b = Book::new("ops-people");
    // set_audience / set_sensitivity
    let aud = json!({"group":"future","sensitivity":"important","watchlist":["THYAO"],"positions":{"THYAO":100}});
    case(
        &mut b,
        "set_audience.ok.own",
        &[],
        json!([{"op":"set_audience","audience":aud}]),
    );
    case(
        &mut b,
        "set_audience.ok.other_by_owner",
        &[],
        json!([{"op":"set_audience","principal":"viewer","audience":aud}]),
    );
    case_by(
        &mut b,
        "set_audience.ok.viewer_own",
        &[],
        "v",
        json!([{"op":"set_audience","audience":aud}]),
    );
    case_by(
        &mut b,
        "set_audience.err.viewer_other",
        &[],
        "v",
        json!([{"op":"set_audience","principal":"arkin","audience":aud}]),
    );
    case(
        &mut b,
        "set_audience.err.watchlist",
        &[],
        json!([{"op":"set_audience","audience":{"group":"future","watchlist":[""]}}]),
    );
    case(
        &mut b,
        "set_audience.ok.unknown_principal",
        &[],
        json!([{"op":"set_audience","principal":"nobody","audience":aud}]),
    );
    case(
        &mut b,
        "set_sensitivity.ok",
        &[],
        json!([{"op":"set_sensitivity","sensitivity":"important"}]),
    );
    case(
        &mut b,
        "set_sensitivity.ok.clear",
        &[],
        json!([{"op":"set_sensitivity","principal":"viewer","sensitivity":null}]),
    );
    case_by(
        &mut b,
        "set_sensitivity.err.other",
        &[],
        "v",
        json!([{"op":"set_sensitivity","principal":"scout","sensitivity":"all"}]),
    );
    // add_comment
    case(
        &mut b,
        "add_comment.ok.capsule",
        &[],
        json!([{"op":"add_comment","about":{"capsule":"cap-overbought"},"lang":"tr",
        "text":"RSI yüksek","cites":["fnd-overbought"]}]),
    );
    case(
        &mut b,
        "add_comment.ok.by_finding",
        &[],
        json!([{"op":"add_comment","about":{"capsule":"fnd-overbought"},"lang":"en",
        "text":"high","cites":["fnd-overbought"]}]),
    );
    case(
        &mut b,
        "add_comment.ok.instrument",
        &[],
        json!([{"op":"add_comment","id":"k1","about":{"instrument":"THYAO"},"lang":"tr",
        "text":"t","cites":["fact-rsi-cross70"]}]),
    );
    case(
        &mut b,
        "add_comment.err.no_cites",
        &[],
        json!([{"op":"add_comment","about":{"instrument":"THYAO"},"lang":"tr","text":"t","cites":[]}]),
    );
    case(
        &mut b,
        "add_comment.err.both",
        &[],
        json!([{"op":"add_comment","about":{"capsule":"cap-overbought","instrument":"THYAO"},
        "lang":"tr","text":"t","cites":[]}]),
    );
    case(
        &mut b,
        "add_comment.err.capsule",
        &[],
        json!([{"op":"add_comment","about":{"capsule":"cap-overbougt"},"lang":"tr","text":"t","cites":[]}]),
    );
    case(
        &mut b,
        "add_comment.err.instrument",
        &[],
        json!([{"op":"add_comment","about":{"instrument":"GARAN"},"lang":"tr","text":"t","cites":[]}]),
    );
    case(
        &mut b,
        "add_comment.err.cite",
        &[],
        json!([{"op":"add_comment","about":{"capsule":"cap-overbought"},"lang":"tr","text":"t","cites":["nope"]}]),
    );
    // role checks in front of expansion
    case_by(&mut b, "role.viewer.write", &[], "v", intent("iv"));
    case_by(&mut b, "role.editor.write", &[], "e", intent("ie"));
    case_by(
        &mut b,
        "role.proposer.approve",
        &[],
        "s",
        json!([{"op":"approve_finding","finding":"fnd-overbought"}]),
    );
    case_by(
        &mut b,
        "role.editor.approve",
        &[],
        "e",
        json!([{"op":"approve_finding","finding":"fnd-overbought"}]),
    );
    b.check();
}

/// Transcript 69's construction: ana (viewer) with agent bot (proposer, on her behalf,
/// policy `r5b`), eve (viewer).
fn watch_rig() -> Rig {
    let mut r = Rig::of("69-watch");
    r.hello("a", "ana", "headless");
    r.hello("b", "bot", "headless");
    r.hello("e", "eve", "headless");
    for c in ["a", "b", "e"] {
        r.sub(c, "w", json!({"kind":"intents"}));
        r.sub(c, "ch", json!({"kind":"changes"}));
    }
    r
}

#[test]
fn charac_session_ops_watch() {
    let mut b = Book::new("ops-watch");
    let w = |extra: Value| {
        let mut o = json!({"op":"add_watch","text":"THYAO","match":{"instrument":"THYAO"}});
        for (k, v) in extra.as_object().unwrap() {
            o[k] = v.clone();
        }
        json!([o])
    };
    let mut cases: Vec<(&str, &str, Value)> = vec![
        ("add_watch.ok.agent", "b", w(json!({}))),
        ("add_watch.ok.person", "a", w(json!({"max_matches":2}))),
        (
            "add_watch.ok.for_self",
            "b",
            w(json!({"for":"ana","expires_t":1_800_000_000_000_i64})),
        ),
        ("add_watch.err.for_other", "b", w(json!({"for":"eve"}))),
        (
            "add_watch.ok.unknown_instrument",
            "b",
            json!([{"op":"add_watch","text":"x","match":{"instrument":"GARAN"}}]),
        ),
        ("add_watch.err.max", "b", w(json!({"max_matches":0}))),
    ];
    for (name, conn, ops) in cases.drain(..) {
        let mut rig = watch_rig();
        on(&mut b, name, &mut rig, &[], conn, ops);
    }
    let made = |rig: &mut Rig| -> String {
        let out = rig.cmd(
            "b",
            "mk",
            json!([{"op":"add_watch","text":"THYAO","match":{"instrument":"THYAO"}}]),
        );
        out[0]["msg"]["created"][0].as_str().unwrap().to_owned()
    };
    for (name, conn, ops) in [
        ("remove_watch.ok.agent", "b", "own"),
        ("remove_watch.ok.person", "a", "own"),
        ("remove_watch.err.other", "e", "own"),
        ("remove_watch.err.missing", "b", "watch-9-9"),
        ("set_intent_active.err.watch", "b", "active"),
        ("remove.err.watch", "b", "remove"),
        ("remove.err.watch_other", "e", "remove"),
    ] {
        let mut rig = watch_rig();
        let id = made(&mut rig);
        let ops = match ops {
            "own" => json!([{"op":"remove_watch","intent":id}]),
            "active" => json!([{"op":"set_intent_active","intent":id,"active":false}]),
            "remove" => json!([{"op":"remove","id":id}]),
            other => json!([{"op":"remove_watch","intent":other}]),
        };
        on(&mut b, name, &mut rig, &[], conn, ops);
    }
    b.check();
}
