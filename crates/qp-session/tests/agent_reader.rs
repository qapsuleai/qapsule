//! (R5B, `docs/PROTOCOL.md` §25.7) An agent reads with the entitlements of the person it
//! works for (`on_behalf`), intersected with its own row when it has one: non-interference
//! on the agent path. In a session with a licensed flow (transcript 39), an agent working
//! for an unentitled person gets, on every query and subscription, the bytes the person
//! gets (identity fields aside) — so never a licensed number.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_schema::data::DataEvent;
use qp_schema::protocol::{Principal, SessionInit, Transcript};
use qp_session::Session;
use serde_json::{json, Value};
use support::root;

fn transcript() -> Transcript {
    qp_session::read_json(&root().join("fixtures/protocol/conformance/39-capsule-entitlement.json"))
        .unwrap()
}

fn agent(id: &str, person: &str) -> Principal {
    serde_json::from_value(json!({"id":id,"author":{"kind":"agent","name":id},
        "role":"viewer","on_behalf":person}))
    .unwrap()
}

/// Transcript 39's init with agents for `bora` (bar feed only) and `can` (nothing): `bb`
/// and `cc` have no row of their own, `bb2` and `cc2` have `*` (the intersection holds).
fn init() -> SessionInit {
    let qp_schema::protocol::InitRef::Inline(i) = transcript().init else {
        panic!("inline init")
    };
    let mut i = *i;
    for (a, p) in [
        ("bb", "bora"),
        ("cc", "can"),
        ("bb2", "bora"),
        ("cc2", "can"),
    ] {
        i.principals.push(agent(a, p));
    }
    let d = i.data.as_mut().unwrap();
    d.entitlements.insert("bb2".into(), vec!["*".into()]);
    d.entitlements.insert("cc2".into(), vec!["*".into()]);
    i
}

/// `v` without identity fields (principal ids).
fn anon(mut v: Value) -> Value {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(m) => {
                m.remove("principal");
                for x in m.values_mut() {
                    walk(x);
                }
            }
            Value::Array(a) => a.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut v);
    v
}

