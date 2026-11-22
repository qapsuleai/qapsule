//! (R5B, M28 c) The `query` surface: one read tool (`ask`) with a small query language and
//! one write tool (`act`), over a synthetic session with a capsule engine and a licensed
//! flow (transcript 39). Every noun and every action answers; no decision is reachable;
//! an agent working for an unentitled person never receives a licensed number; the three
//! formats carry the same numbers.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qp_mcp::surfaces;
use qp_mcp::{Config, Hub, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::data::DataEvent;
use qp_schema::protocol::{InitRef, Surface, Transcript};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Transcript 39's session after its data steps, with two agents: `scout` for `ana`
/// (entitled to everything) and `gast` for `can` (entitled to nothing). `ana` holds THYAO.
fn hub() -> SharedHub {
    let r = root();
    let t: Transcript =
        qp_session::read_json(&r.join("fixtures/protocol/conformance/39-capsule-entitlement.json"))
            .unwrap();
    let InitRef::Inline(i) = t.init.clone() else {
        panic!("inline init")
    };
    let mut i = *i;
    for (a, p) in [("scout", "ana"), ("gast", "can")] {
        i.principals.push(
            serde_json::from_value(json!({"id": a, "author": {"kind": "agent", "name": a},
                "role": "proposer", "on_behalf": p}))
            .unwrap(),
        );
    }
    let h = Hub::shared(qp_session::Session::new(i, &r).unwrap());
    for step in &t.steps {
        if let Some(d) = &step.data {
            let ev: DataEvent = serde_json::from_value(serde_json::to_value(d).unwrap()).unwrap();
            h.borrow_mut().session_mut().data_event(&ev).unwrap();
        }
    }
    // The people's own (unmarked) connections.
    for (c, p) in [("ana-desk", "ana"), ("can-desk", "can")] {
        let hello =
            json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"desktop"}});
        let r = h.borrow_mut().send(c, &hello.to_string());
        assert_eq!(r[0]["type"], "welcome", "{r:?}");
    }
    let aud = json!({"type":"command","id":"aud","ops":[{"op":"set_audience","audience":
        {"group":"habituated","positions":{"THYAO":100}}}]});
    let r = h.borrow_mut().send("ana-desk", &aud.to_string());
    assert_eq!(r[0]["type"], "ack", "{r:?}");
    let market: qp_schema::market::MarketStatus = serde_json::from_value(json!({
        "exchange_time": 1683788400000_u64, "open": false, "holiday": false, "source": "calendar",
        "delayed": true, "stale": false, "phases": {"THYAO": {"code": 1, "key": "closed"}}}))
    .unwrap();
    h.borrow_mut().session_mut().set_market_status(market);
    h
}

fn served(h: &SharedHub, conn: &str, principal: &str) -> McpServer {
    McpServer::open(
        h.clone(),
        conn,
        Config::new(
            principal,
            serde_json::from_value::<Surface>(json!({"kind": "headless"})).unwrap(),
        ),
        surfaces::by_name("query").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
            capsule_format: None,
        },
    )
    .unwrap()
}

