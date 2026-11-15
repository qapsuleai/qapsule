//! RP2/T1 — measurement baseline of today's reference authority (`qp_session::Session`).
//!
//! One binary, run natively and as `wasm32-wasip1` under Node. Prints `key = value`
//! lines grouped by section; times in microseconds (`_us`) or milliseconds (`_ms`).
//! Everything is deterministic (fixed seeds) except the times themselves.
//!
//! Usage:
//! - `tick-bench [bars] [ticks] [reps] [studies,...]` (defaults `10000 600 15 5,50,500`)
//! - `tick-bench profile <studies> <ticks>`: only the session tick loop (for `sample`).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use qp_compute::{create, Field, Study};
use qp_schema::protocol::{
    ChangeOp, ClientMsg, DocumentRef, Op, OutboundLine, QueryResult, ServerMsg, SessionInit,
    WriteMode,
};
use qp_schema::{Bar, Document, SeriesData, SeriesSource};
use qp_session::{apply_change, Session};
use serde_json::{json, Value};

const THYAO: &str = include_str!("../../../../../fixtures/protocol/docs/thyao.json");
const INIT: &str = include_str!("../../../../../fixtures/protocol/init/thyao.json");
const DAY: i64 = 86_400_000;
/// Studies of the fixture document (`rsi14`, `macd`, `bb20`); extra ones are generated.
const BASE_STUDIES: usize = 3;

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// SplitMix64: fixed-seed generator for the *data*, never used in compute code.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn digest_json<T: serde::Serialize>(v: &T) -> String {
    format!("{:016x}", fnv(&serde_json::to_vec(v).unwrap()))
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

fn sorted(v: &[f64]) -> Vec<f64> {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s
}

