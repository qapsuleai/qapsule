//! Generates the R4 interaction transcripts (`fixtures/protocol/conformance/5[1-5]-input-*.json`,
//! `docs/PROTOCOL.md` §4.6, §7, §15): pixel gestures, opening between depths, evidence
//! for an entitled and an unentitled reader, and view state that never reaches the log.
//!
//! Like `make_capsule_transcripts`: the expected outputs are what the reference authority
//! (in process) answers, complete (welcome trimmed to its type and `seq`); `qp conform`
//! checks them in process, over `qp serve` and over MCP. Deterministic.
//!
//! ```text
//! cargo run -p qp-session --example make_input_transcripts            # check only
//! cargo run -p qp-session --example make_input_transcripts -- --write
//! ```

use std::path::{Path, PathBuf};

use qp_schema::protocol::SessionInit;
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

const SEED: u64 = 1;
const N: usize = 160;
/// Bars loaded at start: the engine has run over them (capsules exist).
const K: usize = 129;
/// The capsule the scenarios open (public price story, visible to every reader).
const CAP: &str = "cap.THYAO.breakout.down.124";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bars() -> Vec<Bar> {
    qp_source::synthetic_market(1, N, SEED)[0].bars.clone()
}

/// THYAO daily bars `0..K` bound on feed `bist`, the capsule engine on `px`, and a
/// candle view `main` of the bound series. Arkın (owner) and Ana (viewer) may read
/// everything; Can has no data entitlement.
fn init() -> Value {
    let b = bars();
    json!({
        "document": {
            "schema_version": "0", "id": "doc-input", "title": "Girdi",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}],
            "views": [{"id":"main","x":{"kind":"time"},
                       "panels":[{"id":0,"y":{"kind":"linear"}}],
                       "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"arkin","author":{"kind":"human","name":"Arkın"},"role":"owner"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": {"symbol":"THYAO","interval":"1d","feed":"bist"}, "bars": b[..K]}],
            "entitlements": {"host": ["*"], "arkin": ["*"], "ana": ["*"], "can": []}
        },
        "capsules": {"series": ["px"], "flow": [{"series": "px", "seed": SEED}]}
    })
}

struct In(&'static str, Value, Option<&'static str>);

fn send(conn: &'static str, msg: Value) -> In {
    In(conn, msg, None)
}

fn send_n(conn: &'static str, msg: Value, note: &'static str) -> In {
    In(conn, msg, Some(note))
}

fn hello(conn: &'static str, who: &str) -> In {
    send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}}),
    )
}

fn input(conn: &'static str, id: &str, g: Value, note: &'static str) -> In {
    send_n(conn, json!({"type":"input","id":id,"gesture":g}), note)
}

fn query(conn: &'static str, id: &str, q: Value) -> In {
    send(conn, json!({"type":"query","id":id,"query":q}))
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"]});
    }
    v
}

fn run(name: &str, description: &str, covers: &[&str], steps: Vec<In>) -> Value {
    let init_v = init();
    let init: SessionInit = serde_json::from_value(init_v.clone()).expect("init");
    let mut s = Session::new(init, &root()).expect("session");
    let log0 = s.log().len();
    let mut out_steps = Vec::new();
    for In(conn, msg, note) in steps {
        let lines = s.handle_raw(conn, &msg.to_string());
        let mut step = json!({"send": {"conn": conn, "msg": msg}});
        if let Some(n) = note {
            step["note"] = json!(n);
        }
        step["expect"] = Value::Array(
            lines
                .iter()
                .map(
                    |l| json!({"conn": l.conn, "msg": trim(serde_json::to_value(&l.msg).unwrap())}),
                )
                .collect(),
        );
        out_steps.push(step);
    }
    // Gestures and opening are presence: the log keeps its length from the start.
    assert_eq!(s.log().len(), log0, "{name}: an input reached the log");
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init_v,
        "steps": out_steps,
        "final": {"log_len": s.log().len()}
    })
}