fn rpc(s: &mut McpServer, id: u64, method: &str, mut params: Value) -> Value {
    params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
    let out = s.handle_message(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
    out.into_iter()
        .rev()
        .find(|m| m.get("id") == Some(&json!(id)))
        .expect("a response")
}

/// A tool call's `result` (the MCP `CallToolResult`).
fn call(s: &mut McpServer, name: &str, args: Value) -> Value {
    let r = rpc(s, 7, "tools/call", json!({"name": name, "arguments": args}));
    r.get("result")
        .cloned()
        .unwrap_or_else(|| panic!("no result: {r}"))
}

fn ok(s: &mut McpServer, name: &str, args: Value) -> Value {
    let r = call(s, name, args.clone());
    assert_eq!(r["isError"], false, "{name} {args}: {r}");
    r
}

fn text(r: &Value) -> String {
    r["content"][0]["text"].as_str().unwrap_or("").to_owned()
}

/// Numeric tokens of a text: digit runs with inner `.`/`,` (as written).
fn tokens(s: &str) -> BTreeSet<String> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_ascii_digit() {
            let mut j = i;
            while j < cs.len()
                && (cs[j].is_ascii_digit()
                    || ((cs[j] == '.' || cs[j] == ',')
                        && j + 1 < cs.len()
                        && cs[j + 1].is_ascii_digit()))
            {
                j += 1;
            }
            out.insert(cs[i..j].iter().collect());
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// One `ask` per noun (with filters that apply to it).
fn every_find() -> Vec<Value> {
    vec![
        json!({"find": "capsules"}),
        json!({"find": "capsules", "where": {"instrument": "THYAO", "open": false, "direction": "down"}, "limit": 3}),
        json!({"find": "stories", "where": {"instrument": "thyao"}, "limit": 5}),
        json!({"find": "facts", "where": {"instrument": "THYAO", "since_bar": 80}}),
        json!({"find": "instrument", "where": {"instrument": "THYAO"}}),
        json!({"find": "ledger", "where": {"by_kind": true}}),
        json!({"find": "market"}),
        json!({"find": "glossary", "where": {"instrument": "THYAO"}}),
        json!({"find": "changes", "where": {"after_seq": 2, "since_bar": 85}}),
        json!({"find": "comments"}),
        json!({"find": "watches"}),
        json!({"find": "proposals", "where": {"state": "pending"}}),
    ]
}

#[test]
fn tools_list_is_two_tools_plus_common_and_no_decision() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    let list = rpc(&mut a, 1, "tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["ask", "act", "render", "notifications"]);
    for t in tools {
        assert!(t["inputSchema"].get("$schema").is_none());
        assert_eq!(t["inputSchema"]["type"], "object");
        let d = t.to_string();
        for op in [
            "accept_proposal",
            "reject_proposal",
            "approve_finding",
            "reject_finding",
        ] {
            assert!(!d.contains(op), "{op} in {}", t["name"]);
        }
    }
    for t in tools {
        println!("{} {} bytes", t["name"], t.to_string().len());
    }
    if std::env::var("QP_DUMP_TOOLS").is_ok() {
        println!("{}", serde_json::to_string_pretty(&tools[..2]).unwrap());
    }
    let own: usize = tools[..2].iter().map(|t| t.to_string().len()).sum();
    let all: usize = tools.iter().map(|t| t.to_string().len()).sum();
    println!("tool definitions: ask+act {own} bytes, with common tools {all} bytes");
    assert!(own < 16_000, "{own}");
}

#[test]
fn every_noun_answers_with_ids() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    for q in every_find() {
        for f in ["card", "json", "layered"] {
            let mut q = q.clone();
            q["format"] = json!(f);
            let r = ok(&mut a, "ask", q.clone());
            assert!(!text(&r).is_empty(), "{q}");
            assert!(r["structuredContent"]["ids"].is_array(), "{q}: {r}");
        }
    }
    // Capsules carry capsule, finding and fact ids.
    let r = ok(&mut a, "ask", json!({"find": "capsules", "limit": 3}));
    let ids = r["structuredContent"]["ids"].to_string();
    for p in ["cap.", "fnd.", "fx."] {
        assert!(ids.contains(p), "{p}: {ids}");
    }
    // Holdings: what the authority marks `relevant` (ana holds THYAO). Today the
    // authority marks it for the person, not for an agent on their behalf: the agent
    // gets an empty answer that says so.
    let mut person = served(&h, "mcp-9", "ana");
    let r = ok(
        &mut person,
        "ask",
        json!({"find": "capsules", "where": {"holdings": true}, "limit": 1}),
    );
    assert_eq!(r["structuredContent"]["count"], 1, "{r}");
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "capsules", "where": {"holdings": true}}),
    );
    assert!(text(&r).contains("relevant"), "{r}");
    // The market phase is the projected one.
    let r = ok(&mut a, "ask", json!({"find": "market", "format": "json"}));
    assert_eq!(
        r["structuredContent"]["items"][0]["phases"]["THYAO"]["key"], "closed",
        "{r}"
    );
    // The glossary has terms.
    let r = ok(&mut a, "ask", json!({"find": "glossary", "format": "json"}));
    assert!(r["structuredContent"]["count"].as_u64().unwrap() > 0, "{r}");
    // The glossary read left the connection's voice at the default.
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "capsules", "format": "json", "limit": 1}),
    );
    assert!(
        r["structuredContent"]["items"][0].get("voice").is_none(),
        "{r}"
    );
}

