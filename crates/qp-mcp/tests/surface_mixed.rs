//! (R5B, M28 d) The mixed surface: task tools (`overview`, `find`, `capsule`, `watch`,
//! `write`) and the escape tool `query`, on a synthetic replay session. Every tool is
//! called; no decision is possible; an agent working for an unentitled person never
//! receives a licensed number from any tool; the three capsule formats carry the same
//! numbers.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qp_mcp::surfaces::{self, mixed};
use qp_mcp::{Config, Hub, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::protocol::{ReplayControl, ReplayInit, Surface};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn headless() -> Surface {
    serde_json::from_value(json!({"kind": "headless"})).unwrap()
}

/// The synthetic replay (`qp_session::demo_init`): three instruments; `arkin` and its
/// agent `claude` read everything, `guest` and its agent `claude-guest` read nothing.
fn hub() -> SharedHub {
    let all = || vec!["*".to_owned()];
    let mut init = qp_session::demo_init(ReplayInit {
        instruments: Some(3),
        days: Some(90),
        history: Some(60),
        entitlements: [
            ("host".to_owned(), all()),
            ("arkin".to_owned(), all()),
            ("claude".to_owned(), all()),
            ("guest".to_owned(), Vec::new()),
            ("claude-guest".to_owned(), Vec::new()),
        ]
        .into(),
        ..ReplayInit::default()
    });
    for (a, p) in [("claude", "arkin"), ("claude-guest", "guest")] {
        init.principals.push(
            serde_json::from_value(json!({"id": a, "author": {"kind": "agent", "name": a},
                "role": "proposer", "on_behalf": p}))
            .unwrap(),
        );
    }
    Hub::shared_replay(
        qp_session::ReplayHost::with_local_dir(init, &root(), Path::new("")).unwrap(),
    )
}

fn step(h: &SharedHub, n: u64) {
    let c: ReplayControl = serde_json::from_value(json!({"kind": "step", "n": n})).unwrap();
    h.borrow_mut().host_control(&c).unwrap();
}

/// An agent endpoint as `qp mcp serve` opens it (marked connection, common tools).
fn served(h: &SharedHub, principal: &str) -> McpServer {
    let conn = format!("mcp-{principal}");
    h.borrow_mut().mark_agent_connection(&conn);
    McpServer::open(
        h.clone(),
        &conn,
        Config::new(principal, headless()),
        surfaces::by_name("mixed").unwrap(),
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

fn call(s: &mut McpServer, name: &str, args: Value) -> Value {
    s.call_tool(name, args, None)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"))
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

/// The whole result as one string (text content and structured content).
fn all_text(r: &Value) -> String {
    format!("{}\n{}", text(r), r["structuredContent"])
}

fn ok(r: &Value) -> &Value {
    assert_eq!(r["isError"], false, "{r}");
    r
}

fn capsule_ids(s: &mut McpServer) -> Vec<String> {
    let r = call(s, "find", json!({"format": "json", "limit": 50}));
    ok(&r)["structuredContent"]["capsules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn tools_list_offers_the_task_tools_the_escape_and_no_decision() {
    let h = hub();
    let mut s = served(&h, "claude");
    let list = rpc(&mut s, 1, "tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap().clone();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let mut want: Vec<&str> = mixed::TOOLS.to_vec();
    want.extend(["render", "notifications"]);
    assert_eq!(names, want);
    for t in &tools {
        let n = t["name"].as_str().unwrap();
        for d in mixed::DECISION_OPS {
            assert!(!n.contains(d), "{n}");
        }
        let schema = &t["inputSchema"];
        assert_eq!(schema["type"], "object", "{n}");
        // No top-level combinator (some clients refuse them).
        for k in ["oneOf", "anyOf", "allOf"] {
            assert!(schema.get(k).is_none(), "{n}: top-level {k}");
        }
        let d = t["description"].as_str().unwrap();
        assert!(
            d.contains("{\"") || n == "render" || n == "notifications",
            "{n}"
        );
    }
    // `write` proposals: the protocol `Op` schema narrowed to the proposal-only ops.
    let write = tools.iter().find(|t| t["name"] == "write").unwrap();
    let ops: BTreeSet<&str> = write["inputSchema"]["$defs"]["Op"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["properties"]["op"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(ops, mixed::PROPOSAL_OPS.into_iter().collect());
    // The escape tool's `query` is the protocol `Query` schema (every kind).
    let q = tools.iter().find(|t| t["name"] == "query").unwrap();
    let kinds: BTreeSet<&str> = q["inputSchema"]["$defs"]["Query"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["properties"]["kind"]["const"].as_str().unwrap())
        .collect();
    for k in [
        "describe", "list", "capsules", "capsule", "stories", "ledger", "at", "study", "log",
        "screen", "preview",
    ] {
        assert!(kinds.contains(k), "{k}");
    }
    // Every $ref resolves inside the tool's own schema.
    for t in &tools {
        let s = t["inputSchema"].to_string();
        for part in s.split("\"$ref\":\"#/$defs/").skip(1) {
            let name = &part[..part.find('"').unwrap()];
            assert!(
                t["inputSchema"]["$defs"].get(name).is_some(),
                "{}: {name}",
                t["name"]
            );
        }
    }
    let bytes: usize = tools.iter().map(|t| t.to_string().len()).sum();
    let own: usize = tools
        .iter()
        .filter(|t| mixed::TOOLS.contains(&t["name"].as_str().unwrap()))
        .map(|t| t.to_string().len())
        .sum();
    for t in &tools {
        eprintln!("  {}: {} bytes", t["name"], t.to_string().len());
    }
    eprintln!(
        "mixed tools/list: {} tools, {bytes} bytes ({own} without render/notifications)",
        tools.len()
    );
    let instr = rpc(&mut s, 2, "server/discover", json!({}));
    let i = instr["result"]["instructions"].as_str().unwrap();
    let sentences = i.matches(". ").count() + 1;
    assert!((5..=8).contains(&sentences), "{sentences}: {i}");
}

#[test]
fn every_tool_answers_on_the_synthetic_replay() {
    let h = hub();
    step(&h, 5);
    let mut s = served(&h, "claude");

    // overview: card by default, with the next step and the ids.
    let o = call(&mut s, "overview", json!({}));
    let t = text(ok(&o));
    assert!(t.starts_with("Overview at now"), "{t}");
    assert!(t.contains("cap."), "{t}");
    let oj = call(&mut s, "overview", json!({"format": "json", "limit": 3}));
    let now = ok(&oj)["structuredContent"]["now"].as_i64().unwrap();
    assert!(
        oj["structuredContent"]["capsules"]
            .as_array()
            .unwrap()
            .len()
            <= 3
    );
    let since = call(
        &mut s,
        "overview",
        json!({"since_t": now, "format": "layered"}),
    );
    ok(&since);

    // find: filters, card and json.
    let ids = capsule_ids(&mut s);
    assert!(!ids.is_empty());
    let f = call(
        &mut s,
        "find",
        json!({"instrument": "SYN01", "open": false}),
    );
    assert!(text(ok(&f)).contains("capsule(s)"));
    let fl = call(
        &mut s,
        "find",
        json!({"meaning": "breakout", "latest": true, "format": "json"}),
    );
    assert!(
        ok(&fl)["structuredContent"]["capsules"]
            .as_array()
            .unwrap()
            .len()
            <= 1
    );

    // capsule: by id and by selector (the reply names the resolved id).
    let c = call(&mut s, "capsule", json!({"capsule": ids[0]}));
    let ct = text(ok(&c));
    assert!(
        ct.contains(&ids[0]) && ct.contains("Evidence:") && ct.contains("Ids: "),
        "{ct}"
    );
    let sel = call(
        &mut s,
        "capsule",
        json!({"capsule": {"meaning": "breakout", "latest": true}, "format": "json"}),
    );
    assert!(
        ok(&sel)["structuredContent"]["resolved"]["/query/capsule"].is_string(),
        "{sel}"
    );
    let missing = call(
        &mut s,
        "capsule",
        json!({"capsule": "cap.NOPE.breakout.up.1"}),
    );
    assert_eq!(missing["isError"], true);
    assert!(text(&missing).contains("not_found"), "{missing}");

    // watch: add (subscribes to intents), list, matches on the replay, remove; follow.
    let w = call(
        &mut s,
        "watch",
        json!({"action": "add", "text": "anything on SYN01", "match": {"instrument": "SYN01"}}),
    );
    let wid = ok(&w)["structuredContent"]["created"][0]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(wid.starts_with("watch-"), "{w}");
    let l = call(&mut s, "watch", json!({"action": "list"}));
    assert!(text(ok(&l)).contains(&wid));
    let fo = call(
        &mut s,
        "watch",
        json!({"action": "follow", "instrument": "SYN02"}),
    );
    let sub = ok(&fo)["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    step(&h, 40);
    let n = call(&mut s, "notifications", json!({}));
    let ns = n["structuredContent"]["notifications"].to_string();
    assert!(ns.contains("intent_matched") && ns.contains(&wid), "{ns}");
    assert!(ns.contains(&sub), "{ns}");
    let un = call(&mut s, "watch", json!({"action": "unfollow", "sub": sub}));
    ok(&un);
    let rm = call(&mut s, "watch", json!({"action": "remove", "intent": wid}));
    ok(&rm);
    let again = call(&mut s, "watch", json!({"action": "remove", "intent": wid}));
    assert_eq!(again["isError"], true);
    let bad = call(&mut s, "watch", json!({"action": "add"}));
    assert_eq!(bad["isError"], true);
    assert!(text(&bad).contains("needs `text` and `match`"));

    // write: a cited comment is published; a made-up number is refused with reasons.
    let ids = capsule_ids(&mut s);
    let cap = &ids[0];
    let good = call(
        &mut s,
        "write",
        json!({"kind": "comment", "about": {"capsule": cap}, "lang": "en",
               "text": "The engine reports this story [1].", "cites": [cap]}),
    );
    let gt = text(ok(&good));
    assert!(gt.starts_with("Comment cmt-"), "{gt}");
    let c2 = call(&mut s, "capsule", json!({"capsule": cap}));
    assert!(text(&c2).contains("Comments (1)"), "{c2}");
    let wrong = call(
        &mut s,
        "write",
        json!({"kind": "comment", "about": {"capsule": cap}, "lang": "en",
               "text": "It rose 47.123 percent [1].", "cites": [cap]}),
    );
    assert_eq!(wrong["isError"], true, "{wrong}");
    let wt = text(&wrong);
    assert!(
        wt.contains("comment_rejected") && wt.contains("reasons"),
        "{wt}"
    );
    assert_eq!(
        wrong["structuredContent"]["error"]["code"], "comment_rejected",
        "{wrong}"
    );
    assert!(wrong["structuredContent"]["error"]["reasons"][0]["hint"].is_string());

    // write: a view proposal is pending (a person decides), visible in overview and preview.
    let series = ids
        .iter()
        .find(|i| i.starts_with("cap.SYN01"))
        .map_or("syn01", |_| "syn01");
    let p = call(
        &mut s,
        "write",
        json!({"kind": "proposal", "reason": "a price view",
               "ops": [{"op": "put_view", "view": {"id": "agentv", "x": {"kind": "time"},
                 "panels": [{"id": 0, "y": {"kind": "linear"}}],
                 "layers": [{"id": "c", "mark": "candle", "series": series, "panel": 0}]}}]}),
    );
    let pid = ok(&p)["structuredContent"]["proposal"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(p["structuredContent"]["status"], "proposed");
    let ov = call(&mut s, "overview", json!({}));
    assert!(text(&ov).contains(&pid));

    // query: the escape runs any query kind as is.
    let pv = call(
        &mut s,
        "query",
        json!({"query": {"kind": "preview", "proposal": pid}}),
    );
    assert_eq!(
        ok(&pv)["structuredContent"]["data"]["kind"],
        "preview",
        "{pv}"
    );
    for q in [
        json!({"kind": "describe"}),
        json!({"kind": "stories", "instrument": "SYN01"}),
        json!({"kind": "ledger", "by_kind": true}),
        json!({"kind": "log", "limit": 2}),
        json!({"kind": "study", "series": {"kind": "bars", "symbol": "SYN01"}, "reduce": "last"}),
    ] {
        let r = call(&mut s, "query", json!({"query": q}));
        assert_eq!(ok(&r)["structuredContent"]["type"], "result", "{q}: {r}");
    }
    let qe = call(
        &mut s,
        "query",
        json!({"query": {"kind": "stories", "instrument": "NOPE"}}),
    );
    assert_eq!(qe["isError"], true);
    let nk = call(&mut s, "query", json!({"query": {"instrument": "SYN01"}}));
    assert_eq!(nk["isError"], true);

    // Malformed arguments: invalid params.
    let e = s.call_tool("find", json!({"bogus": 1}), None).unwrap_err();
    assert_eq!(e.code, -32602);
    let e = s.call_tool("nope", json!({}), None).unwrap_err();
    assert_eq!(e.code, -32602);
}

#[test]
fn no_decision_is_possible_from_the_mixed_surface() {
    let h = hub();
    let mut s = served(&h, "claude");
    let p = call(
        &mut s,
        "write",
        json!({"kind": "proposal", "ops": [{"op": "put_view", "view": {"id": "v2",
            "x": {"kind": "time"}, "panels": [{"id": 0, "y": {"kind": "linear"}}],
            "layers": [{"id": "c", "mark": "candle", "series": "syn01", "panel": 0}]}}]}),
    );
    let pid = ok(&p)["structuredContent"]["proposal"]
        .as_str()
        .unwrap()
        .to_owned();
    let log_len = || h.borrow().session().log().len();
    let before = log_len();
    for (i, op) in mixed::DECISION_OPS.iter().enumerate() {
        let o = if op.ends_with("_proposal") {
            json!({"op": op, "proposal": pid})
        } else {
            json!({"op": op, "finding": {"latest": true}})
        };
        // Alone, and hidden behind an allowed op: refused before anything is sent.
        for ops in [
            json!([o]),
            json!([{"op": "set_view_range", "view": "v2"}, o]),
        ] {
            let r = call(&mut s, "write", json!({"kind": "proposal", "ops": ops}));
            assert_eq!(r["isError"], true, "{op}: {r}");
            assert!(text(&r).contains("person's decision"), "{op} {i}: {r}");
            assert!(r.get("structuredContent").is_none());
        }
        // Not a tool either.
        assert_eq!(
            s.call_tool(&format!("op_{op}"), json!({"proposal": pid}), None)
                .unwrap_err()
                .code,
            -32602
        );
    }
    assert_eq!(log_len(), before, "nothing was written");
    // Still pending: only a person decides it.
    let q = call(
        &mut s,
        "query",
        json!({"query": {"kind": "list", "collection": "proposals", "state": "pending"}}),
    );
    assert_eq!(q["structuredContent"]["data"]["items"][0]["id"], json!(pid));
    // The escape tool is read-only: it sends queries only.
    let list = rpc(&mut s, 9, "tools/list", json!({}));
    for t in list["result"]["tools"].as_array().unwrap() {
        if t["name"] == "query" {
            assert_eq!(t["annotations"]["readOnlyHint"], true);
        }
    }
}

/// Number tokens of a text, read with `.` or `,` as the decimal mark: (value, decimals).
fn number_tokens(s: &str) -> Vec<(f64, u32)> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let st = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b',') {
                i += 1;
            }
            let tok = s[st..i].trim_end_matches(['.', ',']);
            for mark in ['.', ','] {
                if let Some((a, f)) = tok.split_once(mark) {
                    if a.chars().all(|c| c.is_ascii_digit())
                        && f.chars().all(|c| c.is_ascii_digit())
                    {
                        if let Ok(v) = format!("{a}.{f}").parse::<f64>() {
                            out.push((v, u32::try_from(f.len()).unwrap()));
                        }
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `token` writes `value` (exactly, or rounded to the token's decimals, 2 or more).
fn writes(token: (f64, u32), value: f64) -> bool {
    let (t, d) = token;
    if (t - value).abs() < 1e-12 {
        return true;
    }
    if d >= 2 {
        let p = 10f64.powi(i32::try_from(d).unwrap());
        return ((value.abs() * p).round() / p - t).abs() < 1e-9;
    }
    false
}

#[test]
fn an_agent_for_an_unentitled_guest_gets_no_licensed_number_from_any_tool() {
    let h = hub();
    step(&h, 10);
    let mut owner = served(&h, "claude");
    let mut guest = served(&h, "claude-guest");

    // The licensed numbers: every evidence value and fact value the entitled agent reads,
    // and the bars' last closes (non-integers: bar indices and times are public).
    let ids = capsule_ids(&mut owner);
    assert!(ids.len() >= 3, "{ids:?}");
    let mut licensed: Vec<f64> = Vec::new();
    for id in &ids {
        let r = call(
            &mut owner,
            "capsule",
            json!({"capsule": id, "format": "json"}),
        );
        let d = &ok(&r)["structuredContent"]["detail"];
        for e in d["evidence"].as_array().unwrap() {
            for v in e["values"].as_object().into_iter().flatten().map(|x| x.1) {
                licensed.push(v.as_f64().unwrap());
            }
        }
    }
    for sym in ["SYN01", "SYN02", "SYN03"] {
        let r = call(
            &mut owner,
            "query",
            json!({"query": {"kind": "study", "series": {"kind": "bars", "symbol": sym}, "range": {"last": 30}}}),
        );
        for out in ["o", "h", "l", "c"] {
            for v in ok(&r)["structuredContent"]["data"]["values"]["outputs"][out]
                .as_array()
                .unwrap()
            {
                if let Some(x) = v.as_f64() {
                    licensed.push(x);
                }
            }
        }
    }
    licensed.retain(|v| v.fract() != 0.0);
    assert!(licensed.len() > 50, "{}", licensed.len());

    // The guest's agent: every tool, every format.
    let gids = capsule_ids(&mut guest);
    assert_eq!(gids, ids, "the same stories, redacted");
    let mut answers: Vec<(String, Value)> = Vec::new();
    for f in ["card", "json", "layered"] {
        answers.push((
            format!("overview {f}"),
            call(
                &mut guest,
                "overview",
                json!({"format": f, "detail": true, "limit": 50}),
            ),
        ));
        answers.push((
            format!("find {f}"),
            call(
                &mut guest,
                "find",
                json!({"format": f, "detail": true, "limit": 50}),
            ),
        ));
        for id in &gids {
            answers.push((
                format!("capsule {id} {f}"),
                call(
                    &mut guest,
                    "capsule",
                    json!({"capsule": id, "format": f, "detail": true}),
                ),
            ));
        }
    }
    for q in [
        json!({"kind": "capsules"}),
        json!({"kind": "stories", "instrument": "SYN01"}),
        json!({"kind": "ledger", "by_kind": true}),
        json!({"kind": "log", "limit": 20}),
        json!({"kind": "list", "collection": "facts"}),
        json!({"kind": "study", "series": "syn01", "reduce": "last"}),
        json!({"kind": "at", "series": "syn01", "bar": 40}),
        json!({"kind": "document"}),
    ] {
        answers.push((
            format!("query {q}"),
            call(&mut guest, "query", json!({"query": q})),
        ));
    }
    answers.push((
        "watch list".into(),
        call(&mut guest, "watch", json!({"action": "list"})),
    ));
    // A comment quoting a licensed number is refused without saying why (no oracle).
    let lic = licensed
        .iter()
        .find(|v| format!("{v:.2}").parse::<f64>().ok() == Some(**v) && **v > 1.0)
        .copied();
    if let Some(v) = lic {
        let r = call(
            &mut guest,
            "write",
            json!({"kind": "comment", "about": {"capsule": gids[0]}, "lang": "en",
                   "text": format!("It closed at {v:.2} [1]."), "cites": [gids[0]]}),
        );
        assert_eq!(r["isError"], true, "{r}");
        let t = all_text(&r);
        assert!(!t.contains("licensed"), "{t}");
    }
    let step_answers = answers.len();
    step(&h, 20);
    answers.push((
        "notifications".into(),
        call(&mut guest, "notifications", json!({})),
    ));

    let mut checked = 0usize;
    for (what, r) in &answers {
        let t = all_text(r);
        for tok in number_tokens(&t) {
            for v in &licensed {
                assert!(
                    !writes(tok, *v),
                    "{what}: token {tok:?} writes licensed {v}\n{t}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 100 && step_answers > 20, "{checked}");
    // The redaction is real: the entitled agent's card carries numbers the guest's lacks.
    let oc = text(&call(&mut owner, "capsule", json!({"capsule": ids[0]})));
    assert!(
        number_tokens(&oc)
            .iter()
            .any(|t| licensed.iter().any(|v| writes(*t, *v))),
        "{oc}"
    );
}

/// The key numbers of a capsule as the protocol writes them.
fn key_numbers(detail: &Value) -> Vec<String> {
    let i = &detail["item"];
    let mut out: Vec<String> = ["importance", "born_bar", "last_bar", "tier", "t"]
        .iter()
        .map(|k| i[*k].to_string())
        .collect();
    out.push(detail["ledger"]["n"].to_string());
    out.push(detail["ledger"]["realized"].to_string());
    for e in detail["evidence"].as_array().unwrap() {
        for v in e["values"].as_object().into_iter().flatten().map(|x| x.1) {
            out.push(v.to_string());
        }
    }
    out
}

#[test]
fn the_three_formats_carry_the_same_numbers() {
    let h = hub();
    step(&h, 10);
    let mut s = served(&h, "claude");
    let ids = capsule_ids(&mut s);
    let mut seen_values = 0;
    for id in ids.iter().take(6) {
        let j = call(&mut s, "capsule", json!({"capsule": id, "format": "json"}));
        let detail = ok(&j)["structuredContent"]["detail"].clone();
        let card = text(&call(&mut s, "capsule", json!({"capsule": id})));
        let layered = text(&call(
            &mut s,
            "capsule",
            json!({"capsule": id, "format": "layered", "detail": true}),
        ));
        let summary = call(
            &mut s,
            "capsule",
            json!({"capsule": id, "format": "layered"}),
        );
        let jt = text(&j);
        for n in key_numbers(&detail) {
            for (f, t) in [("json", &jt), ("card", &card), ("layered", &layered)] {
                assert!(t.contains(&n), "{id}: {n} missing in {f}:\n{t}");
            }
            seen_values += 1;
        }
        // The summary layer: the item's numbers, unchanged.
        let st = summary["structuredContent"]["summary"].clone();
        for k in ["importance", "born_bar", "last_bar", "tier"] {
            assert_eq!(st[k], detail["item"][k], "{id} {k}");
        }
        assert_eq!(st["ledger"], detail["ledger"]);
        // Every number the card writes outside the sentences is a number of the json.
        let jn: BTreeSet<String> = number_tokens(&jt)
            .into_iter()
            .map(|(v, _)| v.to_string())
            .collect();
        for line in card.lines().filter(|l| !l.starts_with("Sentence:")) {
            for (v, _) in number_tokens(line) {
                assert!(
                    jn.contains(&v.to_string()),
                    "{id}: {v} in card line `{line}`"
                );
            }
        }
    }
    assert!(seen_values > 20, "{seen_values}");
    // Lists too: each capsule's importance and bars in all three overview formats.
    let oj = call(&mut s, "overview", json!({"format": "json", "limit": 50}));
    let oc = text(&call(&mut s, "overview", json!({"limit": 50})));
    let ol = text(&call(
        &mut s,
        "overview",
        json!({"format": "layered", "limit": 50}),
    ));
    for c in oj["structuredContent"]["capsules"].as_array().unwrap() {
        for k in ["importance", "born_bar", "last_bar", "id"] {
            let n = super_plain(&c[k]);
            assert!(oc.contains(&n) && ol.contains(&n), "{k} {n}");
        }
    }
}

fn super_plain(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), str::to_owned)
}