fn tap_hit() -> Value {
    run(
        "51-input-tap-hit",
        "R4 (§4.6): pixel taps resolved in the core. On the candle view `main` (phone, frame 390x300 reported with the tap) a tap on a candle answers what is under the finger (the `hit` code) and that bar's values (`at`); a tap outside the plot answers the empty hit and no bar. A tap on a capsule's face in the feed (frame 358x140) opens it: the engine decides, the answer carries the presentation at capsule depth and the elements face and opened capsule share, with their boxes in both scenes. `screen` then shows the view state to an agent.",
        &["input", "tap_at", "hit", "open", "view-state", "screen"],
        vec![
            hello("p", "arkin"),
            input("p", "t1", json!({"kind":"tap_at","target":{"view":"main"},"x":300.0,"y":150.0,"width":390,"height":300}),
                "A tap on a candle: the bar under the finger and its values."),
            input("p", "t2", json!({"kind":"tap_at","target":{"view":"main"},"x":2.0,"y":5.0}),
                "Outside the plot (the remembered 390x300 frame): no panel, no bar, no item."),
            input("p", "t3", json!({"kind":"tap_at","target":{"capsule":CAP},"x":100.0,"y":40.0,"width":358,"height":140}),
                "A tap on a closed capsule's face opens it one depth (sentence → capsule)."),
            query("p", "s1", json!({"kind":"screen"})),
        ],
    )
}

fn pan_pinch() -> Value {
    run(
        "52-input-pan-pinch",
        "R4 (§4.6): drag and pinch in pixels. On a document view the engine turns pixels into bars with the scene's slot width and goes through `pan`/`zoom` (the connection viewport, `viewport` events to attention subscribers); the result equals the bar-space gesture. On an open capsule's chart the same rules move the view state's range. Dragging a face (not a chart) is `invalid`; a pinch factor of 0 is `invalid`.",
        &["input", "drag", "pinch", "viewport", "view-state", "errors"],
        vec![
            hello("p", "arkin"),
            send("p", json!({"type":"subscribe","id":"att","topic":{"kind":"attention"}})),
            input("p", "d1", json!({"kind":"drag","target":{"view":"main"},"dx":50.0,"width":390,"height":300}),
                "Finger 50 px right: older bars come into view (bars = round(-dx / slot))."),
            input("p", "z1", json!({"kind":"pinch","target":{"view":"main"},"factor":2.0,"x":200.0}),
                "Fingers apart around x = 200: half the bars, the bar under x stays in place."),
            input("p", "o1", json!({"kind":"open","capsule":CAP,"to":"chart","width":390,"height":400}),
                "Open the capsule straight to its marked chart."),
            input("p", "d2", json!({"kind":"drag","dx":40.0}),
                "No target: the current scene, the open capsule's chart; the view state's range moves."),
            input("p", "z2", json!({"kind":"pinch","factor":0.5}),
                "Fingers together: twice the bars, anchored at the last visible bar."),
            input("p", "e1", json!({"kind":"drag","target":{"capsule":"cap.THYAO.flow.up.90"},"dx":10.0}),
                "A capsule that is not open is a face: it does not pan."),
            input("p", "e2", json!({"kind":"pinch","target":{"view":"main"},"factor":0.0}),
                "Factor must be positive."),
        ],
    )
}

fn open_close() -> Value {
    run(
        "53-input-open-close",
        "R4 (§4.6, GRAMMAR §18.6): opening between depths with input. sentence → capsule → chart, back to capsule and to the feed. Each answer carries what to show at the new depth (presentation), the frame it was laid out at, the shared elements (glyph, marked bar, price trace, fact…) with their boxes before and after, and the view state. Errors: `close` with nothing open, `open` to the sentence, an unknown capsule.",
        &["input", "open", "close", "shared-elements", "view-state", "errors"],
        vec![
            hello("p", "arkin"),
            input("p", "x1", json!({"kind":"close"}), "Nothing is open yet."),
            input("p", "o1", json!({"kind":"open","capsule":CAP}),
                "One depth further than now: capsule."),
            input("p", "o2", json!({"kind":"open","capsule":CAP}),
                "Again: its marked chart. Shared with the capsule: glyph, price trace, marked bar."),
            input("p", "c1", json!({"kind":"close"}), "One back: capsule."),
            input("p", "c2", json!({"kind":"close","to":"sentence"}), "Back to the feed: nothing open."),
            input("p", "e1", json!({"kind":"open","capsule":CAP,"to":"sentence"}),
                "`open` does not go to the sentence."),
            input("p", "e2", json!({"kind":"open","capsule":"cap.nope"}), "Unknown capsule."),
        ],
    )
}

