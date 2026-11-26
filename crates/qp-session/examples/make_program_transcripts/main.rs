//! Generates the RL1 conformance transcripts 71–77 (`docs/PROGRAMS-SESSION.md` §4):
//! programs in a session — add / check / try / drawing / events / an agent's proposal /
//! replay.
//!
//! The expected outputs are the reference authority's answers (welcome trimmed to its type
//! and `seq`). `qp conform` checks them in process, over `qp serve` and over MCP; at the
//! end of each transcript it replays the log and compares the document's bytes. The
//! generator asserts what the transcripts claim (events counted, the try = the document's
//! event series, a replica's digests) before it writes. Deterministic: two runs write the
//! same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_program_transcripts            # check only
//! cargo run -p qp-session --example make_program_transcripts -- --write
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A program with a parameter, a series and an event output (PR §12.5 style).
const SMA: &str = "param len int = 5 in 2..50\nm = sma(close, len)\nseries avg = m\nevent up = close > m and defined(m)\n";
/// A signal and a state output (try only).
const SIGNAL_STATE: &str = "r = rsi(close, 14)\n\
entry = cooldown(cross_below(r, 30), 12)\n\
signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source \"rsi_dip\" \"1\"\n\
state zone { lvl = bucket(r, [30, 50, 70]) }\n";
/// Four signals open on every bar for 500 bars, behind a 640-bar `stdev`: over 20 000
/// bars its try work passes `program.try.max_work` (50 000 000), and its live cost alone
/// is over the per-owner live limit (`W_LIVE_BUDGET`) although it checks.
const HEAVY_TRY: &str = "e = close > 0 or stdev(close, 640) > 0\n\
signal a = e side al target 900% stop 90% max_bars 500 source \"a\" \"1\"\n\
signal b = e side al target 900% stop 90% max_bars 500 source \"b\" \"1\"\n\
signal c = e side al target 900% stop 90% max_bars 500 source \"c\" \"1\"\n\
signal d = e side al target 900% stop 90% max_bars 500 source \"d\" \"1\"\n";
/// A series output costing ≈ 600 per bar: three fit the live limit (2000), four do not.
const HEAVY: &str = "series s = stdev(close, 600)\n";
/// Close above a level (an event output and the level as a series).
const ABOVE: &str =
    "param lv float = 105 in 0..1000\nevent up = close > lv\nseries level = lv + close * 0\n";

enum In {
    Send(&'static str, Value, &'static str),
    Data(Value, &'static str),
}

fn hello(conn: &'static str, p: &str, note: &'static str) -> In {
    In::Send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
        note,
    )
}

fn cmd(conn: &'static str, id: &str, mode: Option<&str>, ops: Value, note: &'static str) -> In {
    let mut m = json!({"type":"command","id":id,"ops":ops});
    if let Some(md) = mode {
        m["mode"] = json!(md);
    }
    In::Send(conn, m, note)
}

fn q(conn: &'static str, id: &str, query: Value, note: &'static str) -> In {
    In::Send(conn, json!({"type":"query","id":id,"query":query}), note)
}

fn sub(conn: &'static str, id: &str, topic: Value, note: &'static str) -> In {
    In::Send(
        conn,
        json!({"type":"subscribe","id":id,"topic":topic}),
        note,
    )
}

fn clock(now: i64, note: &'static str) -> In {
    In::Send("h", json!({"type":"clock","now":now}), note)
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"]});
    }
    v
}

