//! Transcripts 74 (drawing) and 77 (replay).

use qp_schema::protocol::SessionInit;
use qp_session::{Replica, Session};
use serde_json::{json, Value};

use super::{add, cmd, hello, q, reply, root, run, thyao, transcript, In, ABOVE, SMA};

fn desk(conn: &'static str, p: &str, note: &'static str) -> In {
    In::Send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"desktop"}}),
        note,
    )
}

fn draw_ops() -> (Value, Value) {
    let mut p2 = add("p2", ABOVE, false);
    p2["show"] = json!({"view":"main","panel":0});
    (json!([add("p1", SMA, true)]), json!([p2]))
}

/// A point of view `main` whose hit is an item of layer `layer` (searched down x = 600).
fn point_on(layer: &str) -> (u32, u32) {
    let (p1, p2) = draw_ops();
    let si: SessionInit = serde_json::from_value(thyao()).unwrap();
    let mut s = Session::new(si, &root()).unwrap();
    let send = |s: &mut Session, m: Value| {
        let out = s.handle_raw("a", &m.to_string());
        serde_json::to_value(&out[0].msg).unwrap()
    };
    send(
        &mut s,
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}),
    );
    send(&mut s, json!({"type":"command","id":"c1","ops":p1}));
    send(&mut s, json!({"type":"command","id":"c2","ops":p2}));
    for y in 0..1000 {
        let r = send(
            &mut s,
            json!({"type":"query","id":"h","query":{"kind":"hit","view":"main","x":600,"y":y}}),
        );
        if r["data"]["hit"]["item"]["layer"] == layer {
            return (600, y);
        }
    }
    panic!("no point on layer {layer}");
}

pub fn t74() -> Value {
    let (p1, p2) = draw_ops();
    // Layer ids follow the series ids in `ack.created`; found by a first run.
    let (pre, _) = run(
        &thyao(),
        vec![desk("a", "arkin", ""), cmd("a", "c1", None, p1.clone(), "")],
    );
    let layer = reply(&pre, "c1")["created"][2].as_str().unwrap().to_owned();
    let (x, y) = point_on(&layer);
    let steps = vec![
        desk("a", "arkin", "A person on a desktop surface."),
        cmd("a", "c1", None, p1, "add_program with `show`: a line for the series output and a bar layer for the event output, in a new panel, labelled with the series ids."),
        cmd("a", "c2", None, p2, "`show.panel: 0`: the level drawn on the price panel."),
        q("a", "q1", json!({"kind":"get","id":"main"}), "The view's layers."),
        q("a", "q2", json!({"kind":"screen"}), "The screen reading names the program series in their panels."),
        q("a", "q3", json!({"kind":"hit","view":"main","x":x,"y":y}), "A point on the program's line: the hit's item is its layer, data its series."),
    ];
    let init = thyao();
    let (out, s) = run(&init, steps);
    let shot = serde_json::to_string(&reply(&out, "q2")["data"]).unwrap();
    assert!(
        shot.contains("p1.avg") && shot.contains("p2.level"),
        "{shot}"
    );
    assert_eq!(
        reply(&out, "q3")["data"]["hit"]["item"]["layer"],
        json!(layer)
    );
    transcript(
        "74-program-draw",
        "RL1 (PS §1.4): add_program's `show` adds a layer per output (series → line, event → bar; a new panel by default, or `show.panel`); the screen reading and the point query hit see the program series.",
        &["add_program", "show", "screen", "hit", "program"],
        init,
        out,
        &s,
    )
}

fn steps77() -> Vec<In> {
    vec![
        hello("a", "arkin", "A person."),
        cmd(
            "a",
            "c1",
            None,
            json!([add("p1", SMA, true)]),
            "A program, drawn.",
        ),
        cmd(
            "a",
            "c2",
            None,
            json!([{"op":"set_program_params","program":"p1","params":{"len":12}}]),
            "New values.",
        ),
        cmd(
            "a",
            "c3",
            None,
            json!([add("p2", ABOVE, false)]),
            "A second program.",
        ),
        cmd(
            "a",
            "c4",
            None,
            json!([{"op":"remove_program","program":"p2"}]),
            "Removed.",
        ),
        cmd(
            "a",
            "c5",
            None,
            json!([{"op":"undo","seq":4}]),
            "Undo brings it back.",
        ),
        cmd(
            "a",
            "c6",
            None,
            json!([{"op":"undo","seq":2}]),
            "Undo the new values: `len` 5 again.",
        ),
        q(
            "a",
            "q1",
            json!({"kind":"study","series":"p1.avg"}),
            "The values after the undo.",
        ),
        q(
            "a",
            "q2",
            json!({"kind":"log"}),
            "The log: only `put_series`/`insert_layer`/… changes, the IR inline, no text.",
        ),
    ]
}

/// A replica following the same session sees the same document and draws the same view.
fn replica_agrees() {
    let si: SessionInit = serde_json::from_value(thyao()).unwrap();
    let mut s = Session::new(si, &root()).unwrap();
    let mut r = Replica::with_memory("viewer", qp_source::MemoryProvider::new());
    let deliver =
        |s: &mut Session, r: &mut Replica, lines: Vec<qp_schema::protocol::OutboundLine>| {
            let mut q: std::collections::VecDeque<_> = lines.into();
            while let Some(l) = q.pop_front() {
                if l.conn != "r" {
                    continue;
                }
                r.on_message(&l.msg);
                for m in r.take_outbox() {
                    q.extend(s.handle_raw("r", &m.to_string()));
                }
            }
        };
    let hi = json!({"type":"hello","protocol":"qp/0","principal":"viewer","surface":{"kind":"headless"}});
    let lines = s.handle_raw("r", &hi.to_string());
    deliver(&mut s, &mut r, lines);
    for st in steps77() {
        if let In::Send(conn, msg, _) = st {
            let lines = s.handle_raw(conn, &msg.to_string());
            deliver(&mut s, &mut r, lines);
        }
    }
    assert_eq!(
        r.document_digest().as_deref(),
        Some(qp_session::document_digest(s.document()).as_str()),
        "replica document"
    );
    let mine = qp_session::view_digest(&mut s, "main", 800, 400).unwrap();
    assert_eq!(
        r.view_digest("main", 800, 400).unwrap(),
        mine,
        "replica view"
    );
}

pub fn t77() -> Value {
    let init = thyao();
    let (out, s) = run(&init, steps77());
    assert_eq!(
        reply(&out, "c5")["status"],
        "applied",
        "{}",
        reply(&out, "c5")
    );
    assert_eq!(
        reply(&out, "c6")["status"],
        "applied",
        "{}",
        reply(&out, "c6")
    );
    let log = serde_json::to_string(&reply(&out, "q2")["data"]).unwrap();
    assert!(
        log.contains("qp-ir/0") && !log.contains("sma(close"),
        "IR only in the log"
    );
    replica_agrees();
    transcript(
        "77-program-replay",
        "RL1 (PS §3.1): program ops write existing changes only; undo of remove_program and of set_program_params; the log replays to the same document bytes (checked by the runner at the end), program series included; a replica following the session has the same document and view digests (checked by the generator).",
        &["replay", "undo", "program", "log"],
        init,
        out,
        &s,
    )
}
