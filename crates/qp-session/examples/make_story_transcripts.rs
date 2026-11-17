//! Generates the R5/M18 story-list conformance transcripts 60–63
//! (`fixtures/protocol/conformance/6x-stories-*.json`, `docs/PROTOCOL.md` §23): the
//! `stories` query and topic for an entitled reader, for readers without the licensed
//! flow and without any data entitlement, over ranges of about 5, 50 and 200 stories
//! (closed, open and feed-hidden ones mixed), and the live updates.
//!
//! Each scenario is messages and data events; the expected outputs are the reference
//! authority's answers (welcome trimmed to its type and `seq`). `qp conform` checks them
//! in process, over `qp serve` and over MCP. Deterministic: two runs write the same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_story_transcripts            # check only
//! cargo run -p qp-session --example make_story_transcripts -- --write
//! cargo run --release -p qp-session --example make_story_transcripts -- --stats
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

/// Seed of the synthetic bars and of the licensed flow.
const SEED: u64 = 7;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn principals() -> Value {
    json!([
        {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
        {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
        {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
        {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"}
    ])
}

/// THYAO daily bars `synthetic_bars(k, SEED)` on feed `bist`, the capsule engine on `px`
/// with the synthetic licensed flow. Ana may read everything, Bora the bars only, Can
/// nothing.
fn init(k: usize) -> Value {
    json!({
        "document": {
            "schema_version": "0", "id": "doc-stories", "title": "Hikâye",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": principals(),
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": k, "seed": SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"], "bora": ["bist/THYAO"], "can": []}
        },
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
    let mut surface = json!({"kind":"phone"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    )
}

fn query(conn: &'static str, id: &str, q: Value) -> In {
    send(conn, json!({"type":"query","id":id,"query":q}))
}

fn query_n(conn: &'static str, id: &str, q: Value, note: &'static str) -> In {
    send_n(conn, json!({"type":"query","id":id,"query":q}), note)
}

/// Data event: bar `i` of the synthetic feed (`seq` = position after the initial `k`).
fn bar_ev(k: usize, i: usize, note: Option<&'static str>) -> In {
    let b = qp_source::synthetic_bars(i + 1, SEED)[i].clone();
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

fn run(name: &str, description: &str, covers: &[&str], init_v: Value, steps: Vec<In>) -> Value {
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
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init_v,
        "steps": out_steps,
        "final": {"log_len": s.log().len()}
    })
}

/// Bars of the initial feed in the transcripts.
const K: usize = 400;
/// Last bars of the ~5-story range.
const R5: usize = 25;
/// Last bars of the ~50-story range.
const R50: usize = 230;
/// Bars of the 200-story transcript.
const K200: usize = 1200;

fn t60() -> Value {
    let steps = vec![
        hello("a", "ana", None),
        send_n(
            "a",
            json!({"type":"command","id":"s1","ops":[{"op":"set_audience","audience":{"group":"future","sensitivity":"important"}}]}),
            "Ana reads only tier 1 in her feed: every other story is listed with `in_feed: false, hidden: sensitivity` (it is still drawn on the chart, K50).",
        ),
        query_n(
            "a",
            "r5",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R5}}),
            "A short range: every story whose life (birth to close, or to the last closed bar when open) meets the last 25 bars, by birth.",
        ),
        query_n(
            "a",
            "r50",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R50},"limit":20}),
            "About 50 stories in the last 230 bars, 20 per page: `total` counts the range, `next` is the cursor.",
        ),
        query(
            "a",
            "r50b",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R50},"limit":20,"after":"$NEXT"}),
        ),
        query_n(
            "e",
            "en",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R5}}),
            "The same short range for an English connection of Ana: the face pieces in English.",
        ),
        query_n(
            "a",
            "t",
            json!({"kind":"stories","instrument":"THYAO","range":{"from_t":1_900_000_000_000_i64,"to_t":2_000_000_000_000_i64}}),
            "A time range past the last bar: empty range (`invalid`).",
        ),
        query_n(
            "a",
            "x1",
            json!({"kind":"stories","instrument":"THYA"}),
            "Unknown instrument: `not_found` with a suggestion.",
        ),
        query_n(
            "a",
            "x2",
            json!({"kind":"stories","instrument":"THYAO","limit":501}),
            "Over the limit (at most 500).",
        ),
        query_n(
            "a",
            "x3",
            json!({"kind":"stories","instrument":"THYAO","after":"cap.nope"}),
            "A cursor that is not a story of this list.",
        ),
        query_n(
            "a",
            "x4",
            json!({"kind":"stories","instrument":"THYAO","series":"py"}),
            "A series that is not one of the instrument's.",
        ),
        query_n(
            "a",
            "x5",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":5,"from_bar":1}}),
            "Two range forms at once.",
        ),
    ];
    run(
        "60-stories-entitled",
        "R5/M18 (§23, K50, K54): the instrument's story list for an entitled reader (Ana: bars and licensed flow). Every story of THYAO whose life meets the range, closed (with `closed_bar`, `outcome`) and open, licensed flow stories included, each with its face pieces; stories her feed does not show (sensitivity `important`) are listed with `in_feed: false` and the reason. Ranges by bars, last n bars and time; paging (`limit`, `after`, `next`, `total`); English faces; errors (empty range, unknown instrument, limit, cursor, series, two range forms). No bars travel (K23).",
        &["stories", "story-list", "paging", "entitlement", "feed-hidden", "errors", "lang"],
        init(K),
        fill_next(with_en(steps), &init(K)),
    )
}

