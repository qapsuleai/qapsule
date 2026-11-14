//! RD1/T6d — hand-over of authority (K21-3) vs server authority with a thin mirror.
//!
//! One binary, run natively and as `wasm32-wasip1` under Node. Prints `key = value`
//! lines grouped by section; timings in milliseconds (or microseconds where noted).
//! Everything is deterministic (fixed seed) except the timings themselves.
//!
//! Usage: `t6d-handover [bars] [live_secs] [reps]` (defaults 10000 60 15).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use qp_compute::{create, default_params, run, Field, Study};
use qp_schema::protocol::{
    ChangeOp, ClientMsg, DocumentRef, Event, LogEntry, Op, OutboundLine, QueryResult, Seq,
    ServerMsg, SessionInit, WriteMode,
};
use qp_schema::{Bar, CapsuleState, CompareOp, Condition, Document, SeriesData, SeriesSource};
use qp_session::{apply_change, replay, touched_ids, Session};
use serde_json::{json, Value};

const THYAO: &str = include_str!("../../../fixtures/protocol/docs/thyao.json");
const INIT: &str = include_str!("../../../fixtures/protocol/init/thyao.json");
const DAY: i64 = 86_400_000;

// ---------------------------------------------------------------------------
// Small utilities
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

fn digest_cols(cols: &[(String, Vec<Option<f64>>)]) -> String {
    let mut buf = Vec::new();
    for (name, col) in cols {
        buf.extend_from_slice(name.as_bytes());
        for v in col {
            match v {
                Some(x) => buf.extend_from_slice(&x.to_bits().to_le_bytes()),
                None => buf.extend_from_slice(b"NONE____"),
            }
        }
    }
    format!("{:016x}", fnv(&buf))
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

/// Median of a sample (sorted copy).
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}

