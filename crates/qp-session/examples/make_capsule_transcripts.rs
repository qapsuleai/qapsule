//! Generates the R3 capsule conformance transcripts 38–44 and the R4/T3 (K42 and R3
//! ergonomics) transcripts 45–48, and the R4/S1 transcript 57 (selector `withheld`)
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
    Raw(&'static str, Value, Option<&'static str>),
    Data(Value, Option<&'static str>),
}

fn send(conn: &'static str, msg: Value) -> In {
    In::Send(conn, msg, None)
}

fn send_n(conn: &'static str, msg: Value, note: &'static str) -> In {
    In::Send(conn, msg, Some(note))
}

/// A message the schema rejects, sent as a raw line (`send_raw`).
fn send_raw_n(conn: &'static str, msg: Value, note: &'static str) -> In {
    In::Raw(conn, msg, Some(note))
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
            In::Raw(conn, msg, note) => {
                let line = msg.to_string();
                let lines = s.handle_raw(conn, &line);
                let mut step = json!({"send_raw": {"conn": conn, "line": line}});
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
    for (i, bar) in b.iter().enumerate().take(128 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
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
    for (i, bar) in b.iter().enumerate().take(93 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
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
    for (i, bar) in b.iter().enumerate().take(95 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
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
            "She holds THYAO now: the negative transitions of stories about a position pass any level (`relevant`; K42/A22: births still go through the level).",
        ),
        bar_ev(k, 126, &b[126], None),
        bar_ev(k, 127, &b[127], Some("K42/A22: the tier-3 RSI extreme born on this bar does not reach her although she holds THYAO (a birth goes through the level); the tier-1 breakout's transition does.")),
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
        "R3 (K31, §20): sensitivity and the position override. `set_audience` / `set_sensitivity` write a person's own preferences through the command path (another person's only by an owner); the topic `capsules` filters by them: with the future-investor default (`important`) only tier 1 passes; (R4, K42/A22) only the negative transitions (weakened, invalidated, retracted) of a story about a held position pass any level, its birth does not. The engine writes many entries and never touches the preferences (final document).",
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

fn t44() -> Value {
    let b = bars();
    let k = 121;
    let mut steps = vec![hello("a", "ana", None)];
    for (i, bar) in b.iter().enumerate().take(128 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
    }
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"v1","query":{"kind":"verify_evidence","item":{"finding":{"kind":"breakout.range.down","latest":true}}}}),
        "`verify_evidence.item` takes an item selector (§17.2): the latest breakout finding, resolved and named in `resolved`.",
    ));
    steps.push(query(
        "a",
        "v2",
        json!({"kind":"verify_evidence","item":{"fact":{"kind":"breakout.range.down","bar":126}}}),
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"v3","query":{"kind":"verify_evidence","item":{"finding":{"kind":"breakout.range.down"}}}}),
        "Several matches: `ambiguous` at the selector's path with the candidates in latest-fact-bar order.",
    ));
    steps.push(send_raw_n(
        "a",
        json!({"type":"query","id":"v4","query":{"kind":"verify_evidence","item":{"finding":{"knd":"breakout.range.down"}}}}),
        "An unknown selector key: `schema` at its path, with the keys a finding selector takes.",
    ));
    steps.push(hello("o", "arkin", None));
    steps.push(send_n(
        "o",
        json!({"type":"command","id":"c1","ops":[{"op":"show_finding","finding":{"kind":"breakout.range.down","latest":true},"view":"brk"}]}),
        "`show_finding` on the bound series: the command's scratch document has no bars; the presenter reads the bound bars as of the command's start (as `add_fact` and `set_view_range` do).",
    ));
    steps.push(send_n(
        "o",
        json!({"type":"query","id":"g1","query":{"kind":"get","id":"brk"}}),
        "The stored view: the breakout levels with their fact origins; the finding's licensed flow fact leaves no constant in it.",
    ));
    steps.push(send_n(
        "o",
        json!({"type":"query","id":"g2","query":{"kind":"log","after_seq":10}}),
        "The log stores the resolved finding id, not the selector.",
    ));
    run(
        "44-capsule-selectors-bound",
        "R3/E fixes (§17.2, §18.5, §20.5): on the bound THYAO series of transcript 38, `verify_evidence.item` takes an item selector (finding `latest`, fact by bar) resolved like every selector field (`resolved`; several matches `ambiguous` with candidates; an unknown key `schema` at its path), and `show_finding` on a finding of the bound series stores the chart view built from the bound bars (no licensed constant in it; the log keeps the resolved id).",
        &["selectors", "verify_evidence", "show_finding", "binding", "capsule"],
        init(k, None),
        steps,
        None,
    )
}

