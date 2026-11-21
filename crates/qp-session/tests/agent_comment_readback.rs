//! (R5B audit B13) In the R5B harness session (a replay host; people with audiences, so
//! every reader has a projection), a stored agent comment is read back by its writer and
//! its person through `list`, `get`, `capsule` and `document` — also when it names an
//! instrument (`SYN02`: the symbol's digits are no number).

use std::path::PathBuf;

use qp_schema::protocol::SessionInit;
use qp_session::ReplayHost;
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn first(host: &mut ReplayHost, conn: &str, msg: &Value) -> Value {
    host.handle_raw(conn, &msg.to_string())
        .into_iter()
        .find(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .unwrap()
}

#[test]
fn the_writer_and_its_person_read_the_comment_back() {
    let r = root();
    let init: SessionInit =
        qp_session::read_json(&r.join("rounds/R5B/experiments/harness/session/init.r5b.json"))
            .unwrap();
    let mut h = ReplayHost::new(init, &r).unwrap();
    for (c, p) in [("A", "claude"), ("o", "arkin")] {
        let w = first(
            &mut h,
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":p,
            "surface":{"kind":"headless"}}),
        );
        assert_eq!(w["type"], "welcome", "{w}");
    }
    h.session_mut().mark_agent_connection("A");
    let cap = "cap.SYN02.breakout.up.151";
    // (B14) a run of number words is fine for a reader who sees everything
    let text = "SYN02 yukarı kırılım sürüyor, doksan dokuz gibi bir sözcük de olsa [1].";
    let ack = first(
        &mut h,
        "A",
        &json!({"type":"command","id":"k","ops":[{"op":"add_comment",
        "about":{"capsule":cap},"lang":"tr","text":text,"cites":[cap]}]}),
    );
    assert_eq!(ack["status"], "applied", "{ack}");
    let id = ack["created"][0].as_str().unwrap().to_owned();
    for c in ["A", "o"] {
        for q in [
            json!({"kind":"list","collection":"comments"}),
            json!({"kind":"get","id":id}),
            json!({"kind":"capsule","capsule":cap}),
            json!({"kind":"document"}),
            json!({"kind":"log","limit":50}),
        ] {
            let r = first(&mut h, c, &json!({"type":"query","id":"q","query":q}));
            assert!(r.to_string().contains(text), "{c} {q}: {r}");
        }
    }
}