#[test]
fn reference_by_meaning_and_expand() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "capsules", "where": {"capsule": {"instrument": "THYAO", "meaning": "flow", "latest": true}},
               "expand": ["story", "evidence", "ledger", "comments"], "format": "json"}),
    );
    let it = &r["structuredContent"]["items"][0];
    assert!(
        it["id"].as_str().unwrap().starts_with("cap.THYAO.flow."),
        "{r}"
    );
    for k in ["story", "evidence", "ledger", "comments"] {
        assert!(it.get(k).is_some(), "{k}: {r}");
    }
    assert_eq!(it["evidence"][0]["open"], true, "{r}");
    // Stories expand too.
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "stories", "where": {"instrument": "THYAO", "meaning": "flow"}, "limit": 2, "expand": ["story"]}),
    );
    assert!(text(&r).contains("story: bar"), "{}", text(&r));
}

#[test]
fn every_action_and_no_decision() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    // comment (by meaning): accepted, then readable.
    let r = ok(
        &mut a,
        "act",
        json!({"do": "comment", "capsule": {"instrument": "THYAO", "meaning": "breakout", "direction": "down", "first": true},
               "text": "THYAO'da aşağı kırılım hikâyesi kapandı [1].", "cites": [{"instrument": "THYAO", "meaning": "breakout", "direction": "down", "first": true}]}),
    );
    assert_eq!(r["structuredContent"]["status"], "applied", "{r}");
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "comments", "where": {"instrument": "THYAO"}, "format": "json"}),
    );
    assert_eq!(r["structuredContent"]["count"], 1, "{r}");
    // A comment with an invented number: rejected with reasons.
    let r = call(
        &mut a,
        "act",
        json!({"do": "comment", "instrument": "THYAO", "text": "THYAO %37 yükseldi.", "cites": ["cap.THYAO.breakout.down.30"]}),
    );
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(
        r["structuredContent"]["error"]["code"], "comment_rejected",
        "{r}"
    );
    assert!(r["structuredContent"]["error"]["reasons"].is_array(), "{r}");
    // watch, watches, unwatch.
    let r = ok(
        &mut a,
        "act",
        json!({"do": "watch", "text": "THYAO'da olumsuz bir şey olursa", "match": {"instrument": "THYAO", "negative": true}}),
    );
    let wid = r["structuredContent"]["created"][0]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "watches", "where": {"state": "active"}, "format": "json"}),
    );
    assert_eq!(
        r["structuredContent"]["items"][0]["id"],
        wid.as_str(),
        "{r}"
    );
    ok(&mut a, "act", json!({"do": "unwatch", "watch": wid}));
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "watches", "where": {"state": "active"}}),
    );
    assert_eq!(r["structuredContent"]["count"], 0, "{r}");
    // propose → pending; preview writes nothing.
    let r = ok(
        &mut a,
        "act",
        json!({"do": "propose", "ops": [{"op": "add_study", "id": "rsi7", "study": "rsi",
               "inputs": ["px"], "params": {"period": 7}}], "reason": "short RSI"}),
    );
    assert_eq!(r["structuredContent"]["status"], "proposed", "{r}");
    let pid = r["structuredContent"]["proposal"]
        .as_str()
        .unwrap()
        .to_owned();
    let seq = r["structuredContent"]["seq"].clone();
    let r = ok(
        &mut a,
        "act",
        json!({"do": "preview", "proposal": {"state": "pending", "latest": true}}),
    );
    assert_eq!(r["structuredContent"]["proposal"], pid.as_str(), "{r}");
    assert_eq!(r["structuredContent"]["seq"], seq, "preview wrote: {r}");
    // No decision, by any spelling or path.
    for d in [
        "accept",
        "reject",
        "approve",
        "accept_proposal",
        "reject_proposal",
        "approve_finding",
        "reject_finding",
        "decide",
    ] {
        let r = call(&mut a, "act", json!({"do": d, "proposal": pid}));
        assert_eq!(r["isError"], true, "{d}: {r}");
        assert_eq!(
            r["structuredContent"]["error"]["reason"], "human_only",
            "{d}: {r}"
        );
    }
    for op in ["accept_proposal", "reject_proposal"] {
        let r = call(
            &mut a,
            "act",
            json!({"do": "propose", "ops": [{"op": op, "proposal": pid}]}),
        );
        assert_eq!(
            r["structuredContent"]["error"]["reason"], "human_only",
            "{r}"
        );
    }
    let r = call(
        &mut a,
        "act",
        json!({"do": "propose", "ops": [{"op": "approve_finding", "finding": "fnd.THYAO.breakout.down.30"}]}),
    );
    assert_eq!(
        r["structuredContent"]["error"]["reason"], "human_only",
        "{r}"
    );
    // Unknown tools stay unknown.
    let r = rpc(
        &mut a,
        9,
        "tools/call",
        json!({"name": "op_accept_proposal", "arguments": {"proposal": pid}}),
    );
    assert_eq!(r["error"]["code"], -32602, "{r}");
    // Still pending, for the person to decide.
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "proposals", "where": {"state": "pending"}, "format": "json"}),
    );
    assert_eq!(
        r["structuredContent"]["items"][0]["id"],
        pid.as_str(),
        "{r}"
    );
    assert_eq!(
        r["structuredContent"]["items"][0]["state"], "pending",
        "{r}"
    );
}