/// Runs `steps` on a session of `init`: the transcript's steps with their outputs, and the
/// session at the end.
fn run(init: &Value, steps: Vec<In>) -> (Vec<Value>, Session) {
    let si: SessionInit = serde_json::from_value(init.clone()).expect("init");
    let mut s = Session::new(si, &root()).expect("session");
    let mut out = Vec::new();
    for st in steps {
        let (mut step, lines) = match st {
            In::Send(conn, msg, note) => {
                let lines = s.handle_raw(conn, &msg.to_string());
                (
                    json!({"send": {"conn": conn, "msg": msg}, "note": note}),
                    lines,
                )
            }
            In::Data(ev, note) => {
                let e: DataEvent = serde_json::from_value(ev.clone()).expect("event");
                let lines = s.data_event(&e).expect("data event applies");
                (json!({"data": ev, "note": note}), lines)
            }
        };
        step["expect"] = Value::Array(
            lines
                .iter()
                .map(
                    |l| json!({"conn": l.conn, "msg": trim(serde_json::to_value(&l.msg).unwrap())}),
                )
                .collect(),
        );
        out.push(step);
    }
    // Every transcript ends with a log that replays to the same bytes.
    let r = qp_ops::replay(s.initial_document(), s.log()).expect("replay");
    assert_eq!(
        serde_json::to_vec(&r).unwrap(),
        serde_json::to_vec(s.document()).unwrap(),
        "replay is byte-equal"
    );
    (out, s)
}

fn transcript(
    name: &str,
    description: &str,
    covers: &[&str],
    init: Value,
    out: Vec<Value>,
    s: &Session,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init,
        "steps": out,
        "final": {"log_len": s.log().len()}
    })
}

/// The reply to the step whose message `id` is `id` (its first output).
fn reply<'a>(out: &'a [Value], id: &str) -> &'a Value {
    out.iter()
        .find(|st| st["send"]["msg"]["id"] == id)
        .map(|st| &st["expect"][0]["msg"])
        .unwrap_or_else(|| panic!("no step {id}"))
}

/// The `condition_met` notifications among a step's outputs.
fn met(step: &Value) -> usize {
    step["expect"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["msg"]["event"]["kind"] == "condition_met")
        .count()
}

/// `thyao` with the R5B agent policy; `scout` (agent) works for `arkin`.
fn thyao() -> Value {
    let mut i: Value =
        qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).expect("init");
    for p in i["principals"].as_array_mut().unwrap() {
        if p["id"] == "scout" {
            p["on_behalf"] = json!("arkin");
        }
    }
    i["agent_policy"] = json!("r5b");
    i
}

/// The canonical IR of program text `text` (through `check_program`).
fn ir(text: &str) -> Value {
    let si: SessionInit = serde_json::from_value(thyao()).unwrap();
    let mut s = Session::new(si, &root()).unwrap();
    let hi =
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"headless"}});
    s.handle_raw("a", &hi.to_string());
    let m = json!({"type":"query","id":"q","query":{"kind":"check_program","source":text}});
    let out = s.handle_raw("a", &m.to_string());
    let v = serde_json::to_value(&out[0].msg).unwrap();
    assert_eq!(v["data"]["ok"], true, "{v}");
    v["data"]["ir"].clone()
}

fn add(id: &str, text: &str, show: bool) -> Value {
    let mut op = json!({"op":"add_program","id":id,"program":ir(text),"inputs":["px"]});
    if show {
        op["show"] = json!({"view":"main"});
    }
    op
}

mod draw;
mod event;
mod parts;

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let all = [
        ("71-program-add", parts::t71()),
        ("72-program-check", parts::t72()),
        ("73-program-try", parts::t73()),
        ("74-program-draw", draw::t74()),
        ("75-program-event", event::t75()),
        ("76-program-agent-proposal", parts::t76()),
        ("77-program-replay", draw::t77()),
    ];
    let mut differ = 0;
    for (name, t) in all {
        let path = root().join(format!("fixtures/protocol/conformance/{name}.json"));
        let mut text = serde_json::to_string_pretty(&t).expect("json");
        text.push('\n');
        let same = std::fs::read_to_string(&path).is_ok_and(|x| x == text);
        if write {
            if !same {
                std::fs::write(&path, &text).expect("write");
            }
            println!("{name}: {}", if same { "unchanged" } else { "written" });
        } else {
            println!("{name}: {}", if same { "same" } else { "DIFFERS" });
            if !same {
                differ += 1;
            }
        }
    }
    if differ > 0 {
        std::process::exit(1);
    }
}