fn t45() -> Value {
    let b = bars();
    let k = 121;
    let mut steps = vec![hello("a", "ana", None)];
    for (i, bar) in b.iter().enumerate().take(150 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
    }
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"l1","query":{"kind":"ledger","meaning":"breakout"}}),
        "K42/A23: the ledger key carries the direction (`kind/direction/detector@version`): up and down breakouts of one detector are two rows with separate bases.",
    ));
    steps.push(query(
        "a",
        "l2",
        json!({"kind":"ledger","meaning":"breakout","direction":"down"}),
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"l3","query":{"kind":"ledger","by_kind":true,"direction":"up"}}),
        "Per kind, one direction only.",
    ));
    steps.push(query("a", "l4", json!({"kind":"ledger","by_kind":true})));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"c1","query":{"kind":"capsule","capsule":"cap.THYAO.breakout.up.147"}}),
        "An up breakout counts only earlier up breakouts at its birth (`ledger_at_birth`); its key names the direction.",
    ));
    run(
        "45-capsule-ledger-direction",
        "R4/T3 (K42/A23, §20.2): the ledger key is `kind/direction/detector@version`. On the bound THYAO series of transcript 38 run to bar 150, the `ledger` query gives the up and down range breakouts as separate rows (filter `direction`, rows carry `direction`; `by_kind` sums the directions unless one is asked for), and a story's `ledger_at_birth` counts only closed stories of its own direction.",
        &["ledger", "ledger-direction", "capsule"],
        init(k, None),
        steps,
        None,
    )
}

fn t46() -> Value {
    let b = bars();
    let k = 106;
    let steps = vec![
        hello("a", "ana", None),
        hello("b", "arkin", None),
        sub("a", "k", json!({"kind":"capsules"})),
        sub("b", "k", json!({"kind":"capsules"})),
        send_n(
            "a",
            json!({"type":"command","id":"s1","ops":[{"op":"set_audience","audience":{"group":"future","positions":{"THYAO":100}}}]}),
            "Ana: the future-investor group (`important`, tier 1) and a THYAO position.",
        ),
        send_n(
            "b",
            json!({"type":"command","id":"s2","ops":[{"op":"set_audience","audience":{"group":"future"}}]}),
            "Arkın (entitled like Ana): the same level, no position.",
        ),
        bar_ev(k, 106, &b[106], None),
        bar_ev(k, 107, &b[107], Some("A tier-3 down breakout is born: a birth goes through the level for both (K42/A22), the position does not lift it.")),
        bar_ev(k, 108, &b[108], None),
        bar_ev(k, 109, &b[109], Some("The same story weakens (an opposite fact): a negative transition of a story about Ana's position reaches her whatever the level (`relevant`); Arkın gets nothing.")),
        bar_ev(k, 110, &b[110], Some("A tier-1 breakout is born: both get it by the level.")),
        query("a", "q", json!({"kind":"capsules","audience":true,"since_bar":100})),
    ];
    run(
        "46-capsule-negative-override",
        "R4/T3 (K42/A22, §20.4, CAPSULE §9.8): for a story about a person's position or active intent only the negative transitions (weakened, invalidated, retracted) pass whatever the sensitivity; its birth and the other transitions go through the level like any story. Two readers at `important`, one holding THYAO: the tier-3 birth reaches neither, its weakening reaches the holder only, a tier-1 birth reaches both. The `capsules` query with `audience` applies the same rule to each story's current stage.",
        &["topic-capsules", "sensitivity", "position-override", "negative-transition", "capsules"],
        init(k, None),
        steps,
        None,
    )
}

