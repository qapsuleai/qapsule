//! Replica and viewer: the authority ↔ replica tape (`qp_session::tape`, the `Replica`
//! following the authority's messages, in step and lagging; the viewer's open slice),
//! and a replay host with two viewer connections of different principals (subscribe,
//! `visible`, open/close, replay steps, `viewer_flush` through the host).

use serde_json::json;

use super::order::Run;
use super::rig::Rig;
use super::snap::{jdigest, Book};

#[test]
fn charac_session_replica_tape() {
    let mut book = Book::new("replica-tape");
    for lag in [0, 3, 25] {
        let o = qp_session::tape::run(lag).unwrap();
        let v: Vec<String> = o.violations.iter().map(|v| format!("{v:?}")).collect();
        book.put(
            &format!("tape.lag{lag}"),
            &jdigest(&o.tape),
            &format!(
                "steps={} differing={} final_claims={} shown_final={} shown={:?} violations=[{}]",
                o.tape.steps.len(),
                o.differing_steps,
                o.final_claims,
                o.shown_final,
                o.shown_counts,
                v.join(";")
            ),
        );
        if let Some(last) = o.tape.steps.last() {
            book.put(
                &format!("tape.lag{lag}.last"),
                &jdigest(&last.expect),
                &format!("doc={} view={}", last.expect.doc, last.expect.view),
            );
        }
    }
    for lag in [0, 25] {
        let o = qp_session::tape::run_viewer(lag).unwrap();
        book.note(
            &format!("viewer.lag{lag}"),
            &format!(
                "steps={} differing={} final_claims={} shown_final={} foreign={} shown={:?} violations={}",
                o.steps,
                o.differing_steps,
                o.final_claims,
                o.shown_final,
                o.foreign_steps,
                o.shown_counts,
                o.violations.len()
            ),
        );
    }
    book.check();
}

#[test]
fn charac_session_viewer_two_readers() {
    let mut book = Book::new("viewer");
    let mut r = Run::new(&mut book, "v", Rig::of("59-viewer"));
    r.hello("a", "arkin", "phone");
    r.hello("g", "guest", "phone");
    r.hello("s", "scout", "headless");
    r.send(
        "seek",
        "a",
        json!({"type":"replay","id":"r1","control":{"kind":"seek","step":100}}),
    );
    r.sub("a", "v", json!({"kind":"viewer","limit":5}));
    r.sub("g", "v", json!({"kind":"viewer","limit":3,"width":320}));
    r.sub("s", "v", json!({"kind":"viewer"}));
    r.sub("a", "ka", json!({"kind":"capsules","all":true}));
    r.sub("g", "ch", json!({"kind":"changes"}));
    let caps = ["cap.SYN01.breakout.down.30", "cap.SYN02.breakout.up.48"];
    r.send(
        "vis_a",
        "a",
        json!({"type":"visible","id":"x1","sub":"v","visible":caps}),
    );
    r.send(
        "vis_g",
        "g",
        json!({"type":"visible","id":"x2","sub":"v","visible":[caps[1]]}),
    );
    r.send("open_a", "a", json!({"type":"input","id":"o1","gesture":{"kind":"open","capsule":caps[0],"to":"capsule","width":390,"height":300}}));
    r.send(
        "step9",
        "a",
        json!({"type":"replay","id":"r2","control":{"kind":"step","n":9}}),
    );
    r.send(
        "step_by_guest",
        "g",
        json!({"type":"replay","id":"r3","control":{"kind":"step","n":3}}),
    );
    r.send(
        "close_a",
        "a",
        json!({"type":"input","id":"c1","gesture":{"kind":"close","width":390,"height":300}}),
    );
    r.send(
        "vis_none",
        "a",
        json!({"type":"visible","id":"x3","sub":"v","visible":[]}),
    );
    r.send(
        "vis_bad",
        "g",
        json!({"type":"visible","id":"x4","sub":"nope","visible":[]}),
    );
    r.send(
        "unsub_g",
        "g",
        json!({"type":"unsubscribe","id":"u","sub":"v"}),
    );
    r.send(
        "step5",
        "a",
        json!({"type":"replay","id":"r4","control":{"kind":"step","n":5}}),
    );
    r.query("q_caps", "g", json!({"kind":"capsules"}));
    r.finish();
    book.check();
}