fn pct(v: &[f64], p: f64) -> f64 {
    let s = sorted(v);
    s[((s.len() as f64 - 1.0) * p).round() as usize]
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Runs `f` `reps` times; median time in microseconds and the last result.
fn bench<R>(reps: usize, mut f: impl FnMut() -> R) -> (f64, R) {
    let mut times = Vec::with_capacity(reps);
    let mut last = None;
    for _ in 0..reps {
        let t = Instant::now();
        let r = f();
        times.push(us(t));
        last = Some(r);
    }
    (pct(&times, 0.5), last.unwrap())
}

fn stats(v: &[f64]) -> String {
    format!(
        "mean {:.2} p50 {:.2} p99 {:.2} max {:.2} (n {})",
        mean(v),
        pct(v, 0.5),
        pct(v, 0.99),
        pct(v, 1.0),
        v.len()
    )
}

macro_rules! kv {
    ($k:expr, $($arg:tt)*) => { println!("{} = {}", $k, format!($($arg)*)) };
}

// ---------------------------------------------------------------------------
// Scenario data
// ---------------------------------------------------------------------------

fn px_bars(doc: &Document) -> &[Bar] {
    doc.series
        .iter()
        .find(|s| s.id == "px")
        .unwrap()
        .bars()
        .unwrap()
}

/// Parameters of generated study `i` (deterministic mix of the five built-ins).
fn extra_study(i: usize) -> (&'static str, Value) {
    const K: [f64; 3] = [1.5, 2.0, 2.5];
    let f = |x: usize| x as f64;
    match i % 5 {
        0 => ("sma", json!({"period": f(5 + (i * 7) % 196)})),
        1 => ("ema", json!({"period": f(5 + (i * 11) % 196)})),
        2 => ("rsi", json!({"period": f(2 + (i * 3) % 49)})),
        3 => {
            let fast = 5 + i % 10;
            json_macd(fast, fast + 10 + (i * 3) % 20, 5 + i % 7)
        }
        _ => (
            "bollinger",
            json!({"period": f(10 + (i * 5) % 41), "k": K[i % 3]}),
        ),
    }
}

fn json_macd(fast: usize, slow: usize, signal: usize) -> (&'static str, Value) {
    (
        "macd",
        json!({"fast": fast as f64, "slow": slow as f64, "signal": signal as f64}),
    )
}

/// The fixture document with `px` extended to `n_bars` bars by a fixed-seed random walk
/// (same generator and seed as RD1/T6d) and `n_studies` study series on `px` in total
/// (the fixture's three plus generated ones; series only, no layers).
fn gen_doc(n_bars: usize, n_studies: usize) -> Document {
    let mut v: Value = serde_json::from_str(THYAO).unwrap();
    let instrument = v["series"][0]["instrument"].clone();
    let series = v["series"].as_array_mut().unwrap();
    for i in 0..n_studies.saturating_sub(BASE_STUDIES) {
        let (study, params) = extra_study(i);
        series.push(json!({
            "id": format!("s{i:03}"),
            "instrument": instrument,
            "interval": "1d",
            "source": {"kind": "study", "study": study, "inputs": ["px"], "params": params}
        }));
    }
    let mut doc: Document = serde_json::from_value(v).unwrap();
    let px = doc.series.iter_mut().find(|s| s.id == "px").unwrap();
    let Some(SeriesData::Bars { bars }) = &mut px.data else {
        panic!("px has bars")
    };
    let mut rng = Rng(0x5444_3648_414e_444f); // "TD6HANDO", as in T6d
    while bars.len() < n_bars {
        let last = bars.last().unwrap().clone();
        let o = last.c;
        let c = r2((o * (1.0 + (rng.unit() - 0.5) * 0.04)).max(1.0));
        let h = r2(o.max(c) * (1.0 + rng.unit() * 0.01));
        let l = r2(o.min(c) * (1.0 - rng.unit() * 0.01));
        let v = r2(400_000.0 + rng.unit() * 2_000_000.0);
        bars.push(Bar {
            t: last.t + DAY,
            o,
            h,
            l,
            c,
            v,
        });
    }
    doc
}

#[derive(Clone)]
enum Tick {
    Append(Bar),
    Update(Bar),
}

impl Tick {
    fn bar(&self) -> &Bar {
        match self {
            Self::Append(b) | Self::Update(b) => b,
        }
    }
    fn op(&self) -> Op {
        match self {
            Self::Append(b) => Op::AppendBars {
                series: "px".to_owned(),
                bars: vec![b.clone()],
            },
            Self::Update(b) => Op::UpdateLastBar {
                series: "px".to_owned(),
                bar: b.clone(),
            },
        }
    }
    fn change(&self) -> ChangeOp {
        match self {
            Self::Append(b) => ChangeOp::AppendBars {
                series: "px".to_owned(),
                bars: vec![b.clone()],
            },
            Self::Update(b) => ChangeOp::SetLastBar {
                series: "px".to_owned(),
                bar: b.clone(),
            },
        }
    }
}

/// `n` ticks of a live feed: per simulated second one new bar then 59 revisions of it
/// (same generator and seed as RD1/T6d).
fn gen_feed(last: &Bar, n: usize) -> Vec<Tick> {
    const PER_SEC: usize = 60;
    let mut rng = Rng(0xFEED);
    let mut out = Vec::with_capacity(n);
    let mut prev = last.clone();
    while out.len() < n {
        let o = prev.c;
        let mut b = Bar {
            t: prev.t + DAY,
            o,
            h: o,
            l: o,
            c: o,
            v: r2(rng.unit() * 1000.0),
        };
        out.push(Tick::Append(b.clone()));
        for _ in 1..PER_SEC {
            b.c = r2((b.c * (1.0 + (rng.unit() - 0.5) * 0.004)).max(1.0));
            b.h = b.h.max(b.c);
            b.l = b.l.min(b.c);
            b.v = r2(b.v + rng.unit() * 1000.0);
            out.push(Tick::Update(b.clone()));
        }
        prev = b;
    }
    out.truncate(n);
    out
}

fn tick_msg(i: usize, t: &Tick) -> String {
    serde_json::to_string(&ClientMsg::Command {
        id: format!("t{i}"),
        ops: vec![t.op()],
        mode: WriteMode::Auto,
        base: None,
        reason: None,
    })
    .unwrap()
}

/// Study series of a document: (id, study, params).
fn study_series(doc: &Document) -> Vec<(String, String, BTreeMap<String, f64>)> {
    doc.series
        .iter()
        .filter_map(|s| match &s.source {
            SeriesSource::Study { study, params, .. } => {
                Some((s.id.clone(), study.clone(), params.clone()))
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Reference authority helpers
// ---------------------------------------------------------------------------

fn session(doc: Document, epoch: u64) -> Session {
    let mut init: SessionInit = serde_json::from_str(INIT).unwrap();
    init.document = DocumentRef::Inline(Box::new(doc));
    init.epoch = epoch;
    Session::new(init, Path::new(".")).unwrap()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn hello(s: &mut Session, conn: &str, principal: &str, kind: &str) {
    let r = send(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":kind}}),
    );
    assert!(
        matches!(r[0].msg, ServerMsg::Welcome { .. }),
        "hello failed: {:?}",
        r[0].msg
    );
}

fn subscribe(s: &mut Session, conn: &str, id: &str, topic: &Value) {
    let r = send(s, conn, &json!({"type":"subscribe","id":id,"topic":topic}));
    assert!(matches!(r[0].msg, ServerMsg::Ack { .. }), "{:?}", r[0].msg);
}

fn is_error(lines: &[OutboundLine]) -> bool {
    lines
        .iter()
        .any(|l| matches!(l.msg, ServerMsg::Error { .. }))
}

/// Watchers of the scenario.
#[derive(Clone, Copy, PartialEq)]
enum Watch {
    /// Fixture's live capsule (invalidation: `rsi14` crosses below 70), one `changes`
    /// subscription and one `condition` subscription (`macd` crosses above 0).
    Base,
    /// As in RD1/T6d: the live capsule and one `changes` subscription, no `condition`
    /// subscription (only `rsi14` is recomputed per tick).
    T6d,
    /// `Base` plus one `condition` subscription per study series, so the session
    /// computes every study on every tick (the same study work as the core).
    All,
}

/// A live session on `doc` with the feed connected and the watchers of `watch`.
fn live_session(doc: &Document, watch: Watch) -> Session {
    let mut s = session(doc.clone(), 0);
    hello(&mut s, "u", "arkin", "phone");
    hello(&mut s, "f", "feed", "headless");
    subscribe(&mut s, "u", "sub-ch", &json!({"kind":"changes"}));
    if watch == Watch::T6d {
        return s;
    }
    subscribe(
        &mut s,
        "u",
        "sub-macd",
        &json!({"kind":"condition","condition":
            {"kind":"threshold","series":"macd","op":"cross_above","value":0.0}}),
    );
    if watch == Watch::All {
        // At most 32 subscriptions per connection: spread over viewer connections.
        for (k, (id, _, _)) in study_series(doc).iter().enumerate() {
            let conn = format!("w{}", k / 32);
            if k % 32 == 0 {
                hello(&mut s, &conn, "viewer", "headless");
            }
            subscribe(
                &mut s,
                &conn,
                &format!("all-{id}"),
                &json!({"kind":"condition","condition":
                    {"kind":"threshold","series":id,"op":"gt","value":1e12}}),
            );
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Compute core alone
// ---------------------------------------------------------------------------

struct CoreStudy {
    study: Box<dyn Study>,
    outs: Vec<Vec<Option<f64>>>,
    buf: Vec<Option<f64>>,
}

/// The compute core advancing every study series of `doc` over the feed, no session:
/// `push` for a new bar, `update_last` for a revision; outputs kept in columns (as a
/// view would need them). Attached to history with `Study::extend` (state only).
struct Core {
    studies: Vec<CoreStudy>,
}

impl Core {
    fn new(doc: &Document) -> Self {
        let close = Field::Close.column(px_bars(doc));
        let studies = study_series(doc)
            .into_iter()
            .map(|(_, name, params)| {
                let mut study = create(&name, &params, Field::Close).unwrap();
                study.extend(&[&close]);
                let n = study.outputs().len();
                CoreStudy {
                    study,
                    outs: vec![Vec::new(); n],
                    buf: vec![None; n],
                }
            })
            .collect();
        Self { studies }
    }

    #[inline(never)]
    fn tick(&mut self, t: &Tick) {
        let x = [t.bar().c];
        for s in &mut self.studies {
            match t {
                Tick::Append(_) => {
                    s.study.push(&x, &mut s.buf);
                    for (col, v) in s.outs.iter_mut().zip(&s.buf) {
                        col.push(*v);
                    }
                }
                Tick::Update(_) => {
                    s.study.update_last(&x, &mut s.buf);
                    for (col, v) in s.outs.iter_mut().zip(&s.buf) {
                        match col.last_mut() {
                            Some(last) => *last = *v,
                            None => col.push(*v),
                        }
                    }
                }
            }
        }
    }

    /// Last values of every output (for the bit-equality check).
    fn last_values(&self) -> Vec<Option<f64>> {
        self.studies
            .iter()
            .flat_map(|s| s.buf.iter().copied())
            .collect()
    }
}

fn parse_tick(m: &str) -> Tick {
    let msg: ClientMsg = serde_json::from_str(m).unwrap();
    let ClientMsg::Command { ops, .. } = msg else {
        panic!("command")
    };
    match &ops[0] {
        Op::AppendBars { bars, .. } => Tick::Append(bars[0].clone()),
        Op::UpdateLastBar { bar, .. } => Tick::Update(bar.clone()),
        _ => panic!("tick op"),
    }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

struct Cfg {
    bars: usize,
    ticks: usize,
    reps: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("profile") {
        let n: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(50);
        let ticks: usize = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(30_000);
        profile(n, ticks);
        return;
    }
    let cfg = Cfg {
        bars: args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10_000),
        ticks: args.get(2).and_then(|a| a.parse().ok()).unwrap_or(600),
        reps: args.get(3).and_then(|a| a.parse().ok()).unwrap_or(15),
    };
    let counts: Vec<usize> = args
        .get(4)
        .map_or("5,50,500", String::as_str)
        .split(',')
        .map(|x| x.parse().unwrap())
        .collect();
    let target = if cfg!(target_arch = "wasm32") {
        "wasm32"
    } else {
        std::env::consts::ARCH
    };
    println!("# RP2/T1 tick bench");
    kv!("target", "{target}");
    kv!(
        "config",
        "bars {} ticks {} (60 per simulated second: 1 append_bars + 59 update_last_bar) reps {} studies {:?}",
        cfg.bars,
        cfg.ticks,
        cfg.reps,
        counts
    );
    for n in counts {
        let doc = gen_doc(cfg.bars, n);
        let feed = gen_feed(px_bars(&doc).last().unwrap(), cfg.ticks);
        let msgs: Vec<String> = feed
            .iter()
            .enumerate()
            .map(|(i, t)| tick_msg(i, t))
            .collect();
        println!("\n## studies = {n}");
        kv!(
            format!("n{n}.doc"),
            "digest {} series {} study_series {} bars {}",
            digest_json(&doc),
            doc.series.len(),
            study_series(&doc).len(),
            px_bars(&doc).len()
        );
        kv!(
            format!("n{n}.feed"),
            "ticks {} appends {} bytes_per_msg {:.0}",
            feed.len(),
            feed.iter().filter(|t| matches!(t, Tick::Append(_))).count(),
            msgs.iter().map(String::len).sum::<usize>() as f64 / msgs.len() as f64
        );
        section_snapshot(&cfg, n, &doc);
        let s_t6d = section_session(n, &doc, &feed, &msgs, Watch::T6d);
        let s_base = section_session(n, &doc, &feed, &msgs, Watch::Base);
        let s_all = section_session(n, &doc, &feed, &msgs, Watch::All);
        let core = section_core(&cfg, n, &doc, &feed, &msgs);
        kv!(
            format!("n{n}.ratio.session_t6d_over_core"),
            "{:.0}x (overhead/core {:.0}x)",
            s_t6d / core,
            (s_t6d - core) / core
        );
        kv!(
            format!("n{n}.ratio.session_base_over_core"),
            "{:.0}x (overhead/core {:.0}x)",
            s_base / core,
            (s_base - core) / core
        );
        kv!(
            format!("n{n}.ratio.session_all_over_core"),
            "{:.0}x (overhead/core {:.0}x)",
            s_all / core,
            (s_all - core) / core
        );
        section_breakdown(n, &doc, &feed, &msgs);
        section_selector(&cfg, n, &doc);
    }
}

/// Snapshot (`{"kind":"document"}` result) size and the device-ready time.
fn section_snapshot(cfg: &Cfg, n: usize, doc: &Document) {
    let mut server = session(doc.clone(), 0);
    hello(&mut server, "dev", "arkin", "phone");
    let q = json!({"type":"query","id":"snap","query":{"kind":"document"}}).to_string();
    let (q_us, snap) = bench(cfg.reps, || {
        let lines = server.handle_raw("dev", &q);
        serde_json::to_string(&lines[0].msg).unwrap()
    });
    let t = Instant::now();
    let gz = gzip(snap.as_bytes());
    let gz_ms = us(t) / 1e3;
    kv!(
        format!("n{n}.snapshot.bytes"),
        "{} raw, {} gzip ({:.1}%), limit {} ({})",
        snap.len(),
        gz.len(),
        100.0 * gz.len() as f64 / snap.len() as f64,
        server.limits().max_message_bytes,
        if snap.len() as u64 > server.limits().max_message_bytes {
            "EXCEEDS"
        } else {
            "fits"
        }
    );
    kv!(
        format!("n{n}.snapshot.server_ms"),
        "query+serialize {:.3} gzip6 {:.2}",
        q_us / 1e3,
        gz_ms
    );
    let (parse_us, d) = bench(cfg.reps, || {
        let msg: ServerMsg = serde_json::from_str(&snap).unwrap();
        let ServerMsg::Result {
            data: QueryResult::Document { document },
            ..
        } = msg
        else {
            panic!("snapshot")
        };
        *document
    });
    assert_eq!(
        digest_json(&d),
        digest_json(server.document()),
        "snapshot round-trip"
    );
    let (clone_us, _) = bench(cfg.reps, || d.clone());
    let (sess_us, _) = bench(cfg.reps, || {
        let mut s = session(d.clone(), 1);
        hello(&mut s, "u", "arkin", "phone");
        hello(&mut s, "f", "feed", "headless");
        s
    });
    let sess_us = sess_us - clone_us;
    kv!(
        format!("n{n}.ready_ms"),
        "total {:.3} = parse {:.3} + session_new_and_2_hellos {:.3} (bench clone {:.3} subtracted)",
        (parse_us + sess_us) / 1e3,
        parse_us / 1e3,
        sess_us / 1e3,
        clone_us / 1e3
    );
    // Split: Session::new, hello of a reader (gets welcome.summary), hello of the feed.
    let (mut t_new, mut t_reader, mut t_feed) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..cfg.reps {
        let dd = d.clone();
        let t = Instant::now();
        let mut s = session(dd, 1);
        t_new.push(us(t));
        let t = Instant::now();
        hello(&mut s, "u", "arkin", "phone");
        t_reader.push(us(t));
        let t = Instant::now();
        hello(&mut s, "f", "feed", "headless");
        t_feed.push(us(t));
    }
    kv!(
        format!("n{n}.ready_split_ms"),
        "session_new {:.3} hello_reader {:.3} hello_feed {:.3} (medians)",
        pct(&t_new, 0.5) / 1e3,
        pct(&t_reader, 0.5) / 1e3,
        pct(&t_feed, 0.5) / 1e3
    );
}

/// Per-tick cost of the reference session (`handle_raw`, JSON parse included).
/// Returns the mean in microseconds.
fn section_session(n: usize, doc: &Document, feed: &[Tick], msgs: &[String], w: Watch) -> f64 {
    let label = match w {
        Watch::Base => "base",
        Watch::All => "all",
        Watch::T6d => "t6d",
    };
    let mut s = live_session(doc, w);
    let log0 = s.log().len();
    let mut per = Vec::with_capacity(msgs.len());
    let (mut app, mut upd) = (Vec::new(), Vec::new());
    let mut notes = 0usize;
    let t_all = Instant::now();
    for (m, t) in msgs.iter().zip(feed) {
        let t0 = Instant::now();
        let lines = s.handle_raw("f", m);
        let dt = us(t0);
        assert!(!is_error(&lines), "{:?}", lines[0].msg);
        notes += lines.len();
        per.push(dt);
        match t {
            Tick::Append(_) => app.push(dt),
            Tick::Update(_) => upd.push(dt),
        }
    }
    let total_ms = us(t_all) / 1e3;
    kv!(
        format!("n{n}.session_{label}.per_tick_us"),
        "{}",
        stats(&per)
    );
    kv!(
        format!("n{n}.session_{label}.append_vs_update_us"),
        "append mean {:.1} ; update mean {:.1}",
        mean(&app),
        mean(&upd)
    );
    let engine = s.log()[log0..]
        .iter()
        .filter(|e| e.author == qp_session::ENGINE_PRINCIPAL)
        .count();
    kv!(
        format!("n{n}.session_{label}.run"),
        "total {:.1} ms, log entries {} (engine {}), outbound lines {}, final doc {}",
        total_ms,
        s.log().len() - log0,
        engine,
        notes,
        digest_json(s.document())
    );
    mean(&per)
}

/// The compute core doing the same study work alone. Returns the mean per tick (µs).
fn section_core(cfg: &Cfg, n: usize, doc: &Document, feed: &[Tick], msgs: &[String]) -> f64 {
    // Whole-loop time (no per-tick timer), best and median of reps.
    let mut loops = Vec::new();
    let mut last = None;
    for _ in 0..cfg.reps.min(7) {
        let mut core = Core::new(doc);
        let t = Instant::now();
        for tk in feed {
            core.tick(tk);
        }
        loops.push(us(t) / feed.len() as f64);
        last = Some(core);
    }
    let core_mean = pct(&loops, 0.5);
    kv!(
        format!("n{n}.core.per_tick_us"),
        "loop median {:.3} best {:.3} (of {} loops)",
        core_mean,
        pct(&loops, 0.0),
        loops.len()
    );
    // Per-tick distribution (timer overhead included).
    let mut core = Core::new(doc);
    let mut per = Vec::with_capacity(feed.len());
    for tk in feed {
        let t = Instant::now();
        core.tick(tk);
        per.push(us(t));
    }
    kv!(format!("n{n}.core.per_tick_timed_us"), "{}", stats(&per));
    // With the tick message parsed from JSON first (as it arrives).
    let mut core = Core::new(doc);
    let mut per = Vec::with_capacity(feed.len());
    for m in msgs {
        let t = Instant::now();
        let tk = parse_tick(m);
        core.tick(&tk);
        per.push(us(t));
    }
    kv!(
        format!("n{n}.core_with_parse.per_tick_us"),
        "{}",
        stats(&per)
    );
    // Bit-equality: streaming last values vs batch over the same bars.
    let mut d = doc.clone();
    for tk in feed {
        apply_change(&mut d, &tk.change()).unwrap();
    }
    let bars = px_bars(&d);
    let batch: Vec<Option<f64>> = study_series(&d)
        .iter()
        .flat_map(|(_, name, params)| {
            let out = qp_compute::compute_bars(name, params, Field::Close, bars).unwrap();
            let st = create(name, params, Field::Close).unwrap();
            st.outputs()
                .iter()
                .map(|o| *out.outputs[*o].last().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    let stream = last.unwrap().last_values();
    let eq = batch.len() == stream.len()
        && batch
            .iter()
            .zip(&stream)
            .all(|(a, b)| a.map(f64::to_bits) == b.map(f64::to_bits));
    kv!(
        format!("n{n}.core.stream_eq_batch"),
        "{eq} ({} outputs)",
        batch.len()
    );
    // Batch recompute of all studies per tick (what the session does when all are
    // watched), sampled on the first ticks.
    let sample = feed.len().min(if n >= 500 { 30 } else { 120 });
    let mut d = doc.clone();
    let specs = study_series(doc);
    let mut per = Vec::with_capacity(sample);
    for tk in &feed[..sample] {
        apply_change(&mut d, &tk.change()).unwrap();
        let bars = px_bars(&d);
        let t = Instant::now();
        for (_, name, params) in &specs {
            std::hint::black_box(
                qp_compute::compute_bars(name, params, Field::Close, bars).unwrap(),
            );
        }
        per.push(us(t));
    }
    kv!(
        format!("n{n}.core_batch_all.per_tick_us"),
        "{}",
        stats(&per)
    );
    core_mean
}

/// Where the session's tick goes: the same steps the session takes, timed one by one
/// on a mirror (no change to `crates/`). Base watchers (rsi14 + macd recomputed).
fn section_breakdown(n: usize, doc: &Document, feed: &[Tick], msgs: &[String]) {
    let mut d = doc.clone();
    let (mut parse, mut clone, mut apply, mut drop_old, mut recompute) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut rec_a, mut rec_b) = (Vec::new(), Vec::new());
    let watched: Vec<_> = study_series(doc)
        .into_iter()
        .filter(|(id, _, _)| id == "rsi14" || id == "macd")
        .collect();
    for (m, tk) in msgs.iter().zip(feed) {
        let t = Instant::now();
        let v: Value = serde_json::from_str(m).unwrap();
        let msg: ClientMsg = serde_path_to_error::deserialize(&v).unwrap();
        std::hint::black_box(&msg);
        parse.push(us(t));
        let t = Instant::now();
        let mut next = d.clone();
        clone.push(us(t));
        let t = Instant::now();
        let inv = apply_change(&mut next, &tk.change()).unwrap();
        std::hint::black_box(inv);
        apply.push(us(t));
        let t = Instant::now();
        d = next;
        drop_old.push(us(t));
        let bars = px_bars(&d);
        let mut parts = [0.0; 2];
        for (k, (_, name, params)) in watched.iter().enumerate() {
            let t = Instant::now();
            std::hint::black_box(
                qp_compute::compute_bars(name, params, Field::Close, bars).unwrap(),
            );
            parts[k] = us(t);
        }
        recompute.push(parts[0] + parts[1]);
        rec_a.push(parts[0]);
        rec_b.push(parts[1]);
    }
    let p = |k: &str, v: &[f64]| kv!(format!("n{n}.breakdown.{k}_us"), "{}", stats(v));
    p("parse_value_and_typed", &parse);
    p("doc_clone", &clone);
    p("apply_change", &apply);
    p("drop_old_doc", &drop_old);
    p("recompute_watched_rsi14_macd", &recompute);
    p(&format!("recompute_{}", watched[0].0), &rec_a);
    p(&format!("recompute_{}", watched[1].0), &rec_b);
    let sum = mean(&parse) + mean(&clone) + mean(&apply) + mean(&drop_old) + mean(&recompute);
    kv!(format!("n{n}.breakdown.sum_of_means_us"), "{sum:.2}");
    // Serializing the outbound lines of one tick (transport side, not in handle_raw).
    let mut s = live_session(doc, Watch::Base);
    let mut ser = Vec::new();
    let mut bytes = 0usize;
    for m in msgs.iter().take(120) {
        let lines = s.handle_raw("f", m);
        let t = Instant::now();
        for l in &lines {
            bytes += serde_json::to_string(&l.msg).unwrap().len();
        }
        ser.push(us(t));
    }
    kv!(
        format!("n{n}.breakdown.serialize_outbound_us"),
        "{} ; bytes/tick {:.0}",
        stats(&ser),
        bytes as f64 / ser.len() as f64
    );
}

/// A command with a selector vs the same command by id (set_encoding on the RSI layer).
fn section_selector(cfg: &Cfg, n: usize, doc: &Document) {
    let mut s = session(doc.clone(), 0);
    hello(&mut s, "o", "arkin", "desktop");
    let view = &s.document().views[0];
    let layer = view
        .layers
        .iter()
        .find(|l| l.series.as_deref() == Some("rsi14"))
        .and_then(|l| l.id.clone())
        .expect("rsi layer id");
    let vid = view.id.clone();
    let reps = cfg.reps.max(15) * 4;
    let mut by_id = Vec::new();
    let mut by_sel = Vec::new();
    let mut resolved = String::new();
    for i in 0..reps {
        let w = 2 + (i % 2);
        let m_id = json!({"type":"command","id":format!("ei{i}"),"ops":[
            {"op":"set_encoding","view":vid,"layer":layer,"encoding":{"width":w}}]})
        .to_string();
        let m_sel = json!({"type":"command","id":format!("es{i}"),"ops":[
            {"op":"set_encoding","view":{},"layer":{"shows":{"study":"rsi"}},"encoding":{"width":w + 2}}]})
        .to_string();
        let t = Instant::now();
        let l = s.handle_raw("o", &m_id);
        by_id.push(us(t));
        assert!(!is_error(&l), "{:?}", l[0].msg);
        let t = Instant::now();
        let l = s.handle_raw("o", &m_sel);
        by_sel.push(us(t));
        assert!(!is_error(&l), "{:?}", l[0].msg);
        if let ServerMsg::Ack { resolved: r, .. } = &l[0].msg {
            resolved = format!("{r:?}");
        }
    }
    let (clone_us, _) = bench(cfg.reps, || s.document().clone());
    kv!(format!("n{n}.selector.by_id_us"), "{}", stats(&by_id));
    kv!(
        format!("n{n}.selector.by_selector_us"),
        "{}",
        stats(&by_sel)
    );
    kv!(
        format!("n{n}.selector.diff_p50_us"),
        "{:.1} ; one doc clone (median) {:.1} ; resolved {resolved}",
        pct(&by_sel, 0.5) - pct(&by_id, 0.5),
        clone_us
    );
}

/// Only the session tick loop, for a sampling profiler.
fn profile(n: usize, ticks: usize) {
    let doc = gen_doc(10_000, n);
    let feed = gen_feed(px_bars(&doc).last().unwrap(), ticks);
    let msgs: Vec<String> = feed
        .iter()
        .enumerate()
        .map(|(i, t)| tick_msg(i, t))
        .collect();
    let mut s = live_session(&doc, Watch::Base);
    let t = Instant::now();
    for m in &msgs {
        std::hint::black_box(s.handle_raw("f", m));
    }
    eprintln!(
        "profile: studies {n} ticks {ticks} mean {:.1} us",
        us(t) / ticks as f64
    );
}