/// The licensed numbers of the session: the flow evidence values an entitled reader sees.
fn licensed_values(h: &SharedHub) -> Vec<f64> {
    let q = json!({"type":"query","id":"lf","query":{"kind":"list","collection":"facts"}});
    let r = h.borrow_mut().send("ana-desk", &q.to_string());
    let mut out = Vec::new();
    for f in r[0]["data"]["items"].as_array().unwrap() {
        if f["meaning"]["license"]["class"] == "licensed" {
            for v in f["evidence"].as_object().unwrap().values() {
                out.push(v.as_f64().unwrap());
            }
        }
    }
    assert!(out.len() > 10, "{out:?}");
    out
}

#[test]
fn an_agent_for_an_unentitled_person_never_gets_a_licensed_number() {
    let h = hub();
    let lic = licensed_values(&h);
    let mut forms: BTreeSet<String> = BTreeSet::new();
    for v in &lic {
        forms.insert(format!("{v}"));
        if v.abs() >= 1000.0 {
            let i = format!("{:.0}", v.abs());
            forms.insert(i.clone());
            // Thousands grouping as a sentence writes it (TR `.`, EN `,`).
            let g: Vec<String> = i
                .as_bytes()
                .rchunks(3)
                .rev()
                .map(|c| String::from_utf8(c.to_vec()).unwrap())
                .collect();
            forms.insert(g.join("."));
            forms.insert(g.join(","));
        }
    }
    let mut g = served(&h, "mcp-2", "gast");
    // The entitled agent does see them (the check is not vacuous).
    let mut a = served(&h, "mcp-1", "scout");
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "facts", "where": {"meaning": "flow"}, "format": "json", "limit": 100}),
    );
    assert!(
        tokens(&text(&r)).iter().any(|t| forms.contains(t)),
        "entitled reader sees no licensed value"
    );
    let mut asks = every_find();
    asks.push(json!({"find": "capsules", "expand": ["story", "evidence", "ledger", "comments"], "limit": 100}));
    asks.push(json!({"find": "capsules", "where": {"meaning": "flow", "latest": true}, "expand": ["evidence"]}));
    asks.push(json!({"find": "capsules", "where": {"capsule": {"instrument": "THYAO", "meaning": "flow", "latest": true}}, "expand": ["evidence"]}));
    asks.push(json!({"find": "capsules", "where": {"id": "cap.THYAO.flow.up.90"}}));
    asks.push(json!({"find": "facts", "limit": 100}));
    asks.push(json!({"find": "stories", "where": {"instrument": "THYAO"}, "expand": ["evidence"], "limit": 100}));
    asks.push(json!({"find": "changes", "where": {"after_seq": 0}, "limit": 100}));
    let mut seen = 0;
    for q in asks {
        for f in ["card", "json", "layered"] {
            let mut q = q.clone();
            q["format"] = json!(f);
            let r = call(&mut g, "ask", q.clone());
            let body = r.to_string();
            for t in tokens(&text(&r)).iter().chain(tokens(&body).iter()) {
                assert!(!forms.contains(t), "licensed `{t}` in {q}: {body}");
            }
            seen += 1;
        }
    }
    // A comment carrying a licensed number is refused for this reader.
    let v = format!("{:.0}", lic[0].abs());
    let r = call(
        &mut g,
        "act",
        json!({"do": "comment", "instrument": "THYAO", "text": format!("THYAO kurumsal net {v}."), "cites": ["cap.THYAO.breakout.down.30"]}),
    );
    assert_eq!(r["isError"], true, "{r}");
    assert!(seen > 50);
}