fn pct(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let i = ((s.len() as f64 - 1.0) * p).round() as usize;
    s[i]
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Runs `f` `reps` times, returns the median and min in ms.
fn bench<R>(reps: usize, mut f: impl FnMut() -> R) -> (f64, f64, R) {
    let mut times = Vec::with_capacity(reps);
    let mut last = None;
    for _ in 0..reps {
        let t = Instant::now();
        let r = f();
        times.push(ms(t));
        last = Some(r);
    }
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    (median(&times), min, last.unwrap())
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

/// The fixture document with `px` extended to `n` bars by a fixed-seed random walk.
fn gen_doc(n: usize) -> Document {
    let mut doc: Document = serde_json::from_str(THYAO).unwrap();
    let px = doc.series.iter_mut().find(|s| s.id == "px").unwrap();
    let Some(SeriesData::Bars { bars }) = &mut px.data else {
        panic!("px has bars")
    };
    let mut rng = Rng(0x5444_3648_414e_444f); // "TD6HANDO"
    while bars.len() < n {
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
}

/// `secs` seconds of a live feed: per second one new bar then `per_sec - 1` revisions.
fn gen_feed(last: &Bar, secs: usize, per_sec: usize, seed: u64) -> Vec<Tick> {
    let mut rng = Rng(seed);
    let mut out = Vec::with_capacity(secs * per_sec);
    let mut prev = last.clone();
    for _ in 0..secs {
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
        for _ in 1..per_sec {
            b.c = r2((b.c * (1.0 + (rng.unit() - 0.5) * 0.004)).max(1.0));
            b.h = b.h.max(b.c);
            b.l = b.l.min(b.c);
            b.v = r2(b.v + rng.unit() * 1000.0);
            out.push(Tick::Update(b.clone()));
        }
        prev = b;
    }
    out
}

fn cmd(id: &str, ops: Vec<Op>, base: Option<Seq>) -> String {
    serde_json::to_string(&ClientMsg::Command {
        id: id.to_owned(),
        ops,
        mode: WriteMode::Auto,
        base,
        reason: None,
    })
    .unwrap()
}

fn tick_cmd(i: usize, t: &Tick) -> String {
    cmd(&format!("t{i}"), vec![t.op()], None)
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

fn subscribe_changes(s: &mut Session, conn: &str) {
    let r = send(
        s,
        conn,
        &json!({"type":"subscribe","id":"sub-ch","topic":{"kind":"changes"}}),
    );
    assert!(matches!(r[0].msg, ServerMsg::Ack { .. }), "{:?}", r[0].msg);
}

fn is_error(lines: &[OutboundLine], conn: &str) -> bool {
    lines
        .iter()
        .any(|l| l.conn == conn && matches!(l.msg, ServerMsg::Error { .. }))
}

fn ack_seq(lines: &[OutboundLine], conn: &str) -> Option<Seq> {
    lines.iter().find_map(|l| match &l.msg {
        ServerMsg::Ack { seq, .. } if l.conn == conn => *seq,
        _ => None,
    })
}

fn error_code(lines: &[OutboundLine], conn: &str) -> Option<String> {
    lines.iter().find_map(|l| match &l.msg {
        ServerMsg::Error { error, .. } if l.conn == conn => {
            Some(serde_json::to_value(error.code).unwrap().to_string())
        }
        _ => None,
    })
}

/// Study series of a document: (id, study name, params, main output).
fn study_series(doc: &Document) -> Vec<(String, String, BTreeMap<String, f64>, String)> {
    doc.series
        .iter()
        .filter_map(|s| match &s.source {
            SeriesSource::Study {
                study,
                params,
                output,
                ..
            } => {
                let main = output.clone().unwrap_or_else(|| {
                    if study == "bollinger" {
                        "middle".to_owned()
                    } else {
                        study.clone()
                    }
                });
                Some((s.id.clone(), study.clone(), params.clone(), main))
            }
            _ => None,
        })
        .collect()
}

/// Batch outputs of every study series over the document's `px` bars, as
/// `(series.output, column)` for digests.
fn batch_outputs(doc: &Document) -> Vec<(String, Vec<Option<f64>>)> {
    let bars = px_bars(doc);
    let mut cols = Vec::new();
    for (id, study, params, _) in study_series(doc) {
        let out = qp_compute::compute_bars(&study, &params, Field::Close, bars).unwrap();
        for (name, col) in out.outputs {
            cols.push((format!("{id}.{name}"), col));
        }
    }
    cols
}

// ---------------------------------------------------------------------------
// Incremental device authority ("A at its best")
// ---------------------------------------------------------------------------

struct LiveStudy {
    id: String,
    study: Box<dyn Study>,
    names: Vec<String>,
    outs: Vec<Vec<Option<f64>>>,
    main: usize,
    buf: Vec<Option<f64>>,
}

/// A device-side authority that applies feed ticks in O(1): changes applied in place
/// (no document copy), studies advanced with `push`/`update_last`, live capsules'
/// invalidation evaluated on the streaming outputs. Writes the same log entries and
/// reaches the same document bytes as the reference `Session` (checked).
struct Live {
    doc: Document,
    log: Vec<LogEntry>,
    studies: Vec<LiveStudy>,
    clock: i64,
}

impl Live {
    /// `full = true`: run every study over history and keep all outputs (to draw);
    /// `false`: only attach with `Study::extend` (state only, no outputs kept).
    fn new(doc: Document, clock: i64, full: bool) -> Self {
        let close = Field::Close.column(px_bars(&doc));
        let mut studies = Vec::new();
        for (id, name, params, main) in study_series(&doc) {
            let mut study = create(&name, &params, Field::Close).unwrap();
            let names: Vec<String> = study.outputs().iter().map(|s| (*s).to_owned()).collect();
            let outs = if full {
                run(study.as_mut(), &[&close]).unwrap()
            } else {
                study.extend(&[&close]);
                vec![Vec::new(); names.len()]
            };
            let main = names.iter().position(|n| *n == main).unwrap();
            let buf = vec![None; names.len()];
            studies.push(LiveStudy {
                id,
                study,
                names,
                outs,
                main,
                buf,
            });
        }
        Self {
            doc,
            log: Vec::new(),
            studies,
            clock,
        }
    }

    fn last_two(&self, id: &str) -> Option<(Option<f64>, Option<f64>)> {
        if id == "px" {
            let b = px_bars(&self.doc);
            let n = b.len();
            return Some(((n >= 2).then(|| b[n - 2].c), (n >= 1).then(|| b[n - 1].c)));
        }
        let s = self.studies.iter().find(|s| s.id == id)?;
        let col = &s.outs[s.main];
        let n = col.len();
        Some((
            if n >= 2 { col[n - 2] } else { None },
            if n >= 1 { col[n - 1] } else { None },
        ))
    }

    /// Same semantics as the reference `eval_condition` at the last bar.
    fn holds(&self, c: &Condition) -> bool {
        let (op, (lp, ll), (rp, rl)) = match c {
            Condition::Threshold { series, op, value } => match self.last_two(series) {
                Some(l) => (*op, l, (Some(*value), Some(*value))),
                None => return false,
            },
            Condition::Compare { left, op, right } => {
                match (self.last_two(left), self.last_two(right)) {
                    (Some(l), Some(r)) => (*op, l, r),
                    _ => return false,
                }
            }
            Condition::Text { .. } => return false,
        };
        let (Some(a), Some(b)) = (ll, rl) else {
            return false;
        };
        let prev = match (lp, rp) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };
        match op {
            CompareOp::Lt => a < b,
            CompareOp::Le => a <= b,
            CompareOp::Gt => a > b,
            CompareOp::Ge => a >= b,
            CompareOp::CrossAbove => prev.is_some_and(|(pa, pb)| pa < pb) && a >= b,
            CompareOp::CrossBelow => prev.is_some_and(|(pa, pb)| pa > pb) && a <= b,
        }
    }

    fn tick(&mut self, id: &str, t: &Tick) {
        let change = match t {
            Tick::Append(b) => ChangeOp::AppendBars {
                series: "px".to_owned(),
                bars: vec![b.clone()],
            },
            Tick::Update(b) => ChangeOp::SetLastBar {
                series: "px".to_owned(),
                bar: b.clone(),
            },
        };
        let inverse = apply_change(&mut self.doc, &change).unwrap();
        let summary = match t {
            Tick::Append(_) => "append_bars px +1",
            Tick::Update(_) => "update_last_bar px",
        };
        self.log.push(LogEntry {
            seq: self.log.len() as Seq + 1,
            at: self.clock,
            author: "feed".to_owned(),
            approved_by: None,
            cause: Some(id.to_owned()),
            reason: None,
            request: vec![t.op()],
            changes: vec![change],
            inverse: vec![inverse],
            undoes: None,
            summary: summary.to_owned(),
        });
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
                        if let Some(last) = col.last_mut() {
                            *last = *v;
                        }
                    }
                }
            }
        }
        // §8.1: after a bar change, every live capsule's finding invalidation.
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| c.state == CapsuleState::Live)
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        for (cap, fnd) in candidates {
            let Some(f) = self.doc.findings.iter().find(|f| f.id == fnd) else {
                continue;
            };
            if self.holds(&f.invalidation.clone()) {
                let change = ChangeOp::SetCapsuleState {
                    capsule: cap.clone(),
                    state: CapsuleState::Invalidated,
                };
                let inv = apply_change(&mut self.doc, &change).unwrap();
                self.log.push(LogEntry {
                    seq: self.log.len() as Seq + 1,
                    at: self.clock,
                    author: "qp".to_owned(),
                    approved_by: None,
                    cause: None,
                    reason: None,
                    request: Vec::new(),
                    changes: vec![change],
                    inverse: vec![inv],
                    undoes: None,
                    summary: format!("invalidate {cap}: invalidation of {fnd} holds"),
                });
            }
        }
    }

    fn outputs(&self) -> Vec<(String, Vec<Option<f64>>)> {
        let mut cols = Vec::new();
        for s in &self.studies {
            for (n, c) in s.names.iter().zip(&s.outs) {
                cols.push((format!("{}.{n}", s.id), c.clone()));
            }
        }
        cols
    }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

