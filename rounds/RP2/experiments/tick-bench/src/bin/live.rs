//! RP2/T5 — per-tick cost of the session's live path vs the compute core doing the same
//! work alone (criterion 4). Same document, feed and watcher layouts as T1 (`main.rs`),
//! two paths:
//!
//! - `inline`: each tick is a `command` (`append_bars` / `update_last_bar`) from the feed
//!   connection through `Session::handle_raw` (JSON parse included);
//! - `bound`: `px` is a bound series (THYAO 1d, `last: 10000`, the same bars given as
//!   `init.data`); each tick is a field-level `DataEvent` JSON (`bar`, or `patch` with only
//!   the fields that changed) parsed and handed to `Session::data_event`.
//!
//! Denominator (orchestrator decision, T5): the core advancing **every** study series of
//! the document (all rest on `px`): `push` for a new bar, `update_last` for a revision,
//! outputs kept in columns — plus decoding the same input message (the command JSON into
//! `ClientMsg`, or the `DataEvent` JSON), measured as whole-loop time (long loop, best and
//! median of `reps`). Also reported without the decode.
//!
//! Uses only API present before T5 (`Session::new`, `handle_raw`, `data_event`,
//! `provider`), so the same file builds against the T1/T4 code for the "before" column
//! (`run-live.sh`).
//!
//! Usage: `live [bars] [ticks] [reps] [studies,...] [paths] [watches]` (defaults
//! `10000 600 15 5,50,500 inline,bound t6d,base,all`); `live profile <inline|bound> <t6d|base|all> <n> <ticks>`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use qp_compute::{create, Field, Study};
use qp_schema::data::{DataChange, DataEvent};
use qp_schema::protocol::{ClientMsg, Op, ServerMsg, SessionInit};
use qp_schema::{Bar, Document, SeriesData, SeriesSource};
use qp_session::Session;
use serde_json::{json, Value};

const THYAO: &str = include_str!("../../../../../../fixtures/protocol/docs/thyao.json");
const INIT: &str = include_str!("../../../../../../fixtures/protocol/init/thyao.json");
const DAY: i64 = 86_400_000;
const BASE_STUDIES: usize = 3;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

