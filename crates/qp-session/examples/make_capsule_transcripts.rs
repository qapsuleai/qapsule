//! Generates the R3 capsule conformance transcripts 38–43
//! (`fixtures/protocol/conformance/38-*.json` …, `docs/PROTOCOL.md` §15, §20).
//!
//! Each scenario is written as messages and data events; the expected outputs are what
//! the reference authority (in process) answers, complete (welcome trimmed to its type and
//! `seq`). `qp conform` then checks them in process, over `qp serve` and over MCP.
//! Deterministic: running it twice writes the same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_capsule_transcripts            # check only
//! cargo run -p qp-session --example make_capsule_transcripts -- --write
//! ```

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

const SEED: u64 = 1;
const N: usize = 160;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bars() -> Vec<Bar> {
    qp_source::synthetic_market(1, N, SEED)[0].bars.clone()
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn principals() -> Value {
    json!([
        {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
        {"id":"arkin","author":{"kind":"human","name":"Arkın"},"role":"owner"},
        {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
        {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
        {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"}
    ])
}

/// The init: THYAO daily bars `0..k` on feed `bist`, the engine on `px` with the synthetic
/// licensed flow; `settled`: bars replacing the provider's in the settled source.
fn init(k: usize, settled: Option<Vec<Bar>>) -> Value {
    let b = bars();
    let mut data = json!({
        "feeds": [{"feed": feed(), "bars": b[..k]}],
        "entitlements": {
            "host": ["*"], "arkin": ["*"], "ana": ["*"],
            "bora": ["bist/THYAO"], "can": []
        }
    });
    if let Some(s) = settled {
        data["settled"] = json!({"feeds":[{"feed": feed(), "bars": s}], "verify_closed": false});
    }
    json!({
        "document": {
            "schema_version": "0", "id": "doc-capsules", "title": "Kapsül",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": principals(),
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": data,
        "capsules": {"series": ["px"], "flow": [{"series": "px", "seed": SEED}]}
    })
}

enum In {
    Send(&'static str, Value, Option<&'static str>),
    Data(Value, Option<&'static str>),
}

fn send(conn: &'static str, msg: Value) -> In {
    In::Send(conn, msg, None)
}

fn send_n(conn: &'static str, msg: Value, note: &'static str) -> In {
    In::Send(conn, msg, Some(note))
}

fn hello(conn: &'static str, who: &str, lang: Option<&str>) -> In {
    let mut surface = json!({"kind":"headless"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    )
}

fn sub(conn: &'static str, id: &str, topic: Value) -> In {
    send(conn, json!({"type":"subscribe","id":id,"topic":topic}))
}

fn query(conn: &'static str, id: &str, q: Value) -> In {
    send(conn, json!({"type":"query","id":id,"query":q}))
}

/// Data event: bar `i` of the feed (`seq` = position after the initial `k` bars).
fn bar_ev(k: usize, i: usize, b: &Bar, note: Option<&'static str>) -> In {
    In::Data(
        json!({"feed": feed(), "seq": i - k + 1, "change": {"kind":"bar","bar": b}}),
        note,
    )
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"]});
    }
    v
}

fn run(
    name: &str,
    description: &str,
    covers: &[&str],
    init_v: Value,
    steps: Vec<In>,
    final_doc: Option<Value>,
) -> Value {
    let init: SessionInit = serde_json::from_value(init_v.clone()).expect("init");
    let mut s = Session::new(init, &root()).expect("session");
    let mut out_steps = Vec::new();
    for st in steps {
        let (mut step, lines) = match st {
            In::Send(conn, msg, note) => {
                let lines = s.handle_raw(conn, &msg.to_string());
                let mut step = json!({"send": {"conn": conn, "msg": msg}});
                if let Some(n) = note {
                    step["note"] = json!(n);
                }
                (step, lines)
            }
            In::Data(ev, note) => {
                let e: DataEvent = serde_json::from_value(ev.clone()).expect("event");
                let lines = s.data_event(&e).expect("data event applies");
                let mut step = json!({"data": ev});
                if let Some(n) = note {
                    step["note"] = json!(n);
                }
                (step, lines)
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
        out_steps.push(step);
    }
    let mut fin = json!({"log_len": s.log().len()});
    if let Some(d) = final_doc {
        fin["document"] = d;
    }
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init_v,
        "steps": out_steps,
        "final": fin
    })
}

fn t38() -> Value {
    let b = bars();
    let k = 121;
    let mut steps = vec![
        hello("a", "ana", None),
        sub("a", "k", json!({"kind":"capsules","all":true})),
    ];
    for i in k..=128 {
        steps.push(bar_ev(k, i, &b[i], None));
    }
    steps.push(query("a", "q1", json!({"kind":"capsules","since_bar":120})));
    steps.push(query(
        "a",
        "q2",
        json!({"kind":"capsule","capsule":"cap.THYAO.breakout.down.124"}),
    ));
    steps.push(query(
        "a",
        "q3",
        json!({"kind":"ledger","meaning":"breakout"}),
    ));
    steps.push(query("a", "q4", json!({"kind":"ledger","by_kind":true})));
    run(
        "38-capsule-lifecycle",
        "R3 (§20): the authority runs the capsule engine on the bound THYAO series; every bar that becomes final goes through the detectors and the engine and is written as one engine entry. Topic `capsules` shows the transitions in order: an invalidated breakout (barrier stop), a licensed flow story born and invalidated, a breakout born and confirmed by volume and flow (new families), strengthened, realized; an RSI extreme born. Then the `capsules` list (most important first), one capsule opened to its evidence (`capsule`: history with reasons, facts, sentences, ledger, glyph) and the ledger by key and by kind.",
        &["capsules", "capsule", "ledger", "topic-capsules", "engine-entry", "lifecycle"],
        init(k, None),
        steps,
        None,
    )
}

fn t39() -> Value {
    let b = bars();
    let k = 90;
    let mut steps = vec![
        hello("a", "ana", None),
        hello("b", "bora", None),
        hello("c", "can", None),
        sub("a", "k", json!({"kind":"capsules","all":true})),
        sub("b", "k", json!({"kind":"capsules","all":true})),
        sub("c", "k", json!({"kind":"capsules","all":true})),
    ];
    for i in k..=93 {
        steps.push(bar_ev(k, i, &b[i], None));
    }
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"e1","query":{"kind":"capsule","capsule":"cap.THYAO.flow.up.90"}}),
        "Ana holds the flow entitlement: the licensed story opens to its values.",
    ));
    steps.push(send_n(
        "b",
        json!({"type":"query","id":"e2","query":{"kind":"capsule","capsule":"cap.THYAO.flow.up.90"}}),
        "Bora may read the bars but not the licensed flow: the licensed story does not exist for him (same answer as an unknown id, no existence oracle); he got the server comments instead.",
    ));
    steps.push(query("b", "e3", json!({"kind":"capsules","since_bar":85})));
    steps.push(send_n(
        "c",
        json!({"type":"query","id":"e4","query":{"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}}),
        "Can has no data entitlement: the public story opens to what its evidence rests on and the entitlement it requires (K34); no values, no digest, no digit in the sentence.",
    ));
    steps.push(query("b", "e5", json!({"kind":"ledger","by_kind":true})));
    run(
        "39-capsule-entitlement",
        "R3 (§20, K33, K34, M6): three readers of the same capsules. Ana (entitled to everything) sees the licensed institutional-flow story with its numbers. Bora (bars `bist/THYAO`, no flow) never receives the licensed story, its fact or its time: only the trust path's server comments (`trust: server`, mühür) and the license-independent ledger. Can (no data entitlement) gets the public stories with digit-free sentences and opens a capsule to 'requires bist/THYAO' and what the evidence rests on.",
        &["capsule", "capsules", "topic-capsules", "entitlement", "server-comment", "muhur", "ledger"],
        init(k, None),
        steps,
        None,
    )
}

fn t40() -> Value {
    let b = bars();
    let k = 92;
    let mut steps = vec![hello("t", "ana", None), hello("e", "ana", Some("en"))];
    for i in k..=95 {
        steps.push(bar_ev(k, i, &b[i], None));
    }
    steps.push(send_n(
        "t",
        json!({"type":"query","id":"tr","query":{"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}}),
        "K26: the state began 11 bars ago and ended now: past tense and 'artık … altında değil'.",
    ));
    steps.push(send_n(
        "e",
        json!({"type":"query","id":"en","query":{"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}}),
        "The same in English for a connection that asked for it (`surface.lang: en`): 'no longer below'.",
    ));
    steps.push(query(
        "t",
        "x",
        json!({"kind":"capsules","stages":["expired"],"since_bar":90}),
    ));
    run(
        "40-capsule-k26-sentence",
        "R3 (K26, §20): a capsule's sentence follows the time of its fact and its validity today. A short-average state that began 11 bars ago and ended on the last bar is told in the past with 'artık … değil' / 'no longer'; a divergence whose 15-bar life ran out is expired. Turkish by default, English for a connection whose surface asks for it.",
        &["capsule", "capsules", "k26", "lang", "expired"],
        init(k, None),
        steps,
        None,
    )
}

fn t41() -> Value {
    let b = bars();
    let k = 124;
    let mut steps = vec![
        hello("a", "ana", None),
        sub("a", "k", json!({"kind":"capsules"})),
        send_n(
            "a",
            json!({"type":"command","id":"s1","ops":[{"op":"set_audience","audience":{"group":"future"}}]}),
            "Ana (a viewer) sets her own preferences: the 'future investor' group, whose default is `important` (tier 1 only).",
        ),
        bar_ev(k, 124, &b[124], None),
        bar_ev(k, 125, &b[125], Some("Only the tier-1 breakout reaches her; the tier-2 continuation born on the same bar does not.")),
        send_n(
            "a",
            json!({"type":"command","id":"s2","ops":[{"op":"set_audience","audience":{"group":"future","positions":{"THYAO":100}}}]}),
            "She holds THYAO now: stories about a position pass any level (`relevant`).",
        ),
        bar_ev(k, 126, &b[126], None),
        bar_ev(k, 127, &b[127], Some("A tier-3 RSI extreme passes because of the position.")),
        send_n(
            "a",
            json!({"type":"command","id":"s3","ops":[{"op":"set_sensitivity","sensitivity":"balanced"}]}),
            "Only the level changes; the position stays.",
        ),
        send_n(
            "b",
            json!({"type":"hello","protocol":"qp/0","principal":"bora","surface":{"kind":"headless"}}),
            "Bora, a viewer, may not set another person's preferences.",
        ),
        send(
            "b",
            json!({"type":"command","id":"s4","ops":[{"op":"set_sensitivity","principal":"ana","sensitivity":"all"}]}),
        ),
        bar_ev(k, 128, &b[128], None),
        query("a", "q", json!({"kind":"capsules","audience":true,"since_bar":124})),
    ];
    let _ = &mut steps;
    run(
        "41-capsule-sensitivity",
        "R3 (K31, §20): sensitivity and the position override. `set_audience` / `set_sensitivity` write a person's own preferences through the command path (another person's only by an owner); the topic `capsules` filters by them: with the future-investor default (`important`) only tier 1 passes, a story about a held position passes any level. The engine writes many entries and never touches the preferences (final document).",
        &["set_audience", "set_sensitivity", "topic-capsules", "capsules", "sensitivity", "position-override"],
        init(k, None),
        steps,
        Some(json!({"audiences":{"ana":{"group":"future","sensitivity":"balanced","positions":{"THYAO":100.0}}}})),
    )
}

fn t42() -> Value {
    let b = bars();
    let k = 143;
    let mut low = b[143].clone();
    // The bar first closes under the 20-bar high: no breakout.
    let level = b[123..143].iter().map(|x| x.h).fold(f64::MIN, f64::max);
    low.c = (level - 0.05).min(b[143].c);
    low.h = low.h.max(low.c);
    low.l = low.l.min(low.c);
    let steps = vec![
        hello("a", "ana", None),
        sub("a", "k", json!({"kind":"capsules","all":true})),
        sub("a", "ch", json!({"kind":"changes"})),
        In::Data(
            json!({"feed": feed(), "seq": 1, "change": {"kind":"bar","bar": b[143]}}),
            Some("Bar 143 opens and trades above the 20-bar high: a breakout candidate on a device that watches it (replica, `Replica::candidates`); the authority says nothing, the bar is not final."),
        ),
        In::Data(
            json!({"feed": feed(), "seq": 2, "change": {"kind":"patch","t": b[143].t, "fields": {"c": low.c, "h": low.h.max(b[143].h), "l": low.l}}}),
            Some("The price falls back under the level within the bar: the candidate disappears. Still nothing from the authority."),
        ),
        In::Data(
            json!({"feed": feed(), "seq": 3, "change": {"kind":"bar","bar": b[144]}}),
            Some("Bar 144 makes bar 143 final: one engine entry, and no breakout story was ever born (no trace in the log or the ledger)."),
        ),
        query("a", "q", json!({"kind":"capsules","since_bar":140})),
        query("a", "l", json!({"kind":"ledger","meaning":"breakout"})),
    ];
    run(
        "42-capsule-candidate",
        "R3 (K28, A16, §20): a candidate is never the authority's. A breakout that forms inside the live bar and disappears before the close leaves no trace: no message while the bar is live, and at the close one engine entry without that story; the ledger is unchanged.",
        &["candidate", "topic-capsules", "changes", "ledger"],
        init(k, None),
        steps,
        None,
    )
}

fn t43() -> Value {
    let b = bars();
    let k = 150;
    // The feed's bar 152 is a bad print: a spike above the 20-bar high (a false breakout).
    let level = b[132..152].iter().map(|x| x.h).fold(f64::MIN, f64::max);
    let mut spike = b[152].clone();
    spike.c = level * 1.04;
    spike.h = spike.h.max(spike.c);
    let steps = vec![
        hello("a", "ana", None),
        hello("r", "bora", None),
        sub("a", "k", json!({"kind":"capsules","all":true})),
        bar_ev(k, 150, &b[150], None),
        bar_ev(k, 151, &b[151], None),
        In::Data(
            json!({"feed": feed(), "seq": 3, "change": {"kind":"bar","bar": spike}}),
            Some("The feed delivers a bad print for bar 152."),
        ),
        In::Data(
            json!({"feed": feed(), "seq": 4, "change": {"kind":"bar","bar": b[153]}}),
            Some("Bar 152 is final: the authority's data says breakout; the story is born."),
        ),
        send_n(
            "r",
            json!({"type":"dispute","id":"d1","claim":{"kind":"capsule","capsule":"cap.THYAO.breakout.up.152","finding":"fnd.THYAO.breakout.up.152"},"t": b[152].t}),
            "A replica's clean, refetched bar 152 has no breakout: it disputes. The authority asks its provider's settled bars, finds bar 152 corrected, re-runs the detectors: the breakout fact is gone. Upheld: the capsule is retracted (K37, `dispute_upheld`), never in the ledger.",
        ),
        send("r", json!({"type":"dispute","id":"d2","claim":{"kind":"capsule","capsule":"cap.THYAO.breakout.up.152","finding":"fnd.THYAO.breakout.up.152"},"t": b[152].t})),
        query("a", "q", json!({"kind":"capsule","capsule":"cap.THYAO.breakout.up.152"})),
        query("a", "l", json!({"kind":"ledger","meaning":"breakout"})),
    ];
    run(
        "43-capsule-retract",
        "R3 (K37, §19.9, §20): retraction of a story whose data proved wrong. A bad print makes a false breakout; a replica disputes the capsule; the settled bar contradicts it: upheld, the story goes to `retracted` (state `invalidated`), the transition reason is `dispute_upheld`, and the ledger never counts it. A repeated dispute is `duplicate`.",
        &["dispute", "retracted", "capsule", "ledger", "topic-capsules"],
        init(k, Some(vec![b[152].clone()])),
        steps,
        None,
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
    for t in [t38(), t39(), t40(), t41(), t42(), t43()] {
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