/// Snapshot as columnar JSON: `px.data` becomes `{"kind":"columns","t":[…],…}`.
fn columnar(doc: &Document, delta_t: bool) -> String {
    let mut v = serde_json::to_value(doc).unwrap();
    let bars = px_bars(doc);
    let col = |f: fn(&Bar) -> f64| bars.iter().map(f).collect::<Vec<_>>();
    let t: Vec<i64> = if delta_t {
        let mut prev = 0;
        bars.iter()
            .map(|b| {
                let d = b.t - prev;
                prev = b.t;
                d
            })
            .collect()
    } else {
        bars.iter().map(|b| b.t).collect()
    };
    let data = json!({"kind": if delta_t {"columns_dt"} else {"columns"}, "t": t,
        "o": col(|b| b.o), "h": col(|b| b.h), "l": col(|b| b.l), "c": col(|b| b.c), "v": col(|b| b.v)});
    for s in v["series"].as_array_mut().unwrap() {
        if s["id"] == "px" {
            s["data"] = data.clone();
        }
    }
    v.to_string()
}

fn sizes(label: &str, s: &[u8]) {
    let t = Instant::now();
    let g = gzip(s);
    let gz_ms = ms(t);
    kv!(
        format!("{label}.bytes"),
        "{} raw, {} gzip ({:.1}%), gzip_ms {:.2}",
        s.len(),
        g.len(),
        100.0 * g.len() as f64 / s.len() as f64,
        gz_ms
    );
}

struct Cfg {
    bars: usize,
    secs: usize,
    reps: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cfg = Cfg {
        bars: args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10_000),
        secs: args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60),
        reps: args.get(3).and_then(|a| a.parse().ok()).unwrap_or(15),
    };
    let target = if cfg!(target_arch = "wasm32") {
        "wasm32"
    } else {
        std::env::consts::ARCH
    };
    println!("# T6d hand-over experiment");
    kv!("target", "{target}");
    kv!(
        "config",
        "bars {} live_secs {} ticks/s 60 reps {}",
        cfg.bars,
        cfg.secs,
        cfg.reps
    );

    let doc = gen_doc(cfg.bars);
    kv!("doc.digest", "{}", digest_json(&doc));
    kv!("doc.last_bar_t", "{}", px_bars(&doc).last().unwrap().t);
    let feed = gen_feed(px_bars(&doc).last().unwrap(), cfg.secs, 60, 0xFEED);
    kv!("feed.ticks", "{}", feed.len());

    let snapshot = section_snapshot(&cfg, &doc);
    section_ready(&cfg, &snapshot, &doc);
    let (a_doc, a_outs) = section_live_a(&cfg, &doc, &feed);
    section_live_b(&cfg, &doc, &feed, &snapshot, &a_doc, &a_outs);
    section_offline(&cfg, &doc, &feed);
}

/// 1. Server side: build and serialize the `document` snapshot.
fn section_snapshot(cfg: &Cfg, doc: &Document) -> String {
    println!("\n## 1. snapshot (server)");
    let mut server = session(doc.clone(), 0);
    hello(&mut server, "dev", "arkin", "phone");
    let q = json!({"type":"query","id":"snap","query":{"kind":"document"}}).to_string();
    let (med, min, snap) = bench(cfg.reps, || {
        let lines = server.handle_raw("dev", &q);
        serde_json::to_string(&lines[0].msg).unwrap()
    });
    kv!(
        "snapshot.query_and_serialize_ms",
        "median {med:.3} min {min:.3}"
    );
    let (med, min, _) = bench(cfg.reps, || serde_json::to_string(doc).unwrap());
    kv!(
        "snapshot.serialize_doc_only_ms",
        "median {med:.3} min {min:.3}"
    );
    let limit = server.limits().max_message_bytes;
    kv!(
        "snapshot.vs_max_message_bytes",
        "{} / {} ({})",
        snap.len(),
        limit,
        if snap.len() as u64 > limit {
            "EXCEEDS"
        } else {
            "fits"
        }
    );
    sizes("snapshot.rows_json", snap.as_bytes());
    sizes("snapshot.columnar_json", columnar(doc, false).as_bytes());
    sizes("snapshot.columnar_dt_json", columnar(doc, true).as_bytes());
    let bars_only = serde_json::to_string(px_bars(doc)).unwrap();
    sizes("snapshot.px_bars_only_json", bars_only.as_bytes());
    let mut nobars = doc.clone();
    for s in &mut nobars.series {
        if s.id == "px" {
            s.data = Some(SeriesData::Bars { bars: Vec::new() });
        }
    }
    sizes(
        "snapshot.doc_without_bars_json",
        serde_json::to_string(&nobars).unwrap().as_bytes(),
    );
    // C: precomputed study outputs shipped with the snapshot.
    let outs: Vec<_> = study_series(doc)
        .into_iter()
        .map(|(_, study, params, _)| {
            qp_compute::compute_bars(&study, &params, Field::Close, px_bars(doc)).unwrap()
        })
        .collect();
    let outs_json = serde_json::to_string(&outs).unwrap();
    sizes(
        "snapshot.precomputed_study_outputs_json",
        outs_json.as_bytes(),
    );
    snap
}

