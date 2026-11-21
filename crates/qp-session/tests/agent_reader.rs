//! (R5B, `docs/PROTOCOL.md` §25.7) An agent reads with the entitlements of the person it
//! works for (`on_behalf`), intersected with its own row when it has one: non-interference
//! on the agent path. In a session with a licensed flow (transcript 39), an agent working
//! for an unentitled person gets, on every query and subscription, the bytes the person
//! gets (identity fields aside) — so never a licensed number.

mod common;

use common::root;
use qp_schema::data::DataEvent;
use qp_schema::protocol::{Principal, SessionInit, Transcript};
use qp_session::Session;
use serde_json::{json, Value};

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
fn on_behalf_must_name_another_known_principal() {
    let mut i = init();
    i.principals.push(agent("lost", "nobody"));
    let e = Session::new(i, &root()).err().expect("refused");
    assert!(e.contains("on_behalf"), "{e}");
}