fn pct(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[((s.len() as f64 - 1.0) * p).round() as usize]
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

macro_rules! kv {
    ($k:expr, $($arg:tt)*) => { println!("{} = {}", $k, format!($($arg)*)) };
}

// --- scenario (as T1) ------------------------------------------------------

fn extra_study(i: usize) -> (&'static str, Value) {
    const K: [f64; 3] = [1.5, 2.0, 2.5];
    let f = |x: usize| x as f64;
    match i % 5 {
        0 => ("sma", json!({"period": f(5 + (i * 7) % 196)})),
        1 => ("ema", json!({"period": f(5 + (i * 11) % 196)})),
        2 => ("rsi", json!({"period": f(2 + (i * 3) % 49)})),
        3 => {
            let fast = 5 + i % 10;
            (
                "macd",
                json!({"fast": fast as f64, "slow": (fast + 10 + (i * 3) % 20) as f64, "signal": (5 + i % 7) as f64}),
            )
        }
        _ => (
            "bollinger",
            json!({"period": f(10 + (i * 5) % 41), "k": K[i % 3]}),
        ),
    }
}

/// T1's document (px extended to `n_bars`, `n_studies` study series); `bound`: px is a
/// bound series instead and its bars are returned for the provider.
fn gen_doc(n_bars: usize, n_studies: usize, bound: bool) -> (Document, Vec<Bar>) {
    let mut v: Value = serde_json::from_str(THYAO).unwrap();
    let instrument = v["series"][0]["instrument"].clone();
    let series = v["series"].as_array_mut().unwrap();
    for i in 0..n_studies.saturating_sub(BASE_STUDIES) {
        let (study, params) = extra_study(i);
        series.push(json!({
            "id": format!("s{i:03}"), "instrument": instrument, "interval": "1d",
            "source": {"kind": "study", "study": study, "inputs": ["px"], "params": params}
        }));
    }
    let mut doc: Document = serde_json::from_value(v).unwrap();
    let px = doc.series.iter_mut().find(|s| s.id == "px").unwrap();
    let Some(SeriesData::Bars { bars }) = &mut px.data else {
        panic!("px has bars")
    };
    let mut rng = Rng(0x5444_3648_414e_444f);
    while bars.len() < n_bars {
        let last = bars.last().unwrap().clone();
        let o = last.c;
        let c = r2((o * (1.0 + (rng.unit() - 0.5) * 0.04)).max(1.0));
        let h = r2(o.max(c) * (1.0 + rng.unit() * 0.01));
        let l = r2(o.min(c) * (1.0 - rng.unit() * 0.01));
        let v = r2(400_000.0 + rng.unit() * 2_000_000.0);
        bars.push(Bar { t: last.t + DAY, o, h, l, c, v });
    }
    let all = bars.clone();
    if bound {
        px.data = None;
        px.source = serde_json::from_value(json!({"kind":"bind","symbol":"THYAO","interval":"1d",
            "window":{"last": n_bars}}))
        .unwrap();
    }
    (doc, all)
}

#[derive(Clone)]
enum Tick {
    Append(Bar),
    Update(Bar),
}

/// As T1: per simulated second one new bar then 59 revisions (seed 0xFEED).
fn gen_feed(last: &Bar, n: usize) -> Vec<Tick> {
    let mut rng = Rng(0xFEED);
    let mut out = Vec::with_capacity(n);
    let mut prev = last.clone();
    while out.len() < n {
        let o = prev.c;
        let mut b = Bar { t: prev.t + DAY, o, h: o, l: o, c: o, v: r2(rng.unit() * 1000.0) };
        out.push(Tick::Append(b.clone()));
        for _ in 1..60 {
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

/// The tick messages: commands (inline) or field-level data events (bound: a revision
/// carries only the fields that changed against the previous state of the bar).
fn messages(feed: &[Tick], bound: bool) -> Vec<String> {
    let mut prev: Option<Bar> = None;
    feed.iter()
        .enumerate()
        .map(|(i, t)| {
            if !bound {
                let op = match t {
                    Tick::Append(b) => json!({"op":"append_bars","series":"px","bars":[b]}),
                    Tick::Update(b) => json!({"op":"update_last_bar","series":"px","bar":b}),
                };
                return json!({"type":"command","id":format!("t{i}"),"ops":[op]}).to_string();
            }
            let change = match t {
                Tick::Append(b) => json!({"kind":"bar","bar":b}),
                Tick::Update(b) => {
                    let p = prev.as_ref().unwrap();
                    let mut f = serde_json::Map::new();
                    for (k, a, z) in [("o", p.o, b.o), ("h", p.h, b.h), ("l", p.l, b.l), ("c", p.c, b.c), ("v", p.v, b.v)] {
                        if a != z {
                            f.insert(k.to_owned(), json!(z));
                        }
                    }
                    json!({"kind":"patch","t":b.t,"fields":f})
                }
            };
            prev = Some(match t {
                Tick::Append(b) | Tick::Update(b) => b.clone(),
            });
            json!({"feed":{"symbol":"THYAO","interval":"1d"},"seq":i + 1,"change":change}).to_string()
        })
        .collect()
}

// --- session -------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Watch {
    T6d,
    Base,
    All,
}

impl Watch {
    fn name(self) -> &'static str {
        match self {
            Self::T6d => "t6d",
            Self::Base => "base",
            Self::All => "all",
        }
    }
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<ServerMsg> {
    s.handle_raw(conn, &msg.to_string()).into_iter().map(|l| l.msg).collect()
}

fn hello(s: &mut Session, conn: &str, principal: &str) {
    let r = send(s, conn, &json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":"headless"}}));
    assert!(matches!(r[0], ServerMsg::Welcome { .. }), "{:?}", r[0]);
}

fn subscribe(s: &mut Session, conn: &str, id: &str, topic: &Value) {
    let r = send(s, conn, &json!({"type":"subscribe","id":id,"topic":topic}));
    assert!(matches!(r[0], ServerMsg::Ack { .. }), "{:?}", r[0]);
}

fn studies(doc: &Document) -> Vec<(String, String, BTreeMap<String, f64>)> {
    doc.series
        .iter()
        .filter_map(|s| match &s.source {
            SeriesSource::Study { study, params, .. } => Some((s.id.clone(), study.clone(), params.clone())),
            _ => None,
        })
        .collect()
}

/// A live session with the watchers of `w` (as T1), every study computed beforehand
/// (a view compile would do the same) so that the loop measures ticks only.
fn live_session(doc: &Document, bars: &[Bar], bound: bool, w: Watch) -> Session {
    let mut init: SessionInit = serde_json::from_str(INIT).unwrap();
    init.document = qp_schema::protocol::DocumentRef::Inline(Box::new(doc.clone()));
    if bound {
        init.data = Some(
            serde_json::from_value(json!({
                "feeds":[{"feed":{"symbol":"THYAO","interval":"1d"},"bars":bars}],
                "entitlements":{"host":["*"],"arkin":["*"],"viewer":["*"]}
            }))
            .unwrap(),
        );
    }
    let mut s = Session::new(init, Path::new(".")).unwrap();
    hello(&mut s, "u", "arkin");
    hello(&mut s, "f", "feed");
    subscribe(&mut s, "u", "sub-ch", &json!({"kind":"changes"}));
    if w != Watch::T6d {
        subscribe(&mut s, "u", "sub-macd", &json!({"kind":"condition","condition":
            {"kind":"threshold","series":"macd","op":"cross_above","value":0.0}}));
    }
    if w == Watch::All {
        for (k, (id, _, _)) in studies(doc).iter().enumerate() {
            let conn = format!("w{}", k / 32);
            if k % 32 == 0 {
                hello(&mut s, &conn, "viewer");
            }
            subscribe(&mut s, &conn, &format!("all-{id}"), &json!({"kind":"condition","condition":
                {"kind":"threshold","series":id,"op":"gt","value":1e12}}));
        }
    }
    let _ = s.provider();
    s
}

fn tick_session(s: &mut Session, bound: bool, m: &str) -> usize {
    if bound {
        let ev: DataEvent = serde_json::from_str(m).unwrap();
        s.data_event(&ev).unwrap().len()
    } else {
        s.handle_raw("f", m).len()
    }
}

// --- core alone ------------------------------------------------------------

struct CoreStudy {
    study: Box<dyn Study>,
    outs: Vec<Vec<Option<f64>>>,
    buf: Vec<Option<f64>>,
}

struct Core {
    studies: Vec<CoreStudy>,
    last: Bar,
}

impl Core {
    /// `full`: output columns hold every bar (as the session keeps them for drawing);
    /// else only the bars of the stream (T1's core: a small working set).
    fn new(doc: &Document, bars: &[Bar], full: bool) -> Self {
        let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
        let studies = studies(doc)
            .into_iter()
            .map(|(_, name, params)| {
                let mut study = create(&name, &params, Field::Close).unwrap();
                study.extend(&[&close]);
                let n = study.outputs().len();
                let mut outs = vec![Vec::with_capacity(close.len() + 64); n];
                if full {
                    for o in &mut outs {
                        o.resize(close.len(), None);
                    }
                } else {
                    outs = vec![Vec::new(); n];
                }
                CoreStudy { study, outs, buf: vec![None; n] }
            })
            .collect();
        Self { studies, last: bars.last().unwrap().clone() }
    }

    fn step(&mut self, append: bool, c: f64) {
        for s in &mut self.studies {
            if append {
                s.study.push(&[c], &mut s.buf);
                for (col, v) in s.outs.iter_mut().zip(&s.buf) {
                    col.push(*v);
                }
            } else {
                s.study.update_last(&[c], &mut s.buf);
                for (col, v) in s.outs.iter_mut().zip(&s.buf) {
                    match col.last_mut() {
                        Some(last) => *last = *v,
                        None => col.push(*v),
                    }
                }
            }
        }
    }

    /// Decodes the message as it arrives, then advances every study. `via_value`: decode
    /// through `serde_json::Value` first (as the session does; faster for the internally
    /// tagged `ClientMsg`).
    fn tick_msg(&mut self, bound: bool, via_value: bool, m: &str) {
        if bound {
            let ev: DataEvent = if via_value {
                serde_json::from_value(serde_json::from_str::<Value>(m).unwrap()).unwrap()
            } else {
                serde_json::from_str(m).unwrap()
            };
            match ev.change {
                DataChange::Bar { bar } => {
                    self.last = bar;
                    self.step(true, self.last.c);
                }
                DataChange::Patch { fields, .. } => {
                    fields.apply_to(&mut self.last);
                    self.step(false, self.last.c);
                }
            }
        } else {
            let msg: ClientMsg = if via_value {
                serde_json::from_value(serde_json::from_str::<Value>(m).unwrap()).unwrap()
            } else {
                serde_json::from_str(m).unwrap()
            };
            let ClientMsg::Command { ops, .. } = msg else { unreachable!() };
            match &ops[0] {
                Op::AppendBars { bars, .. } => self.step(true, bars[0].c),
                Op::UpdateLastBar { bar, .. } => self.step(false, bar.c),
                _ => unreachable!(),
            }
        }
    }

    fn last_values(&self) -> Vec<Option<u64>> {
        self.studies
            .iter()
            .flat_map(|s| s.outs.iter().map(|c| c.last().copied().flatten().map(f64::to_bits)))
            .collect()
    }
}

/// Whole-loop time per tick over `reps` fresh runs: (median, best).
fn loop_time(reps: usize, mut setup: impl FnMut() -> Box<dyn FnMut()>) -> (f64, f64, usize) {
    let mut v = Vec::new();
    let mut n = 0;
    for _ in 0..reps {
        let mut f = setup();
        let t = Instant::now();
        f();
        v.push(us(t));
        n += 1;
    }
    (pct(&v, 0.5), pct(&v, 0.0), n)
}

struct Cfg {
    bars: usize,
    ticks: usize,
    reps: usize,
}

fn section(cfg: &Cfg, n: usize, bound: bool, watches: &[Watch]) {
    let path = if bound { "bound" } else { "inline" };
    let (doc, bars) = gen_doc(cfg.bars, n, bound);
    let feed = gen_feed(bars.last().unwrap(), cfg.ticks);
    let msgs = messages(&feed, bound);
    let k = format!("n{n}.{path}");
    kv!(format!("{k}.msg"), "bytes_per_msg {:.0}", msgs.iter().map(String::len).sum::<usize>() as f64 / msgs.len() as f64);
    let ticks = cfg.ticks as f64;
    // Core alone: decode + advance every study (whole loop; the faster of the two
    // decodes), and advance only.
    let mut best = (f64::MAX, f64::MAX, "");
    for (via, name) in [(false, "direct"), (true, "via_value")] {
        let (m, b, _) = loop_time(cfg.reps, || {
            let mut c = Core::new(&doc, &bars, true);
            let msgs = msgs.clone();
            Box::new(move || {
                for m in &msgs {
                    c.tick_msg(bound, via, m);
                }
                std::hint::black_box(&c.studies.len());
            })
        });
        kv!(format!("{k}.core_parse_{name}.loop_us"), "median {:.3} best {:.3}", m / ticks, b / ticks);
        if m < best.0 {
            best = (m, b, name);
        }
    }
    let (cm, cb, decode) = best;
    let mut adv = Vec::new();
    for full in [true, false] {
        let (m, b, _) = loop_time(cfg.reps, || {
            let mut c = Core::new(&doc, &bars, full);
            let feed = feed.clone();
            Box::new(move || {
                for t in &feed {
                    match t {
                        Tick::Append(b) => c.step(true, b.c),
                        Tick::Update(b) => c.step(false, b.c),
                    }
                }
                std::hint::black_box(&c.studies.len());
            })
        });
        adv.push((m / ticks, b / ticks));
    }
    // T1-style core with decode: the lean core plus the decode cost measured above.
    let decode_only = (cm / ticks - adv[0].0).max(0.0);
    let core_parse = cm / ticks;
    let core = adv[0].0;
    let lean_parse = adv[1].0 + decode_only;
    kv!(format!("{k}.core_parse.loop_us"), "median {:.3} best {:.3} (reps {}, decode {decode}; full columns)", core_parse, cb / ticks, cfg.reps);
    kv!(format!("{k}.core.loop_us"), "median {:.3} best {:.3} (full columns, no decode)", core, adv[0].1);
    kv!(format!("{k}.core_lean.loop_us"), "median {:.3} best {:.3} (stream-only columns as T1, no decode); with decode {:.3}", adv[1].0, adv[1].1, lean_parse);
    // Reference values for the bit check.
    let mut cref = Core::new(&doc, &bars, true);
    for m in &msgs {
        cref.tick_msg(bound, false, m);
    }
    for &w in watches {
        let kw = format!("{k}.{}", w.name());
        // Per-tick distribution (one run, timer per tick).
        let mut s = live_session(&doc, &bars, bound, w);
        let mut per = Vec::with_capacity(msgs.len());
        let mut lines = 0;
        for m in &msgs {
            let t = Instant::now();
            lines += tick_session(&mut s, bound, m);
            per.push(us(t));
        }
        let appends: Vec<f64> = feed.iter().zip(&per).filter(|(t, _)| matches!(t, Tick::Append(_))).map(|(_, x)| *x).collect();
        kv!(format!("{kw}.session.per_tick_us"), "mean {:.2} p50 {:.2} p99 {:.2} max {:.2} append_mean {:.2} (n {})",
            mean(&per), pct(&per, 0.5), pct(&per, 0.99), pct(&per, 1.0), mean(&appends), per.len());
        // Bit check: every study's last value in the session = the core's.
        let mut sv = Vec::new();
        {
            use qp_grammar::DataProvider;
            let p = s.provider();
            for (id, _, _) in studies(&doc) {
                let mut fields: Vec<String> = Vec::new();
                for f in ["bandwidth", "histogram", "lower", "macd", "middle", "signal", "upper", "sma", "ema", "rsi"] {
                    if p.values(&id, f).is_some() {
                        fields.push(f.to_owned());
                    }
                }
                // Core outputs are in the study's own order; compare as a sorted multiset per study.
                let mut vals: Vec<Option<u64>> = fields.iter().map(|f| p.values(&id, f).unwrap().last().copied().flatten().map(f64::to_bits)).collect();
                vals.sort();
                sv.push(vals);
            }
        }
        let mut cv = Vec::new();
        for st in &cref.studies {
            let mut vals: Vec<Option<u64>> = st.outs.iter().map(|c| c.last().copied().flatten().map(f64::to_bits)).collect();
            vals.sort();
            cv.push(vals);
        }
        let _ = cref.last_values();
        kv!(format!("{kw}.check"), "session_eq_core {} lines {} doc {:016x}", sv == cv, lines, fnv(&serde_json::to_vec(s.document()).unwrap()));
        // Whole loop (fresh session per rep, built outside the timer).
        let (sm, sb, _) = loop_time(cfg.reps.min(7), || {
            let mut s = live_session(&doc, &bars, bound, w);
            let msgs = msgs.clone();
            Box::new(move || {
                for m in &msgs {
                    std::hint::black_box(tick_session(&mut s, bound, m));
                }
            })
        });
        let sess = sm / ticks;
        kv!(format!("{kw}.session.loop_us"), "median {:.3} best {:.3}", sess, sb / ticks);
        kv!(format!("{kw}.ratio"), "overhead/core_parse {:.2} (target <= 3) ; overhead/core_lean_parse {:.2} ; overhead/core(no parse) {:.1} ; overhead/core_lean(no parse) {:.1}",
            (sess - core_parse) / core_parse, (sess - lean_parse) / lean_parse, (sess - core) / core, (sess - adv[1].0) / adv[1].0);
    }
}

fn profile(path: &str, watch: &str, n: usize, ticks: usize) {
    let bound = path == "bound";
    let w = match watch {
        "t6d" => Watch::T6d,
        "base" => Watch::Base,
        _ => Watch::All,
    };
    let (doc, bars) = gen_doc(10_000, n, bound);
    let feed = gen_feed(bars.last().unwrap(), ticks);
    let msgs = messages(&feed, bound);
    let mut s = live_session(&doc, &bars, bound, w);
    let t = Instant::now();
    for m in &msgs {
        std::hint::black_box(tick_session(&mut s, bound, m));
    }
    eprintln!("profile {path} {watch} n {n}: mean {:.2} us", us(t) / ticks as f64);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).map(String::as_str) == Some("profile") {
        profile(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap());
        return;
    }
    let cfg = Cfg {
        bars: a.get(1).and_then(|x| x.parse().ok()).unwrap_or(10_000),
        ticks: a.get(2).and_then(|x| x.parse().ok()).unwrap_or(600),
        reps: a.get(3).and_then(|x| x.parse().ok()).unwrap_or(15),
    };
    let counts: Vec<usize> = a.get(4).map_or("5,50,500", String::as_str).split(',').map(|x| x.parse().unwrap()).collect();
    let paths: Vec<String> = a.get(5).map_or("inline,bound", String::as_str).split(',').map(str::to_owned).collect();
    let watches: Vec<Watch> = a
        .get(6)
        .map_or("t6d,base,all", String::as_str)
        .split(',')
        .map(|w| match w {
            "t6d" => Watch::T6d,
            "base" => Watch::Base,
            _ => Watch::All,
        })
        .collect();
    let target = if cfg!(target_arch = "wasm32") { "wasm32" } else { std::env::consts::ARCH };
    println!("# RP2/T5 live bench");
    kv!("target", "{target}");
    kv!("config", "bars {} ticks {} reps {} studies {:?} paths {:?}", cfg.bars, cfg.ticks, cfg.reps, counts, paths);
    for n in counts {
        for p in &paths {
            section(&cfg, n, p == "bound", &watches);
        }
    }
}