/// 2. Device side: from the snapshot bytes to a ready authority.
fn section_ready(cfg: &Cfg, snapshot: &str, doc: &Document) {
    println!("\n## 2. device ready (A)");
    let (m_parse, _, d) = bench(cfg.reps, || {
        let msg: ServerMsg = serde_json::from_str(snapshot).unwrap();
        let ServerMsg::Result {
            data: QueryResult::Document { document },
            ..
        } = msg
        else {
            panic!("snapshot")
        };
        *document
    });
    assert_eq!(digest_json(&d), digest_json(doc), "snapshot round-trip");
    kv!("ready.parse_snapshot_ms", "median {m_parse:.3}");
    let (m_sess, _, _) = bench(cfg.reps, || {
        let mut s = session(d.clone(), 1);
        hello(&mut s, "u", "arkin", "phone");
        hello(&mut s, "f", "feed", "headless");
        s
    });
    let (m_clone, _, _) = bench(cfg.reps, || d.clone());
    kv!(
        "ready.session_new_ms",
        "median {:.3} (incl. doc clone {:.3}, schema build, 2 hellos)",
        m_sess - m_clone,
        m_clone
    );
    let close_t = Instant::now();
    let close = Field::Close.column(px_bars(&d));
    let _ = ms(close_t);
    let (m_ext, _, _) = bench(cfg.reps, || {
        let mut v = Vec::new();
        for (_, name, params, _) in study_series(&d) {
            let mut st = create(&name, &params, Field::Close).unwrap();
            st.extend(&[&close]);
            v.push(st);
        }
        v
    });
    kv!(
        "ready.attach_studies_extend_ms",
        "median {m_ext:.3} (3 studies, state only)"
    );
    let (m_run, _, _) = bench(cfg.reps, || {
        let mut v = Vec::new();
        for (_, name, params, _) in study_series(&d) {
            let mut st = create(&name, &params, Field::Close).unwrap();
            v.push(run(st.as_mut(), &[&close]).unwrap());
        }
        v
    });
    kv!(
        "ready.attach_studies_run_all_outputs_ms",
        "median {m_run:.3} (3 studies, all outputs kept)"
    );
    // C: parse precomputed outputs instead of computing them (state still needs extend).
    let outs: Vec<_> = study_series(&d)
        .into_iter()
        .map(|(_, s, p, _)| qp_compute::compute_bars(&s, &p, Field::Close, px_bars(&d)).unwrap())
        .collect();
    let outs_json = serde_json::to_string(&outs).unwrap();
    let (m_pout, _, _) = bench(cfg.reps, || {
        serde_json::from_str::<Vec<qp_schema::StudyOutput>>(&outs_json).unwrap()
    });
    kv!("ready.C_parse_precomputed_outputs_ms", "median {m_pout:.3}");
    kv!(
        "ready.total_A_ms",
        "{:.3} (parse + session + extend) ; with all outputs {:.3}",
        m_parse + m_sess + m_ext,
        m_parse + m_sess + m_run
    );
    kv!(
        "ready.total_B_mirror_ms",
        "{:.3} (parse only; values come from the server)",
        m_parse
    );
    let _ = default_params("rsi");
}