fn t47() -> Value {
    let b = bars();
    let k = 92;
    let mut steps = vec![hello("t", "ana", None), hello("e", "ana", Some("en"))];
    for (i, bar) in b.iter().enumerate().take(96 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
    }
    steps.push(send_n(
        "t",
        json!({"type":"query","id":"v1","query":{"kind":"capsules","open":true,"since_bar":84}}),
        "K42/A28: every item carries `validity`. The short-average state ended (its state condition no longer holds) but the story is still open: `no_longer`; the other open stories are `valid`.",
    ));
    steps.push(send_n(
        "t",
        json!({"type":"query","id":"v2","query":{"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}}),
        "The opened capsule: the story keeps the state condition (`condition`) apart from the expected resolution (`resolution`: the barrier rule); the glyph carries `validity: no_longer`, and the sentence says 'artık … değil'.",
    ));
    steps.push(send_n(
        "e",
        json!({"type":"query","id":"v3","query":{"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}}),
        "The same in English: 'no longer below', glyph `no_longer`.",
    ));
    steps.push(query(
        "t",
        "v4",
        json!({"kind":"capsules","open":false,"since_bar":90}),
    ));
    run(
        "47-capsule-validity",
        "R4/T3 (K42/A28, §20.2): validity today (`valid` / `no_longer` / `closed`) on the capsule item, the opened capsule and its glyph; the story keeps its state condition and its expected resolution in separate fields. A down-trend state whose condition broke while its story is still open is `no_longer` in the list, in the glyph and in the sentence (Turkish and English); closed stories are `closed`.",
        &["capsules", "capsule", "validity", "glyph", "lang"],
        init(k, None),
        steps,
        None,
    )
}

fn t48() -> Value {
    let b = bars();
    let k = 121;
    let mut steps = vec![hello("a", "ana", None)];
    for (i, bar) in b.iter().enumerate().take(150 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
    }
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s1","query":{"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"breakout","direction":"up","latest":true}}}),
        "The `capsule` query takes a capsule selector (§17.1): THYAO's latest up breakout (order: latest transition bar, then birth bar), named in `resolved`.",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s2","query":{"kind":"capsule","capsule":{"meaning":"breakout","direction":"up"}}}),
        "Several up breakouts: `ambiguous` with the candidates in latest-transition-bar order.",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s3","query":{"kind":"capsule","capsule":{"meaning":"breakout","open":true}}}),
        "No breakout is open at bar 150: `not_found` with the closest capsules (most keys satisfied first).",
    ));
    steps.push(send_raw_n(
        "a",
        json!({"type":"query","id":"s4","query":{"kind":"capsule","capsule":{"stage":"born","opn":true}}}),
        "An unknown key: `schema` at its path, with the keys a capsule selector takes.",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s5","query":{"kind":"capsules","latest":true}}),
        "`capsules` with `latest`: only the most recently born capsule.",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s6","query":{"kind":"capsules","born_since_bar":145,"meaning":"breakout"}}),
        "Birth filter: breakouts born at bar 145 or later (`since_bar` filters on the latest transition instead).",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"s7","query":{"kind":"verify_evidence","item":{"finding":{"meaning":"breakout","stage":"invalidated","latest":true}}}}),
        "A finding selector with the story keys `meaning` and `stage` (also `instrument`, `open`): the latest invalidated breakout's finding.",
    ));
    run(
        "48-capsule-selector",
        "R4/T3 (R3 ergonomics, GOAL 13; §17.1, §20.2): the `capsule` query takes a capsule selector (`instrument`, `meaning`, `direction`, `stage`, `open`, `born_bar`, `series`, `first`/`latest`; order: latest transition bar, birth bar, document) with the usual `resolved`, `ambiguous`, `not_found` and `schema` answers; `capsules` takes `latest` (the most recently born) and the birth filters `born_since_bar` / `born_since_t`; the finding selector takes the story keys `instrument`, `meaning`, `stage`, `open`.",
        &["selectors", "capsule", "capsules", "verify_evidence"],
        init(k, None),
        steps,
        None,
    )
}

fn t57() -> Value {
    let b = bars();
    let k = 121;
    let mut steps = vec![hello("a", "ana", None), hello("b", "bora", None)];
    for (i, bar) in b.iter().enumerate().take(150 + 1).skip(k) {
        steps.push(bar_ev(k, i, bar, None));
    }
    let sel = json!({"instrument":"THYAO","meaning":"flow","latest":true});
    steps.push(send_n(
        "a",
        json!({"type":"query","id":"w1","query":{"kind":"capsule","capsule":sel}}),
        "Ana holds the flow entitlement: THYAO's latest flow story is a licensed one; nothing is withheld.",
    ));
    steps.push(send_n(
        "b",
        json!({"type":"query","id":"w2","query":{"kind":"capsule","capsule":sel}}),
        "Bora (bars, no licensed flow): the same selector resolves among the stories he may see (an open-data flow story). `withheld` says that newer licensed activity exists and what it requires — only what the server comments he already gets told him (no new existence oracle).",
    ));
    steps.push(send_n(
        "b",
        json!({"type":"query","id":"w3","query":{"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"breakout","latest":true}}}),
        "A selector that rules flow out: no `withheld`.",
    ));
    run(
        "57-capsule-selector-withheld",
        "R4/S1 (ergonomics C11; §20.2, §20.5): a capsule selector answered for a reader without the licensed flow resolves among what he may see, and says what it may also have matched: `capsule_detail.withheld` = the entitlement newer licensed stories require and the server comments (mühür) newer than the resolved capsule's latest transition. Absent for an entitled reader, for an id, and for a selector whose `meaning` is not `flow`.",
        &["selectors", "capsule", "entitlement", "server-comment", "withheld"],
        init(k, None),
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
    for t in [
        t38(),
        t39(),
        t40(),
        t41(),
        t42(),
        t43(),
        t44(),
        t45(),
        t46(),
        t47(),
        t48(),
        t57(),
    ] {
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