fn evidence() -> Value {
    run(
        "54-input-evidence",
        "R4 (§4.6, §20.2, K34): opening to the evidence with input. At chart depth a tap on an element of the evidence (the glyph over the fact bar) opens the evidence: the entitled reader gets the values (the `capsule` query's `evidence`); a tap on a plain candle only focuses the bar. A reader without the data entitlement opens the same capsule to its evidence and gets what it rests on and 'requires bist/THYAO' — no values, no chart.",
        &["input", "open", "evidence", "entitlement", "tap_at"],
        vec![
            hello("p", "arkin"),
            hello("c", "can"),
            input("p", "o1", json!({"kind":"open","capsule":CAP,"to":"chart","width":390,"height":400}),
                "Arkın opens the capsule to its chart."),
            input("p", "t1", json!({"kind":"tap_at","x":30.0,"y":300.0}),
                "A candle that is not evidence: the bar is focused, nothing opens."),
            input("p", "t2", json!({"kind":"tap_at","x":GLYPH_X,"y":GLYPH_Y}),
                "The glyph over the fact bar is evidence: the tap opens the evidence (values: Arkın is entitled)."),
            input("c", "o2", json!({"kind":"open","capsule":CAP,"to":"evidence"}),
                "Can (no data entitlement): the evidence says what it requires; no chart is drawn for him."),
        ],
    )
}

/// Where the glyph sits on the 390x400 chart of `CAP` (the `glyph` box of the opening to the chart: 262..294 x 36..68).
const GLYPH_X: f64 = 278.0;
const GLYPH_Y: f64 = 52.0;

fn presence() -> Value {
    let mut steps = vec![
        hello("p", "arkin"),
        hello("a", "ana"),
        send(
            "a",
            json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}),
        ),
    ];
    for i in 0..10 {
        let g = match i % 5 {
            0 => json!({"kind":"drag","target":{"view":"main"},"dx":-25.0}),
            1 => json!({"kind":"pinch","target":{"view":"main"},"factor":1.25,"x":180.0}),
            2 => json!({"kind":"open","capsule":CAP}),
            3 => json!({"kind":"tap_at","target":{"view":"main"},"x":250.0,"y":120.0}),
            _ => json!({"kind":"close","to":"sentence"}),
        };
        steps.push(send(
            "p",
            json!({"type":"input","id":format!("g{i}"),"gesture":g}),
        ));
    }
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"l","query":{"kind":"log","after_seq":2}}),
        "Ana watched `changes`: ten gestures, no change event, and no log entry after the two of the start (the engine's load).",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s","query":{"kind":"screen","principal":"arkin"}}),
        "The agent reads where Arkın looks (viewport, view state) with `screen`.",
    ));
    run(
        "55-input-presence",
        "R4 (§4.6, §7): view state is presence. Ten pixel gestures and open/close on one connection: no log entry, no `change` event to a `changes` subscriber, the log keeps its length (`final.log_len`); another principal reads the view state with `screen`.",
        &["input", "view-state", "presence", "log", "screen"],
        steps,
    )
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("json");
    s.push('\n');
    s
}

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let dir = root().join("fixtures/protocol/conformance");
    let mut stale = 0;
    for t in [tap_hit(), pan_pinch(), open_close(), evidence(), presence()] {
        let name = t["name"].as_str().unwrap().to_owned();
        let path = dir.join(format!("{name}.json"));
        let text = pretty(&t);
        let same = std::fs::read_to_string(&path).is_ok_and(|x| x == text);
        if write && !same {
            std::fs::write(&path, &text).expect("write");
            println!("wrote {}", path.display());
        } else if !same {
            stale += 1;
            println!("differs: {}", path.display());
        } else {
            println!("same: {}", path.display());
        }
    }
    if stale > 0 && !write {
        std::process::exit(1);
    }
}