/// 3. A: the device is the authority and applies the live feed itself.
fn section_live_a(
    cfg: &Cfg,
    doc: &Document,
    feed: &[Tick],
) -> (Document, Vec<(String, Vec<Option<f64>>)>) {
    println!("\n## 3. live on the device (A)");
    let msgs: Vec<String> = feed
        .iter()
        .enumerate()
        .map(|(i, t)| tick_cmd(i, t))
        .collect();
    let up: usize = msgs.iter().map(String::len).sum();
    kv!(
        "liveA.feed_upstream_bytes_per_tick",
        "{:.0} (feed -> authority, same in A and B)",
        up as f64 / msgs.len() as f64
    );

    // (a) Reference Session.
    let mut dev = session(doc.clone(), 1);
    hello(&mut dev, "u", "arkin", "phone");
    hello(&mut dev, "f", "feed", "headless");
    subscribe_changes(&mut dev, "u");
    let mut per = Vec::with_capacity(msgs.len());
    let mut per_append = Vec::new();
    let mut per_update = Vec::new();
    let t_all = Instant::now();
    for (m, t) in msgs.iter().zip(feed) {
        let t0 = Instant::now();
        let lines = dev.handle_raw("f", m);
        let dt = ms(t0) * 1e3;
        assert!(!is_error(&lines, "f"), "{:?}", lines[0].msg);
        per.push(dt);
        match t {
            Tick::Append(_) => per_append.push(dt),
            Tick::Update(_) => per_update.push(dt),
        }
    }
    let total = ms(t_all);
    kv!(
        "liveA.ref_session.per_tick_us",
        "mean {:.1} p50 {:.1} p99 {:.1} max {:.1}",
        mean(&per),
        pct(&per, 0.5),
        pct(&per, 0.99),
        pct(&per, 1.0)
    );
    kv!(
        "liveA.ref_session.append_vs_update_us",
        "append mean {:.1} ; update mean {:.1}",
        mean(&per_append),
        mean(&per_update)
    );
    kv!(
        "liveA.ref_session.cpu_share_at_60_per_s",
        "{:.2}% of one core (total {:.1} ms for {} s of feed)",
        100.0 * total / (cfg.secs as f64 * 1e3),
        total,
        cfg.secs
    );
    kv!("liveA.ref_session.log_entries", "{}", dev.log().len());
    let ref_doc = dev.document().clone();
    let ref_log = dev.log().to_vec();

    // (b) Incremental authority.
    let mut best = f64::INFINITY;
    let mut live_final = None;
    for _ in 0..cfg.reps.min(5) {
        let mut live = Live::new(doc.clone(), dev.clock(), true);
        let t0 = Instant::now();
        for (i, t) in feed.iter().enumerate() {
            live.tick(&format!("t{i}"), t);
        }
        best = best.min(ms(t0));
        live_final = Some(live);
    }
    let live = live_final.unwrap();
    // Per-tick sample including message parsing (as it arrives from a feed).
    let mut live2 = Live::new(doc.clone(), dev.clock(), true);
    let mut per2 = Vec::with_capacity(msgs.len());
    for (i, m) in msgs.iter().enumerate() {
        let t0 = Instant::now();
        let msg: ClientMsg = serde_json::from_str(m).unwrap();
        let ClientMsg::Command { ops, .. } = msg else {
            panic!()
        };
        let tick = match &ops[0] {
            Op::AppendBars { bars, .. } => Tick::Append(bars[0].clone()),
            Op::UpdateLastBar { bar, .. } => Tick::Update(bar.clone()),
            _ => panic!(),
        };
        live2.tick(&format!("t{i}"), &tick);
        per2.push(ms(t0) * 1e3);
    }
    kv!(
        "liveA.incremental.per_tick_us",
        "mean {:.2} (best of runs, no parse) ; with parse: mean {:.2} p50 {:.2} p99 {:.2} max {:.2}",
        best * 1e3 / feed.len() as f64,
        mean(&per2),
        pct(&per2, 0.5),
        pct(&per2, 0.99),
        pct(&per2, 1.0)
    );
    kv!(
        "liveA.incremental.cpu_share_at_60_per_s",
        "{:.4}% of one core",
        100.0 * best / (cfg.secs as f64 * 1e3)
    );
    kv!(
        "liveA.speedup_incremental_vs_ref",
        "{:.0}x",
        mean(&per) / (best * 1e3 / feed.len() as f64)
    );

    // Determinism: same document, same log, same study values.
    let d_ref = digest_json(&ref_doc);
    let d_live = digest_json(&live.doc);
    kv!("det.A.doc_digest.ref_session", "{d_ref}");
    kv!(
        "det.A.doc_digest.incremental",
        "{d_live} (equal: {})",
        d_ref == d_live
    );
    kv!(
        "det.A.log_digest",
        "ref {} incremental {} (equal: {})",
        digest_json(&ref_log),
        digest_json(&live.log),
        digest_json(&ref_log) == digest_json(&live.log)
    );
    let mut stream = live.outputs();
    let mut batch = batch_outputs(&live.doc);
    stream.sort_by(|a, b| a.0.cmp(&b.0));
    batch.sort_by(|a, b| a.0.cmp(&b.0));
    let bit_equal = stream.len() == batch.len()
        && stream.iter().zip(&batch).all(|(a, b)| {
            a.0 == b.0
                && a.1.len() == b.1.len()
                && a.1
                    .iter()
                    .zip(&b.1)
                    .all(|(x, y)| x.map(f64::to_bits) == y.map(f64::to_bits))
        });
    kv!(
        "det.A.study_values",
        "streaming {} batch {} (bit-equal over all {} bars x {} outputs: {bit_equal})",
        digest_cols(&stream),
        digest_cols(&batch),
        px_bars(&live.doc).len(),
        stream.len()
    );
    let caps: Vec<_> = live
        .doc
        .capsules
        .iter()
        .map(|c| format!("{}:{:?}", c.id, c.state))
        .collect();
    kv!("liveA.capsules_after", "{}", caps.join(","));
    kv!(
        "liveA.engine_entries",
        "{}",
        live.log.iter().filter(|e| e.author == "qp").count()
    );
    (ref_doc, stream)
}