#[test]
fn the_three_formats_carry_the_same_numbers() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    for q in [
        json!({"find": "capsules", "expand": ["story", "evidence", "ledger"], "limit": 4}),
        json!({"find": "capsules", "where": {"meaning": "flow"}, "limit": 3}),
        json!({"find": "stories", "where": {"instrument": "THYAO"}, "limit": 6}),
        json!({"find": "facts", "where": {"since_bar": 85}}),
        json!({"find": "instrument"}),
        json!({"find": "ledger"}),
    ] {
        let mut by = Vec::new();
        for f in ["card", "json", "layered"] {
            let mut q = q.clone();
            q["format"] = json!(f);
            let r = ok(&mut a, "ask", q);
            let sc = &r["structuredContent"];
            by.push((tokens(&text(&r)), sc["ids"].clone(), sc["count"].clone()));
        }
        let (card, json_, layered) = (&by[0], &by[1], &by[2]);
        // Same items and ids.
        assert_eq!(card.1, json_.1, "{q}");
        assert_eq!(layered.1, json_.1, "{q}");
        assert_eq!(card.2, json_.2, "{q}");
        // Every number the card or the summary writes is one the protocol object has.
        for (name, set) in [("card", &card.0), ("layered", &layered.0)] {
            let extra: Vec<&String> = set.difference(&json_.0).collect();
            assert!(
                extra.is_empty(),
                "{name} numbers not in json {extra:?} for {q}"
            );
        }
    }
}

#[test]
fn errors_say_what_was_expected() {
    let h = hub();
    let mut a = served(&h, "mcp-1", "scout");
    let r = call(&mut a, "ask", json!({"find": "capsuls"}));
    assert_eq!(r["isError"], true);
    assert_eq!(
        r["structuredContent"]["error"]["did_you_mean"][0], "capsules",
        "{r}"
    );
    let r = call(
        &mut a,
        "ask",
        json!({"find": "capsules", "where": {"instrumnt": "THYAO"}}),
    );
    assert_eq!(
        r["structuredContent"]["error"]["did_you_mean"][0], "instrument",
        "{r}"
    );
    assert!(
        r["structuredContent"]["error"]["expected"].is_array(),
        "{r}"
    );
    let r = call(
        &mut a,
        "ask",
        json!({"find": "ledger", "where": {"instrument": "THYAO"}}),
    );
    assert_eq!(
        r["structuredContent"]["error"]["path"], "/where/instrument",
        "{r}"
    );
    let r = call(
        &mut a,
        "ask",
        json!({"find": "capsules", "where": {"instrument": "THYA0"}}),
    );
    assert_eq!(r["structuredContent"]["error"]["code"], "not_found", "{r}");
    assert_eq!(
        r["structuredContent"]["error"]["did_you_mean"][0], "THYAO",
        "{r}"
    );
    let r = call(
        &mut a,
        "ask",
        json!({"find": "capsules", "select": ["stag"]}),
    );
    assert_eq!(
        r["structuredContent"]["error"]["did_you_mean"][0], "stage",
        "{r}"
    );
    let r = call(&mut a, "ask", json!({"find": "stories"}));
    assert_eq!(
        r["structuredContent"]["error"]["path"], "/where/instrument",
        "{r}"
    );
    let r = call(&mut a, "act", json!({"do": "coment"}));
    assert_eq!(
        r["structuredContent"]["error"]["did_you_mean"][0], "comment",
        "{r}"
    );
    // Aliases are accepted and answered under the canonical noun.
    let r = ok(&mut a, "ask", json!({"find": "capsule", "limit": 1}));
    assert_eq!(r["structuredContent"]["find"], "capsules", "{r}");
    // Select keeps fields.
    let r = ok(
        &mut a,
        "ask",
        json!({"find": "capsules", "select": ["stage"], "format": "json", "limit": 2}),
    );
    let it = r["structuredContent"]["items"][0].as_object().unwrap();
    let keys: Vec<&str> = it.keys().map(String::as_str).collect();
    assert_eq!(keys, ["id", "refs", "stage"], "{r}");
}