/// Adds Ana's English connection after her first hello.
fn with_en(mut steps: Vec<In>) -> Vec<In> {
    steps.insert(1, hello("e", "ana", Some("en")));
    steps
}

fn t61() -> Value {
    let q = json!({"kind":"stories","instrument":"THYAO","range":{"last":R50}});
    let steps = vec![
        hello("a", "ana", None),
        hello("b", "bora", None),
        hello("c", "can", None),
        query_n("a", "a60", q.clone(), "Ana (entitled to everything): the licensed flow stories are listed."),
        query_n(
            "b",
            "b60",
            q.clone(),
            "Bora (bars, not the licensed flow): his world is the public one (§20.5). The licensed stories do not exist for him; their existence, numbers and timing reach him only as the server comments he already gets (`comments`, `trust: server`, digit-free). Tiers and ledgers are the public world's.",
        ),
        query_n(
            "c",
            "c60",
            q,
            "Can (no data entitlement): the public stories with digit-free faces (`redacted`, no `ledger_n`), no comment carries a digit.",
        ),
        query_n(
            "b",
            "bx",
            json!({"kind":"stories","instrument":"THYAO","after":licensed_only_id()}),
            "A licensed story's id as a cursor: for Bora it is not a story of the list (`not_found`, the same as any unknown id: no existence oracle).",
        ),
    ];
    run(
        "61-stories-unentitled",
        "R5/M18 (§23, K33, K34, Ö37): the same story list for three readers. Ana gets every story; Bora (bars, no licensed flow) reads the public world: no licensed story, only the server comments on that data in the range; Can (no data entitlement) gets digit-free faces. No value, confidence, importance, ledger count or data digest derived from licensed data reaches Bora or Can.",
        &["stories", "entitlement", "server-comment", "muhur", "redacted", "non-interference"],
        init(K),
        steps,
    )
}

fn t62() -> Value {
    let steps = vec![
        hello("a", "ana", None),
        send_n(
            "a",
            json!({"type":"command","id":"s1","ops":[{"op":"set_audience","audience":{"group":"future","sensitivity":"balanced"}}]}),
            "Ana's feed shows tiers 1–2: tier-3 stories are listed as not in her feed.",
        ),
        query_n(
            "a",
            "a200",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R200}}),
            "About 200 stories in the range (the chart zoomed out): the first page holds the default 200, `total` counts the range and `next` is the cursor.",
        ),
        query_n(
            "a",
            "a200b",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":R200},"after":"$NEXT"}),
            "The rest of the range.",
        ),
    ];
    run(
        "62-stories-200",
        "R5/M18 (§23, K50, K52): a range holding about 200 stories (closed, open and feed-hidden mixed) for an entitled reader, over two pages. (The same range for a reader without the licensed flow is measured in the round report, not kept as a transcript: its answer also carries about 180 server comments.)",
        &["stories", "story-list", "scale", "paging", "feed-hidden"],
        init(K200),
        fill_next(steps, &init(K200)),
    )
}

/// Bars of the 200-story range.
const R200: usize = 950;