/// 4. B: the server stays the authority; the device mirrors.
fn section_live_b(
    cfg: &Cfg,
    doc: &Document,
    feed: &[Tick],
    snapshot: &str,
    a_doc: &Document,
    a_outs: &[(String, Vec<Option<f64>>)],
) {
    println!("\n## 4. live via the server (B)");
    let msgs: Vec<String> = feed
        .iter()
        .enumerate()
        .map(|(i, t)| tick_cmd(i, t))
        .collect();
    let mut server = session(doc.clone(), 0);
    hello(&mut server, "f", "feed", "headless");
    hello(&mut server, "m", "arkin", "desktop");
    subscribe_changes(&mut server, "m");
    // Device mirror: the document from the snapshot.
    let ServerMsg::Result {
        data: QueryResult::Document { document },
        ..
    } = serde_json::from_str::<ServerMsg>(snapshot).unwrap()
    else {
        panic!()
    };
    let mut mirror = *document;
    let ids: Vec<String> = study_series(doc).into_iter().map(|s| s.0).collect();
    let queries: Vec<String> = ids
        .iter()
        .map(|id| {
            json!({"type":"query","id":format!("v-{id}"),"query":{"kind":"study","series":id,"range":{"last":1}}})
                .to_string()
        })
        .collect();
    let (mut down_change, mut down_values, mut up_q) = (0usize, 0usize, 0usize);
    let mut srv_feed = Vec::with_capacity(msgs.len());
    let mut srv_query = Vec::with_capacity(msgs.len());
    let mut dev_cost = Vec::with_capacity(msgs.len());
    let mut last_vals: BTreeMap<String, Option<f64>> = BTreeMap::new();
    // Server-side incremental engine for the hypothetical "values push" (B-push).
    let mut srv_live = Live::new(doc.clone(), server.clock(), true);
    let mut push_bytes = 0usize;
    for (i, m) in msgs.iter().enumerate() {
        let t0 = Instant::now();
        let lines = server.handle_raw("f", m);
        let notes: Vec<String> = lines
            .iter()
            .filter(|l| l.conn == "m")
            .map(|l| serde_json::to_string(&l.msg).unwrap())
            .collect();
        srv_feed.push(ms(t0) * 1e3);
        // Device applies the change events to its mirror.
        let t1 = Instant::now();
        for n in &notes {
            down_change += n.len();
            if let ServerMsg::Notify {
                event: Event::Change { changes, .. },
                ..
            } = serde_json::from_str::<ServerMsg>(n).unwrap()
            {
                for c in &changes {
                    apply_change(&mut mirror, c).unwrap();
                }
            }
        }
        let mut dev = ms(t1);
        // v0: the device asks for the last value of each study.
        let t2 = Instant::now();
        let mut results = Vec::with_capacity(queries.len());
        for q in &queries {
            up_q += q.len();
            let r = server.handle_raw("m", q);
            results.push(serde_json::to_string(&r[0].msg).unwrap());
        }
        srv_query.push(ms(t2) * 1e3);
        let t3 = Instant::now();
        for r in &results {
            down_values += r.len();
            if let ServerMsg::Result {
                data: QueryResult::Values { values },
                ..
            } = serde_json::from_str::<ServerMsg>(r).unwrap()
            {
                for (k, v) in values.outputs {
                    last_vals.insert(format!("{}.{k}", values.series), v[0]);
                }
            }
        }
        dev += ms(t3);
        dev_cost.push(dev * 1e3);
        // B-push: a compact "values" event per study (not in qp/0).
        srv_live.tick(&format!("t{i}"), &feed[i]);
        let seq = srv_live.log.len();
        let bar = px_bars(&srv_live.doc).len() - 1;
        for s in &srv_live.studies {
            let vals: BTreeMap<&str, Option<f64>> = s
                .names
                .iter()
                .zip(&s.outs)
                .map(|(n, c)| (n.as_str(), *c.last().unwrap()))
                .collect();
            push_bytes += json!({"type":"notify","sub":"vals","event":{"kind":"values","seq":seq,"series":s.id,"bar":bar,"values":vals}})
                .to_string()
                .len();
        }
    }
    // Fan-out cost per extra device: serializing one change notification.
    let sample = server.handle_raw(
        "f",
        &tick_cmd(
            msgs.len(),
            &Tick::Update(px_bars(server.document()).last().unwrap().clone()),
        ),
    );
    let note = sample.iter().find(|l| l.conn == "m").unwrap().msg.clone();
    let (fan, _, _) = bench(cfg.reps, || {
        let mut n = 0;
        for _ in 0..1000 {
            n += serde_json::to_string(&note).unwrap().len();
        }
        n
    });
    kv!(
        "liveB.fanout_serialize_notify_us",
        "{:.2} per device per tick",
        fan
    );
    let n = msgs.len() as f64;
    let per_s = |b: usize| b as f64 / cfg.secs as f64;
    kv!(
        "liveB.down_change_bytes",
        "{:.0}/tick, {:.0} B/s",
        down_change as f64 / n,
        per_s(down_change)
    );
    kv!(
        "liveB.v0_values_by_query",
        "down {:.0}/tick + up {:.0}/tick ; total B/s down {:.0} up {:.0}",
        down_values as f64 / n,
        up_q as f64 / n,
        per_s(down_change + down_values),
        per_s(up_q)
    );
    kv!(
        "liveB.push_values_hypothetical",
        "{:.0}/tick ; total down B/s {:.0}",
        push_bytes as f64 / n,
        per_s(down_change + push_bytes)
    );
    kv!(
        "liveB.mirror_with_local_compute",
        "down B/s {:.0} (change events only)",
        per_s(down_change)
    );
    let total_srv: f64 = srv_feed.iter().sum::<f64>() + srv_query.iter().sum::<f64>();
    kv!(
        "liveB.server_us_per_tick_per_device",
        "feed {:.1} + 3 study queries {:.1} = {:.1} ; {:.2}% of a core per device",
        mean(&srv_feed),
        mean(&srv_query),
        mean(&srv_feed) + mean(&srv_query),
        100.0 * total_srv / 1e3 / (cfg.secs as f64 * 1e3)
    );
    kv!(
        "liveB.device_us_per_tick",
        "mean {:.2} p99 {:.2} (parse + apply change + parse 3 results)",
        mean(&dev_cost),
        pct(&dev_cost, 0.99)
    );
    // Determinism: mirror == server == device authority in A.
    let ds = digest_json(server.document());
    let dm = digest_json(&mirror);
    kv!(
        "det.B.doc_digest",
        "server {ds} mirror {dm} A-device {} (all equal: {})",
        digest_json(a_doc),
        ds == dm && ds == digest_json(a_doc)
    );
    let mut same = true;
    for (k, v) in &last_vals {
        let a = a_outs
            .iter()
            .find(|(n, _)| n == k)
            .and_then(|(_, c)| *c.last().unwrap());
        same &= a.map(f64::to_bits) == v.map(f64::to_bits);
    }
    kv!(
        "det.B.last_values_server_vs_A_device",
        "{} values bit-equal: {same}",
        last_vals.len()
    );
}