/// Lines of `out` for connection `conn`, as bytes without identity fields (principal
/// fields dropped; an agent's id in a message text read as its person's).
fn of(out: &[Value], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l["conn"] == conn)
        .map(|l| {
            let mut t = anon(l["msg"].clone()).to_string();
            for (a, p) in [
                ("bb2", "bora"),
                ("cc2", "can"),
                ("bb", "bora"),
                ("cc", "can"),
            ] {
                t = t.replace(&format!("`{a}`"), &format!("`{p}`"));
            }
            t
        })
        .collect()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<Value> {
    s.handle_raw(conn, &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

/// Person connection → its agents' connections (and whether the host marks them).
const PAIRS: [(&str, &str, &str, &str); 4] = [
    ("b", "bora", "B", "bb"),
    ("c", "can", "C", "cc"),
    ("b", "bora", "B2", "bb2"),
    ("c", "can", "C2", "cc2"),
];

fn mirror(msg: &Value, agent: &str) -> Value {
    let mut m = msg.clone();
    if m["type"] == "hello" {
        m["principal"] = json!(agent);
    }
    m
}

/// Queries every reader asks after the transcript (more read paths than 39 has).
fn extra_queries() -> Vec<Value> {
    [
        json!({"kind":"capsules"}),
        json!({"kind":"capsules","audience":true}),
        json!({"kind":"capsule","capsule":"cap.THYAO.flow.up.90"}),
        json!({"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"flow","latest":true}}),
        json!({"kind":"ledger","by_kind":true}),
        json!({"kind":"stories","instrument":"THYAO","range":{"last":60}}),
        json!({"kind":"list","collection":"facts"}),
        json!({"kind":"list","collection":"capsules"}),
        json!({"kind":"document"}),
        json!({"kind":"log","limit":5}),
        json!({"kind":"describe"}),
        json!({"kind":"binding"}),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, q)| json!({"type":"query","id":format!("x{i}"),"query":q}))
    .collect()
}

#[test]
fn an_agent_for_an_unentitled_person_gets_the_persons_bytes() {
    let t = transcript();
    let mut s = Session::new(init(), &root()).unwrap();
    // One pair of connections is marked by the host (an MCP connection): same answers.
    s.mark_agent_connection("B2");
    s.mark_agent_connection("C2");
    let mut compared = 0usize;
    for step in &t.steps {
        if let Some(send_) = &step.send {
            let msg = &serde_json::to_value(&send_.msg).unwrap();
            let out = send(&mut s, &send_.conn, msg);
            for (pc, _, ac, a) in PAIRS {
                if pc == send_.conn.as_str() {
                    let aout = send(&mut s, ac, &mirror(msg, a));
                    assert_eq!(of(&out, pc), of(&aout, ac), "step {msg}");
                    compared += 1;
                }
            }
        } else if let Some(d) = &step.data {
            let ev: DataEvent = serde_json::from_value(serde_json::to_value(d).unwrap()).unwrap();
            let out: Vec<Value> = s
                .data_event(&ev)
                .unwrap()
                .iter()
                .map(|l| serde_json::to_value(l).unwrap())
                .collect();
            for (pc, _, ac, _) in PAIRS {
                assert_eq!(of(&out, pc), of(&out, ac), "data step");
                compared += 1;
            }
        }
    }
    for q in extra_queries() {
        for (pc, _, ac, _) in PAIRS {
            let p = send(&mut s, pc, &q);
            let a = send(&mut s, ac, &q);
            assert_eq!(of(&p, pc), of(&a, ac), "query {q}");
            compared += 1;
        }
    }
    assert!(compared > 40, "{compared}");
}

#[test]
fn an_agent_with_its_own_wide_row_still_reads_as_its_person() {
    // `cc2` has `*` of its own but works for `can` (nothing): the intersection is nothing.
    let mut s = Session::new(init(), &root()).unwrap();
    for (c, p) in [("c", "can"), ("C2", "cc2"), ("A", "ana")] {
        send(
            &mut s,
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
        );
    }
    let q = json!({"type":"query","id":"q","query":{"kind":"binding"}});
    let can = send(&mut s, "c", &q);
    let agent = send(&mut s, "C2", &q);
    let ana = send(&mut s, "A", &q);
    assert_eq!(of(&can, "c"), of(&agent, "C2"));
    assert_ne!(of(&ana, "A"), of(&agent, "C2"), "ana is entitled");
    assert!(of(&agent, "C2")[0].contains("no_entitlement"));
}

#[test]
fn on_behalf_must_name_a_human_who_works_for_nobody() {
    // Unknown, itself, another agent (a chain), a cycle: the session does not start.
    let cases: [&[(&str, &str)]; 4] = [
        &[("lost", "nobody")],
        &[("self", "self")],
        &[("chain", "bb")],
        &[("x1", "x2"), ("x2", "x1")],
    ];
    for case in cases {
        let mut i = init();
        for (a, p) in case {
            i.principals.push(agent(a, p));
        }
        let e = Session::new(i, &root()).err().expect("refused");
        assert!(e.contains("on_behalf"), "{e}");
    }
    // A human that works for someone cannot be a target either.
    let mut i = init();
    i.principals.push(
        serde_json::from_value(json!({"id":"h2","author":{"kind":"human","name":"h2"},
            "role":"viewer","on_behalf":"ana"}))
        .unwrap(),
    );
    i.principals.push(agent("via", "h2"));
    assert!(Session::new(i, &root()).is_err());
}

/// A provider that cannot say whether a principal has a row of its own (`has_row`
/// default): the agent is held to the intersection (fail closed).
struct Opaque(qp_source::MemoryProvider);

impl qp_source::SourceProvider for Opaque {
    fn entitled(&self, p: &str, f: &qp_schema::data::FeedKey) -> bool {
        self.0.entitled(p, f)
    }
    fn resolve(&self, p: &str, b: &qp_schema::data::DataBinding) -> qp_source::Resolution {
        self.0.resolve(p, b)
    }
}

#[test]
fn an_unknown_row_is_read_as_a_row_fail_closed() {
    let mut i = init();
    let data = i.data.take().unwrap();
    let mp = qp_source::MemoryProvider::from_init(&data, &root()).unwrap();
    let mut s = Session::with_provider(i, &root(), Box::new(Opaque(mp))).unwrap();
    // `bb` has no row in the provider, but the provider does not say so: nothing.
    for (c, p) in [("b", "bora"), ("B", "bb"), ("B2", "bb2")] {
        send(
            &mut s,
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
        );
    }
    let q = json!({"type":"query","id":"q","query":{"kind":"binding"}});
    let bora = send(&mut s, "b", &q);
    let bb = send(&mut s, "B", &q);
    let bb2 = send(&mut s, "B2", &q);
    assert!(of(&bora, "b")[0].contains("ready"), "{bora:?}");
    assert!(of(&bb, "B")[0].contains("no_entitlement"), "{bb:?}");
    // `bb2` has `*` of its own: the intersection is bora's.
    assert_eq!(of(&bora, "b"), of(&bb2, "B2"));
}

/// (R5B, §25.7; M28c finding) An agent reads with its person's audience: capsules on the
/// person's holdings are `relevant` to it exactly as to the person, while another
/// person's audience never reaches it. Same comparison as above, with audiences set
/// (so every reader has a projection).
#[test]
fn an_agent_reads_with_its_persons_audience() {
    let t = transcript();
    let mut init = init();
    let qp_schema::protocol::DocumentRef::Inline(doc) = &mut init.document else {
        panic!("inline document")
    };
    let aud = |positions: Value, watch: Value| {
        serde_json::from_value::<qp_schema::capsule::Audience>(json!({
            "group":"habituated","sensitivity":"important",
            "watchlist": watch, "positions": positions}))
        .unwrap()
    };
    doc.audiences.insert(
        "bora".into(),
        aud(json!({"THYAO": 100.0}), json!(["THYAO"])),
    );
    doc.audiences
        .insert("can".into(), aud(json!({"THYAO": -5.0}), json!([])));
    doc.audiences
        .insert("arkin".into(), aud(json!({}), json!(["THYAO"])));
    let mut s = Session::new(init, &root()).unwrap();
    s.mark_agent_connection("B2");
    s.mark_agent_connection("C2");
    let mut relevant_seen = false;
    for step in &t.steps {
        if let Some(send_) = &step.send {
            let msg = &serde_json::to_value(&send_.msg).unwrap();
            let out = send(&mut s, &send_.conn, msg);
            for (pc, _, ac, a) in PAIRS {
                if pc == send_.conn.as_str() {
                    let aout = send(&mut s, ac, &mirror(msg, a));
                    assert_eq!(of(&out, pc), of(&aout, ac), "step {msg}");
                }
            }
        } else if let Some(d) = &step.data {
            let ev: DataEvent = serde_json::from_value(serde_json::to_value(d).unwrap()).unwrap();
            let out: Vec<Value> = s
                .data_event(&ev)
                .unwrap()
                .iter()
                .map(|l| serde_json::to_value(l).unwrap())
                .collect();
            for (pc, _, ac, _) in PAIRS {
                assert_eq!(of(&out, pc), of(&out, ac), "data step");
            }
        }
    }
    for q in extra_queries() {
        for (pc, _, ac, _) in PAIRS {
            let p = send(&mut s, pc, &q);
            let a = send(&mut s, ac, &q);
            assert_eq!(of(&p, pc), of(&a, ac), "query {q}");
            let text = a.iter().map(Value::to_string).collect::<String>();
            relevant_seen |= text.contains("\"relevant\":true");
            // Never another person's audience.
            let other = if pc == "b" { "\"can\":{" } else { "\"bora\":{" };
            assert!(!text.contains(other), "query {q}: {text}");
            assert!(!text.contains("\"arkin\":{\"group\""), "query {q}");
        }
    }
    assert!(relevant_seen, "no capsule was relevant to an agent");
}
