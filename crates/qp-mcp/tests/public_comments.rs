//! (R5B audit B1) An entitled agent's comment never enters the public world: a
//! public-view reader (here `guest`, entitled to no licensed flow) receives it on no
//! path, however its text carries a number (superscript digits, another instrument).

use std::path::{Path, PathBuf};

use qp_mcp::{Hub, SharedHub};
use qp_schema::protocol::{ReplayControl, ReplayInit};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

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
        ]
        .into(),
        ..ReplayInit::default()
    });
    init.principals.push(
        serde_json::from_value(
            json!({"id": "claude", "author": {"kind": "agent", "name": "claude"},
            "role": "proposer", "on_behalf": "arkin"}),
        )
        .unwrap(),
    );
    let h = Hub::shared_replay(
        qp_session::ReplayHost::with_local_dir(init, &root(), Path::new("")).unwrap(),
    );
    let c: ReplayControl = serde_json::from_value(json!({"kind": "step", "n": 5})).unwrap();
    h.borrow_mut().host_control(&c).unwrap();
    h
}

fn send(h: &SharedHub, conn: &str, msg: &Value) -> Vec<Value> {
    h.borrow_mut().send(conn, &msg.to_string())
}

#[test]
fn an_entitled_agents_comment_never_reaches_a_public_view_reader() {
    let h = hub();
    for (c, p) in [("A", "claude"), ("g", "guest")] {
        let r = send(
            &h,
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":p,
            "surface":{"kind":"headless"}}),
        );
        assert_eq!(r[0]["type"], "welcome", "{r:?}");
    }
    h.borrow_mut().mark_agent_connection("A");
    send(
        &h,
        "g",
        &json!({"type":"subscribe","id":"cm","topic":{"kind":"comments"}}),
    );
    // a capsule the guest can see
    let r = send(
        &h,
        "g",
        &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
    );
    let cap = r[0]["data"]["capsules"][0]["id"]
        .as_str()
        .expect("a capsule")
        .to_owned();
    let mut texts = Vec::new();
    for (i, text) in ["Kapanış ⁹⁹,⁵⁸ oldu [1].", "SYN01'de bu hikâye sürüyor [1]."]
        .iter()
        .enumerate()
    {
        let out = send(
            &h,
            "A",
            &json!({"type":"command","id":format!("k{i}"),"ops":[{
            "op":"add_comment","about":{"capsule":cap},"lang":"tr","text":text,"cites":[cap]}]}),
        );
        let ack = out.iter().find(|l| l["id"] == json!(format!("k{i}")));
        if ack.is_some_and(|a| a["status"] == "applied") {
            texts.push(*text);
        }
        // nothing on the guest's subscription
        assert!(
            !out.iter()
                .any(|l| l["type"] == "notify" && l["sub"] == "cm"),
            "{out:?}"
        );
    }
    assert!(!texts.is_empty(), "at least one comment stored");
    // (audit B13) the writer and its person read the comment back
    send(
        &h,
        "o",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin",
        "surface":{"kind":"headless"}}),
    );
    for c in ["A", "o"] {
        let s = serde_json::to_string(&send(
            &h,
            c,
            &json!({"type":"query","id":"q",
            "query":{"kind":"list","collection":"comments"}}),
        ))
        .unwrap();
        assert!(s.contains(texts[0].trim_end_matches(" [1].")), "{c}: {s}");
    }
    for q in [
        json!({"kind":"document"}),
        json!({"kind":"list","collection":"comments"}),
        json!({"kind":"log","limit":100}),
        json!({"kind":"capsule","capsule":cap}),
    ] {
        let s = serde_json::to_string(&send(&h, "g", &json!({"type":"query","id":"q","query":q})))
            .unwrap();
        for t in &texts {
            let core = t.trim_end_matches(" [1].");
            assert!(!s.contains(core), "{q}: {s}");
        }
    }
}