/// The device's offline edits: 20 commands by the owner.
fn offline_edits() -> Vec<(Op, Vec<&'static str>, String)> {
    let mut v = Vec::new();
    let names = [
        "sma0", "sma1", "sma2", "sma3", "sma4", "sma5", "sma6", "sma7", "sma8", "sma9",
    ];
    for (k, id) in names.iter().enumerate() {
        let op: Op = serde_json::from_value(json!({"op":"add_study","id":id,"study":"sma",
            "inputs":["px"],"params":{"period": 10 + 5 * k},"show":{"view":"main","with":"px"}}))
        .unwrap();
        v.push((op, vec![*id, "main"], (*id).to_owned()));
    }
    for (k, id) in names.iter().take(5).enumerate() {
        let op: Op = serde_json::from_value(
            json!({"op":"set_params","series":id,"params":{"period": 100 + k}}),
        )
        .unwrap();
        v.push((op, vec![*id], (*id).to_owned()));
    }
    let op: Op =
        serde_json::from_value(json!({"op":"set_params","series":"macd","params":{"fast":10}}))
            .unwrap();
    v.push((op, vec!["macd"], "macd".to_owned()));
    for id in &names[5..9] {
        let op: Op = serde_json::from_value(json!({"op":"remove","id":id})).unwrap();
        v.push((op, vec![*id, "main"], (*id).to_owned()));
    }
    v
}

fn log_stats(label: &str, log: &[LogEntry]) {
    let j = serde_json::to_string(log).unwrap();
    kv!(format!("{label}.entries"), "{}", log.len());
    sizes(&format!("{label}.log_json"), j.as_bytes());
}

/// Drops `set_last_bar` feed entries that a later one supersedes before the next
/// append, and renumbers (loses the intermediate ticks from the audit trail).
fn compact(log: &[LogEntry]) -> Vec<LogEntry> {
    let is_set = |e: &LogEntry| matches!(e.changes.as_slice(), [ChangeOp::SetLastBar { .. }]);
    let is_bar = |e: &LogEntry| {
        matches!(
            e.changes.as_slice(),
            [ChangeOp::SetLastBar { .. }] | [ChangeOp::AppendBars { .. }]
        )
    };
    let mut out = Vec::new();
    for (i, e) in log.iter().enumerate() {
        if is_set(e) {
            if let Some(next_bar) = log[i + 1..].iter().find(|x| is_bar(x)) {
                if is_set(next_bar) {
                    continue;
                }
            }
        }
        let mut e = e.clone();
        e.seq = out.len() as Seq + 1;
        out.push(e);
    }
    out
}