fn t63() -> Value {
    let k = 120;
    let mut steps = vec![
        hello("a", "ana", None),
        hello("b", "bora", None),
        hello("c", "can", None),
        query(
            "a",
            "q",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":30}}),
        ),
        send_n(
            "a",
            json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}}),
            "The chart opened: the list was asked for, then its changes (the state at subscription is the answer above: no event).",
        ),
        send("b", json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}})),
        send("c", json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}})),
        send_n(
            "a",
            json!({"type":"subscribe","id":"bad","topic":{"kind":"stories","instrument":"ASELS"}}),
            "A topic for an instrument without a bar series: `not_found`.",
        ),
    ];
    for i in k..k + 12 {
        steps.push(bar_ev(
            k,
            i,
            (i == k + 1).then_some(
                "Each final bar is one engine entry; after it the authority sends a `story` row for every story born, moved, closed, or whose validity changed — for each reader in its own world (Bora and Can: public stories; Bora also the server comments on the topic).",
            ),
        ));
    }
    steps.push(send_n(
        "a",
        json!({"type":"command","id":"s1","ops":[{"op":"set_sensitivity","sensitivity":"important"}]}),
        "Ana's feed level changes: the rows whose feed place changed come again (`in_feed`, `hidden`).",
    ));
    steps.push(send_n(
        "a",
        json!({"type":"unsubscribe","id":"u","sub":"st"}),
        "The chart closed: no more rows for Ana.",
    ));
    steps.push(bar_ev(k, k + 12, None));
    run(
        "63-stories-live",
        "R5/M18 (§23): live updates of the story list. The `stories` topic sends, after every input, the rows whose key (stage, latest and closing bar, validity, tier, feed place, coin-flip mark, redaction) changed: births, transitions, closings, validity changes, feed-place changes after an audience change; and, to a reader without the licensed flow, the server comments on that instrument. Unsubscribing stops it.",
        &["stories", "topic-stories", "story", "live", "server-comment", "set_sensitivity"],
        init(k),
        steps,
    )
}

/// The id of a story Ana sees (licensed flow) that does not exist in Bora's world.
fn licensed_only_id() -> String {
    let init: SessionInit = serde_json::from_value(init(K)).expect("init");
    let mut s = Session::new(init, &root()).expect("session");
    let mut ids = Vec::new();
    for (c, who) in [("a", "ana"), ("b", "bora")] {
        s.handle_raw(
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}})
                .to_string(),
        );
        let l = s.handle_raw(
            c,
            &json!({"type":"query","id":"q","query":{"kind":"stories","instrument":"THYAO","limit":500}})
                .to_string(),
        );
        let v = serde_json::to_value(&l[0].msg).unwrap();
        let list: Vec<String> = v["data"]["stories"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_owned())
            .collect();
        ids.push(list);
    }
    ids[0]
        .iter()
        .find(|i| i.contains(".flow.") && !ids[1].contains(i))
        .expect("a licensed-only story")
        .clone()
}

/// Replaces `"after":"$NEXT"` in the step after a paged query with that answer's `next`.
fn fill_next(steps: Vec<In>, init_v: &Value) -> Vec<In> {
    let init: SessionInit = serde_json::from_value(init_v.clone()).expect("init");
    let mut s = Session::new(init, &root()).expect("session");
    let mut last_next: Option<Value> = None;
    let mut out = Vec::new();
    for st in steps {
        match st {
            In::Send(conn, mut msg, note) => {
                if msg.pointer("/query/after") == Some(&json!("$NEXT")) {
                    msg["query"]["after"] = last_next.clone().unwrap_or(Value::Null);
                }
                for l in s.handle_raw(conn, &msg.to_string()) {
                    let v = serde_json::to_value(&l.msg).unwrap();
                    if let Some(n) = v.pointer("/data/next") {
                        last_next = Some(n.clone());
                    }
                }
                out.push(In::Send(conn, msg, note));
            }
            In::Data(ev, note) => {
                let e: DataEvent = serde_json::from_value(ev.clone()).expect("event");
                let _ = s.data_event(&e);
                out.push(In::Data(ev, note));
            }
        }
    }
    out
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("json");
    s.push('\n');
    s
}

