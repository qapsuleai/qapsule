//! (R5B, M28 b) The task surface (`--surface task`): every tool answers on a synthetic
//! replay session, no decision tool exists, an agent working for an unentitled person
//! gets no licensed number from any tool, and the three answer formats carry the same
//! numbers.

use std::collections::BTreeSet;
use std::path::Path;

use qp_mcp::surfaces::{self, thin::HUMAN_DECISION_TOOLS};
use qp_mcp::{Config, Hub, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::protocol::{Principal, ReplayControl, ReplayInit, Surface};
use serde_json::{json, Value};

const TASK_TOOLS: [&str; 9] = [
    "whats_new",
    "find",
    "explain",
    "instrument",
    "track_record",
    "watch",
    "comment",
    "propose",
    "glossary",
];

/// A synthetic replay: 3 instruments; `arkin` (and `claude`, working for him) entitled
/// to every feed, `guest` (and `claude-guest`) to none.
fn hub() -> SharedHub {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let all = || vec!["*".to_owned()];
    let mut init = qp_session::demo_init(ReplayInit {
        instruments: Some(3),
        days: Some(80),
        history: Some(60),
        entitlements: [
            ("host".to_owned(), all()),
            ("arkin".to_owned(), all()),
            ("claude".to_owned(), all()),
            ("scout".to_owned(), Vec::new()),
            ("guest".to_owned(), Vec::new()),
            ("claude-guest".to_owned(), Vec::new()),
        ]
        .into(),
        ..ReplayInit::default()
    });
    for (id, person) in [("claude", "arkin"), ("claude-guest", "guest")] {
        let p: Principal = serde_json::from_value(json!({"id": id,
            "author": {"kind": "agent", "name": id}, "role": "proposer", "on_behalf": person}))
        .unwrap();
        init.principals.push(p);
    }
    let h = Hub::shared_replay(
        qp_session::ReplayHost::with_local_dir(init, &root, Path::new("")).unwrap(),
    );
    step(&h, 10);
    h
}

fn step(h: &SharedHub, n: u32) {
    h.borrow_mut()
        .host_control(&ReplayControl::Step { n: Some(n) })
        .unwrap();
}

/// An agent's endpoint as `qp mcp serve --surface task` opens it.
fn served(h: &SharedHub, conn: &str, principal: &str) -> McpServer {
    McpServer::open(
        h.clone(),
        conn,
        Config::new(
            principal,
            serde_json::from_value::<Surface>(json!({"kind": "headless"})).unwrap(),
        ),
        surfaces::by_name("task").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
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

/// `tools/call`; the `CallToolResult` (panics on a JSON-RPC error).
fn call(s: &mut McpServer, name: &str, args: Value) -> Value {
    let r = rpc(s, 7, "tools/call", json!({"name": name, "arguments": args}));
    assert!(r.get("error").is_none(), "{name} {r}");
    r["result"].clone()
}

fn text(r: &Value) -> String {
    r["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every number token (runs of digits, `.` and `,`; `,` read as the decimal mark) of a
/// text.
fn nums(t: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for ch in t.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            cur.push(ch);
        } else if !cur.is_empty() {
            let tok = cur.trim_matches(|c| c == '.' || c == ',').replace(',', ".");
            if tok.chars().any(|c| c.is_ascii_digit()) {
                out.insert(tok);
            }
            cur.clear();
        }
    }
    out
}

/// The numbers of the answer as a whole (text and structured content).
fn all_nums(r: &Value) -> BTreeSet<String> {
    let mut n = nums(&text(r));
    n.extend(nums(&r["structuredContent"].to_string()));
    n
}

#[test]
fn the_task_surface_lists_nine_task_tools_and_no_decision_tool() {
    let h = hub();
    let mut s = served(&h, "mcp-1", "claude");
    assert_eq!(s.surface_name(), "task");
    let list = rpc(&mut s, 1, "tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let mut want: Vec<&str> = TASK_TOOLS.to_vec();
    want.extend(["render", "notifications"]);
    assert_eq!(names, want);
    for t in tools {
        let name = t["name"].as_str().unwrap();
        for word in ["accept", "reject", "approve", "decide"] {
            assert!(!name.contains(word), "{name}");
        }
        assert!(!HUMAN_DECISION_TOOLS.contains(&name));
        assert_eq!(t["inputSchema"]["type"], "object", "{name}");
        assert_eq!(t["annotations"]["destructiveHint"], false, "{name}");
        assert!(!t["description"].as_str().unwrap().is_empty());
    }
    let bytes = serde_json::to_string(&list["result"]["tools"])
        .unwrap()
        .len();
    eprintln!(
        "task surface tools/list: {} tools, {bytes} bytes",
        tools.len()
    );
    // The thin surface's decision tools are unknown here.
    for t in HUMAN_DECISION_TOOLS {
        let r = rpc(
            &mut s,
            9,
            "tools/call",
            json!({"name": t, "arguments": {"proposal": "prop-1"}}),
        );
        assert_eq!(r["error"]["code"], -32602, "{t}: {r}");
    }
    // The instructions are short (5–8 sentences).
    let init = rpc(
        &mut s,
        2,
        "initialize",
        json!({"protocolVersion": "2025-11-25"}),
    );
    let ins = init["result"]["instructions"].as_str().unwrap();
    let sentences = ins.matches(". ").count() + 1;
    assert!((5..=8).contains(&sentences), "{sentences}: {ins}");
}

#[test]
fn a_decision_cannot_be_proposed_either() {
    let h = hub();
    let mut s = served(&h, "mcp-1", "claude");
    let r = call(
        &mut s,
        "propose",
        json!({"ops": [{"op": "accept_proposal", "proposal": "prop-1"}]}),
    );
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(r["structuredContent"]["error"]["code"], "forbidden", "{r}");
    assert_eq!(
        r["structuredContent"]["error"]["reason"], "human_only",
        "{r}"
    );
}

#[test]
fn every_tool_answers() {
    let h = hub();
    let mut s = served(&h, "mcp-1", "claude");

    // whats_new: first look, then the moves after a few more bars.
    let w1 = call(&mut s, "whats_new", json!({}));
    assert_eq!(w1["isError"], false, "{w1}");
    assert!(text(&w1).contains("first look"), "{}", text(&w1));
    assert!(!w1["structuredContent"]["ids"]
        .as_array()
        .unwrap()
        .is_empty());
    let w0 = call(&mut s, "whats_new", json!({}));
    assert!(text(&w0).contains("No capsule moved"), "{}", text(&w0));
    step(&h, 8);
    let w2 = call(&mut s, "whats_new", json!({}));
    assert!(text(&w2).contains("since your last look"), "{}", text(&w2));
    assert!(
        !w2["structuredContent"]["ids"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{w2}"
    );

    // find: filters and paging.
    let f = call(&mut s, "find", json!({"limit": 2}));
    assert_eq!(f["isError"], false, "{f}");
    let ids = f["structuredContent"]["ids"].as_array().unwrap().clone();
    assert_eq!(ids.len(), 2);
    let next = f["structuredContent"]["next"].as_str().unwrap().to_owned();
    let f2 = call(&mut s, "find", json!({"limit": 2, "after": next}));
    assert_ne!(f2["structuredContent"]["ids"][0], ids[0]);
    let fb = call(
        &mut s,
        "find",
        json!({"meaning": "breakout", "direction": "up", "latest": true}),
    );
    let fb_ids = fb["structuredContent"]["ids"].as_array().unwrap();
    assert!(fb_ids.len() <= 1, "{fb}");
    for id in fb_ids {
        assert!(id.as_str().unwrap().contains(".breakout.up."), "{fb}");
    }
    let bad = call(&mut s, "find", json!({"after": "cap.nope"}));
    assert_eq!(bad["isError"], true);

    // explain: by id and by meaning (selector).
    let id = ids[0].as_str().unwrap().to_owned();
    let e = call(&mut s, "explain", json!({"capsule": id}));
    assert_eq!(e["isError"], false, "{e}");
    let et = text(&e);
    assert!(et.contains("evidence:") && et.contains("fx."), "{et}");
    assert!(et.contains("track record"), "{et}");
    let inst = id.split('.').nth(1).unwrap().to_owned();
    let sel = call(
        &mut s,
        "explain",
        json!({"capsule": {"instrument": inst, "latest": true}}),
    );
    assert_eq!(sel["isError"], false, "{sel}");
    assert!(sel["structuredContent"]["resolved"].is_object(), "{sel}");
    let nf = call(&mut s, "explain", json!({"capsule": "cap.nope"}));
    assert_eq!(nf["isError"], true);
    assert!(text(&nf).contains("not_found"), "{}", text(&nf));

    // instrument: the list, then one instrument's stories and the clock.
    let l = call(&mut s, "instrument", json!({}));
    assert!(text(&l).contains(&inst), "{}", text(&l));
    let i = call(&mut s, "instrument", json!({"symbol": inst, "limit": 3}));
    assert_eq!(i["isError"], false, "{i}");
    assert!(text(&i).contains("clock: replay"), "{}", text(&i));

    // track_record.
    let t = call(&mut s, "track_record", json!({"by_kind": true}));
    assert_eq!(t["isError"], false, "{t}");
    assert!(text(&t).contains("realized"), "{}", text(&t));

    // watch: add, list, remove.
    let a = call(
        &mut s,
        "watch",
        json!({"text": "tell me if it weakens", "match": {"instrument": inst, "negative": true}}),
    );
    assert_eq!(a["isError"], false, "{a}");
    let wid = a["structuredContent"]["created"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let li = call(&mut s, "watch", json!({"action": "list"}));
    assert!(text(&li).contains(&wid), "{}", text(&li));
    let rm = call(&mut s, "watch", json!({"action": "remove", "intent": wid}));
    assert_eq!(rm["isError"], false, "{rm}");
    let again = call(&mut s, "watch", json!({"action": "remove", "intent": wid}));
    assert_eq!(again["isError"], true, "{again}");

    // comment: a made-up number is rejected with machine-readable reasons; a plain
    // observation citing the capsule is stored and shows on `explain`.
    let rej = call(
        &mut s,
        "comment",
        json!({"capsule": id, "lang": "en", "text": "It moved 99.5 percent [1].", "cites": [id]}),
    );
    assert_eq!(rej["isError"], true, "{rej}");
    assert_eq!(
        rej["structuredContent"]["error"]["code"],
        "comment_rejected"
    );
    assert!(text(&rej).contains("unsupported_number"), "{}", text(&rej));
    assert!(text(&rej).contains("hint"), "{}", text(&rej));
    let ok = call(
        &mut s,
        "comment",
        json!({"capsule": id, "lang": "en", "text": "The engine reports this story [1].", "cites": [id]}),
    );
    assert_eq!(ok["isError"], false, "{ok}");
    let cid = ok["structuredContent"]["created"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let e2 = call(&mut s, "explain", json!({"capsule": id}));
    assert!(text(&e2).contains(&cid), "{}", text(&e2));

    // propose: a new view, pending, with its preview.
    let p = call(
        &mut s,
        "propose",
        json!({"ops": [{"op": "put_view", "view": {"id": "v1", "x": {"kind": "time"},
            "panels": [{"id": 0, "y": {"kind": "linear"}}],
            "layers": [{"id": "c", "mark": "candle", "series": inst.to_lowercase(), "panel": 0}]}}],
            "reason": "a price view"}),
    );
    assert_eq!(p["isError"], false, "{p}");
    assert_eq!(p["structuredContent"]["ack"]["status"], "proposed", "{p}");
    assert!(text(&p).contains("view v1 (new)"), "{}", text(&p));

    // glossary.
    let g = call(&mut s, "glossary", json!({"term": "breakout"}));
    assert!(text(&g).contains("meaning `breakout`"), "{}", text(&g));
    let all = call(&mut s, "glossary", json!({}));
    assert!(text(&all).contains("stage `retracted`"), "{}", text(&all));

    // Bad arguments are a JSON-RPC error.
    let r = rpc(
        &mut s,
        9,
        "tools/call",
        json!({"name": "find", "arguments": {"nope": 1}}),
    );
    assert_eq!(r["error"]["code"], -32602, "{r}");
}

/// M29: `card`, `layered` (with `detail`) and `json` carry the same numbers — card and
/// layered exactly the same, json all of them (and more); numbers are printed as the
/// protocol gave them.
#[test]
fn the_three_formats_carry_the_same_numbers() {
    let h = hub();
    let mut s = served(&h, "mcp-1", "claude");
    let f = call(&mut s, "find", json!({"limit": 5}));
    let id = f["structuredContent"]["ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let inst = id.split('.').nth(1).unwrap().to_owned();
    let cases = [
        ("find", json!({"limit": 5})),
        ("explain", json!({"capsule": id})),
        ("instrument", json!({"symbol": inst, "limit": 5})),
        ("whats_new", json!({"since_t": 0, "limit": 5})),
    ];
    for (tool, args) in cases {
        let with = |fmt: &str, detail: bool| {
            let mut a = args.clone();
            a["format"] = json!(fmt);
            a["detail"] = json!(detail);
            a
        };
        let card = call(&mut s, tool, with("card", false));
        let layered = call(&mut s, tool, with("layered", true));
        let summary = call(&mut s, tool, with("layered", false));
        let full = call(&mut s, tool, with("json", false));
        let (c, l, sm, j) = (
            nums(&text(&card)),
            nums(&text(&layered)),
            nums(&text(&summary)),
            all_nums(&full),
        );
        assert!(!c.is_empty(), "{tool}");
        assert_eq!(c, l, "{tool}: card and layered differ");
        assert!(
            sm.is_subset(&c),
            "{tool}: summary has numbers the card does not"
        );
        let missing: Vec<&String> = c.difference(&j).collect();
        assert!(missing.is_empty(), "{tool}: not in json: {missing:?}");
        assert_eq!(
            card["structuredContent"]["ids"],
            layered["structuredContent"]["ids"]
        );
    }
    // An evidence value is printed with the protocol's own digits.
    let full = call(&mut s, "explain", json!({"capsule": id, "format": "json"}));
    let card = text(&call(&mut s, "explain", json!({"capsule": id})));
    for e in full["structuredContent"]["detail"]["evidence"]
        .as_array()
        .unwrap()
    {
        for (k, v) in e["values"].as_object().into_iter().flatten() {
            assert!(card.contains(&format!("{k}={v}")), "{k}={v} not in {card}");
        }
    }
}

/// K33 on the agent path: an agent working for a person with no data entitlement gets
/// no licensed number (evidence values the entitled agent sees) from any tool, in any
/// format; the evidence says what it requires instead; a comment carrying such a number
/// is rejected without telling why it is licensed.
#[test]
fn an_unentitled_readers_agent_gets_no_licensed_number() {
    let h = hub();
    let mut owner = served(&h, "mcp-a", "claude");
    let mut guest = served(&h, "mcp-g", "claude-guest");

    // The licensed numbers: every evidence value the entitled agent sees (non-integers,
    // so that bar numbers and counts do not collide).
    let f = call(&mut owner, "find", json!({"limit": 50, "format": "json"}));
    let ids: Vec<String> = f["structuredContent"]["capsules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(!ids.is_empty());
    let mut licensed = BTreeSet::new();
    let mut a_value = None;
    for id in &ids {
        let e = call(
            &mut owner,
            "explain",
            json!({"capsule": id, "format": "json"}),
        );
        for ev in e["structuredContent"]["detail"]["evidence"]
            .as_array()
            .unwrap()
        {
            assert_eq!(ev["open"], true);
            for v in ev["values"].as_object().unwrap().values() {
                let x = v.as_f64().unwrap();
                if x.fract() != 0.0 {
                    licensed.insert(v.to_string());
                    a_value.get_or_insert((id.clone(), v.to_string()));
                }
            }
        }
    }
    assert!(!licensed.is_empty());

    let mut answers = Vec::new();
    for fmt in ["card", "layered", "json"] {
        answers.push(call(
            &mut guest,
            "whats_new",
            json!({"since_t": 0, "format": fmt, "detail": true}),
        ));
        answers.push(call(
            &mut guest,
            "find",
            json!({"limit": 50, "format": fmt, "detail": true}),
        ));
        for id in &ids {
            answers.push(call(
                &mut guest,
                "explain",
                json!({"capsule": id, "format": fmt, "detail": true}),
            ));
        }
        for sym in ["SYN01", "SYN02", "SYN03"] {
            answers.push(call(
                &mut guest,
                "instrument",
                json!({"symbol": sym, "format": fmt, "detail": true, "limit": 50}),
            ));
        }
    }
    answers.push(call(&mut guest, "instrument", json!({})));
    answers.push(call(&mut guest, "track_record", json!({})));
    answers.push(call(&mut guest, "track_record", json!({"by_kind": true})));
    answers.push(call(&mut guest, "glossary", json!({})));
    answers.push(call(&mut guest, "watch", json!({"action": "list"})));
    let (cid, value) = a_value.unwrap();
    let leak = call(
        &mut guest,
        "comment",
        json!({"capsule": cid, "lang": "en", "text": format!("The level is {value} [1]."), "cites": [cid]}),
    );
    assert_eq!(leak["isError"], true, "{leak}");
    assert!(!text(&leak).contains("licensed"), "{}", text(&leak));
    answers.push(leak);

    let mut requires = false;
    for r in &answers {
        let got = all_nums(r);
        let hit: Vec<&String> = got.intersection(&licensed).collect();
        assert!(hit.is_empty(), "licensed numbers {hit:?} in {r}");
        requires |= text(r).contains("requires data license");
    }
    assert!(requires, "the evidence says what it requires");
}