/// 5. Offline and reconnect.
#[allow(clippy::too_many_lines)]
fn section_offline(cfg: &Cfg, doc: &Document, feed: &[Tick]) {
    println!("\n## 5. offline and reconnect");
    let edits = offline_edits();
    let every = (feed.len() / edits.len()).max(1);

    // --- A: the device holds the authority (epoch 1) and works offline.
    let run_device = |with_feed: bool| -> Session {
        let mut dev = session(doc.clone(), 1);
        hello(&mut dev, "a", "arkin", "phone");
        hello(&mut dev, "f", "feed", "headless");
        let mut e = 0;
        if with_feed {
            for (i, t) in feed.iter().enumerate() {
                if i % every == 0 && e < edits.len() {
                    let r =
                        dev.handle_raw("a", &cmd(&format!("e{e}"), vec![edits[e].0.clone()], None));
                    assert!(!is_error(&r, "a"), "{:?}", r[0].msg);
                    e += 1;
                }
                let r = dev.handle_raw("f", &tick_cmd(i, t));
                assert!(!is_error(&r, "f"));
            }
        }
        while e < edits.len() {
            let r = dev.handle_raw("a", &cmd(&format!("e{e}"), vec![edits[e].0.clone()], None));
            assert!(!is_error(&r, "a"), "{:?}", r[0].msg);
            e += 1;
        }
        dev
    };
    for (label, with_feed) in [
        ("handback.edits_only", false),
        ("handback.edits_and_local_feed", true),
    ] {
        let dev = run_device(with_feed);
        log_stats(label, dev.log());
        sizes(
            &format!("{label}.final_snapshot_json"),
            serde_json::to_string(dev.document()).unwrap().as_bytes(),
        );
        let (med, min, re) = bench(cfg.reps.min(7), || replay(doc, dev.log()).unwrap());
        kv!(
            format!("{label}.server_replay_ms"),
            "median {med:.3} min {min:.3} ; digest equal to device: {}",
            digest_json(&re) == digest_json(dev.document())
        );
        let jl = serde_json::to_string(dev.log()).unwrap();
        let (pmed, _, _) = bench(cfg.reps.min(7), || {
            serde_json::from_str::<Vec<LogEntry>>(&jl).unwrap()
        });
        kv!(format!("{label}.server_parse_log_ms"), "median {pmed:.3}");
        if with_feed {
            let c = compact(dev.log());
            log_stats(&format!("{label}.compacted"), &c);
            let re = replay(doc, &c).unwrap();
            kv!(
                format!("{label}.compacted.replay_digest_equal"),
                "{}",
                digest_json(&re) == digest_json(dev.document())
            );
        }
    }

    // --- A: hand back while the server buffered the feed (it may not write; §8.4).
    let dev_e = run_device(false);
    let dev_el = run_device(true);
    for (label, dev) in [
        ("handback.rebase_buffered_feed.device_had_no_feed", &dev_e),
        (
            "handback.rebase_buffered_feed.device_had_same_feed",
            &dev_el,
        ),
    ] {
        let t0 = Instant::now();
        let mut srv = session(dev.document().clone(), 2);
        hello(&mut srv, "f", "feed", "headless");
        let mut errs = 0usize;
        for (i, t) in feed.iter().enumerate() {
            if is_error(&srv.handle_raw("f", &tick_cmd(i, t)), "f") {
                errs += 1;
            }
        }
        kv!(
            format!("{label}"),
            "naive re-apply of {} buffered ticks: {errs} errors, {:.1} ms",
            feed.len(),
            ms(t0)
        );
        // Dedupe rule: skip ticks whose bar time is before the device's last bar, or an
        // append of a bar the device already has.
        let last_t = px_bars(dev.document()).last().unwrap().t;
        let mut srv = session(dev.document().clone(), 2);
        hello(&mut srv, "f", "feed", "headless");
        let (mut errs, mut skipped) = (0usize, 0usize);
        for (i, t) in feed.iter().enumerate() {
            let bt = t.bar().t;
            if bt < last_t || (bt == last_t && matches!(t, Tick::Append(_))) {
                skipped += 1;
                continue;
            }
            if is_error(&srv.handle_raw("f", &tick_cmd(i, t)), "f") {
                errs += 1;
            }
        }
        kv!(
            format!("{label}.dedupe_by_t"),
            "skipped {skipped}, errors {errs}"
        );
    }

    // --- Fork: the server kept accepting writes (violating §8.4) while the device
    // was the authority. Items touched by both lineages are silently overwritten by
    // whichever document wins.
    let mut srv = session(doc.clone(), 0);
    hello(&mut srv, "f", "feed", "headless");
    hello(&mut srv, "e", "editor", "headless");
    for (i, t) in feed.iter().enumerate() {
        if i == feed.len() / 2 {
            let r = srv.handle_raw(
                "e",
                &cmd(
                    "ed1",
                    vec![serde_json::from_value(
                        json!({"op":"set_params","series":"macd","params":{"fast":8}}),
                    )
                    .unwrap()],
                    None,
                ),
            );
            assert!(!is_error(&r, "e"));
        }
        srv.handle_raw("f", &tick_cmd(i, t));
    }
    let srv_ids: std::collections::BTreeSet<String> = srv
        .log()
        .iter()
        .flat_map(|e| touched_ids(&e.changes))
        .collect();
    let dev_ids: std::collections::BTreeSet<String> = dev_el
        .log()
        .iter()
        .flat_map(|e| touched_ids(&e.changes))
        .collect();
    let both: Vec<_> = srv_ids.intersection(&dev_ids).cloned().collect();
    kv!(
        "fork.server_epoch0_vs_device_epoch1",
        "doc digests {} vs {} ; ids touched by both lineages: {:?}",
        digest_json(srv.document()),
        digest_json(dev_el.document()),
        both
    );

    // --- B: the device stays a mirror; offline it queues commands with `base` (§8.2)
    // and sends them on reconnect. The server meanwhile took the feed and an editor's
    // change to macd.
    for smart in [false, true] {
        let mut srv = session(doc.clone(), 0);
        hello(&mut srv, "a", "arkin", "phone");
        hello(&mut srv, "f", "feed", "headless");
        hello(&mut srv, "e", "editor", "headless");
        let base0 = 0;
        for (i, t) in feed.iter().enumerate() {
            if i == feed.len() / 2 {
                let r = srv.handle_raw(
                    "e",
                    &cmd(
                        "ed1",
                        vec![serde_json::from_value(
                            json!({"op":"set_params","series":"macd","params":{"fast":8}}),
                        )
                        .unwrap()],
                        None,
                    ),
                );
                assert!(!is_error(&r, "e"));
            }
            srv.handle_raw("f", &tick_cmd(i, t));
        }
        let mut own: BTreeMap<&str, Seq> = BTreeMap::new();
        let (mut applied, mut conflicts, mut other) = (0, 0, 0);
        let mut queued_bytes = 0usize;
        let t0 = Instant::now();
        for (k, (op, touched, _)) in edits.iter().enumerate() {
            let base = if smart {
                touched
                    .iter()
                    .filter_map(|i| own.get(i).copied())
                    .max()
                    .unwrap_or(base0)
                    .max(base0)
            } else {
                base0
            };
            let m = cmd(&format!("q{k}"), vec![op.clone()], Some(base));
            queued_bytes += m.len();
            let r = srv.handle_raw("a", &m);
            if let Some(s) = ack_seq(&r, "a") {
                applied += 1;
                for i in touched {
                    own.insert(i, s);
                }
            } else if error_code(&r, "a").as_deref() == Some("\"conflict\"") {
                conflicts += 1;
            } else {
                other += 1;
            }
        }
        kv!(
            format!(
                "offlineB.queue_with_base.{}",
                if smart { "per_item_base" } else { "single_base" }
            ),
            "{} queued ({} bytes): applied {applied}, conflict {conflicts}, other errors {other}, {:.2} ms",
            edits.len(),
            queued_bytes,
            ms(t0)
        );
    }
}