/// `--stats`: story counts per range and the authority's answer size and time.
fn stats() {
    for (k, last) in [(K, R5), (K, R50), (K200, R200)] {
        let init: SessionInit = serde_json::from_value(init(k)).expect("init");
        let t0 = Instant::now();
        let mut s = Session::new(init, &root()).expect("session");
        let start_ms = t0.elapsed().as_secs_f64() * 1e3;
        for (c, who) in [("a", "ana"), ("b", "bora"), ("c", "can")] {
            s.handle_raw(
                c,
                &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}})
                    .to_string(),
            );
        }
        for c in ["a", "b", "c"] {
            let q = json!({"type":"query","id":"q","query":{"kind":"stories","instrument":"THYAO",
                "range":{"last":last},"limit":500}})
            .to_string();
            // Warm (the reader's view), then timed runs.
            let first = s.handle_raw(c, &q);
            let mut times = Vec::new();
            for _ in 0..20 {
                let t = Instant::now();
                let _ = s.handle_raw(c, &q);
                times.push(t.elapsed().as_secs_f64() * 1e3);
            }
            times.sort_by(f64::total_cmp);
            let v = serde_json::to_value(&first[0].msg).unwrap();
            let bytes = serde_json::to_string(&v).unwrap().len();
            let rows = v
                .pointer("/data/stories")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let closed = v
                .pointer("/data/stories")
                .and_then(Value::as_array)
                .map_or(0, |a| {
                    a.iter().filter(|r| r.get("closed_bar").is_some()).count()
                });
            let hidden = v
                .pointer("/data/stories")
                .and_then(Value::as_array)
                .map_or(0, |a| {
                    a.iter().filter(|r| r["in_feed"] == json!(false)).count()
                });
            let comments = v
                .pointer("/data/comments")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            println!(
                "bars {k} last {last} reader {c}: rows {rows} (closed {closed}, open {}, not in feed {hidden}), comments {comments}, answer {bytes} B; median {:.2} ms, p90 {:.2} ms (session start {start_ms:.0} ms)",
                rows - closed,
                times[times.len() / 2],
                times[times.len() * 9 / 10],
            );
        }
    }
    live_cost();
}

/// The authority's cost of the `stories` topic: time per final bar (data event) on the
/// 1,200-bar instrument with three subscribers (Ana, Bora, Can) against none.
fn live_cost() {
    let more = 40;
    let bars = qp_source::synthetic_bars(K200 + more, SEED);
    let mut per = Vec::new();
    for subscribed in [false, true] {
        let init: SessionInit = serde_json::from_value(init(K200)).expect("init");
        let mut s = Session::new(init, &root()).expect("session");
        for (c, who) in [("a", "ana"), ("b", "bora"), ("c", "can")] {
            s.handle_raw(
                c,
                &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}})
                    .to_string(),
            );
            if subscribed {
                s.handle_raw(
                    c,
                    &json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}})
                        .to_string(),
                );
            }
        }
        let mut times = Vec::new();
        let mut events = 0;
        for (i, b) in bars.iter().enumerate().skip(K200) {
            let e: DataEvent = serde_json::from_value(
                json!({"feed": feed(), "seq": i - K200 + 1, "change": {"kind":"bar","bar": b}}),
            )
            .expect("event");
            let t = Instant::now();
            let lines = s.data_event(&e).expect("event applies");
            times.push(t.elapsed().as_secs_f64() * 1e3);
            events += lines
                .iter()
                .filter(|l| {
                    serde_json::to_string(&l.msg)
                        .unwrap()
                        .contains("\"kind\":\"story\"")
                })
                .count();
        }
        times.sort_by(f64::total_cmp);
        per.push((times[times.len() / 2], times[times.len() * 9 / 10], events));
    }
    println!(
        "live: per final bar, 1200-bar instrument: no stories subscriber median {:.2} ms p90 {:.2} ms; three subscribers median {:.2} ms p90 {:.2} ms, {} story events over {more} bars",
        per[0].0, per[0].1, per[1].0, per[1].1, per[1].2
    );
}

fn main() {
    if std::env::args().any(|a| a == "--stats") {
        stats();
        return;
    }
    let write = std::env::args().any(|a| a == "--write");
    let dir = root().join("fixtures/protocol/conformance");
    let mut stale = 0;
    for t in [t60(), t61(), t62(), t63()] {
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
