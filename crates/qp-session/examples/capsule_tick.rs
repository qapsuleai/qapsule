//! R3/S-D — session-level live tick cost with the capsule engine (GOAL criterion 17).
//!
//! ```text
//! cargo run --release -p qp-session --example capsule_tick \
//!     [-- <instruments> <history> <market ticks> <runs> <modes> <path>]
//! cargo run --release -p qp-session --example capsule_tick -- profile <mode> <market ticks> [history] [path]
//! ```
//!
//! Defaults `30 1000 600 5 none,caps,flow bound`.
//!
//! A document with `instruments` bar series (`synthetic_market(instruments, …, 1)`, daily),
//! each bound (`bind`, feed `bist`, `window.last` 10000; the history given as
//! `init.data`) or inline (`path` = `inline`: bars in the document, ticks as commands from
//! a feed connection). Modes:
//!
//! - `none`: no capsule engine (the RP2 live path over 30 series; the comparison point);
//! - `caps`: `init.capsules.series` = every series (the seven default detectors, six
//!   meaning kinds, and the engine `EngineConfig::r3()`), no licensed flow;
//! - `flow`: as `caps` plus a synthetic licensed flow per series (`init.capsules.flow`,
//!   feed `flow`): the seventh detector (`flow.institutional`) has data and the trust path
//!   (mühür) runs.
//!
//! Connections: `a` (arkin, owner, entitled to everything), `b` (bora, bars only), `c` (can,
//! no entitlement); each subscribes to `changes` and `capsules {all}`.
//!
//! Live feed (RP2 pattern per instrument): per simulated second one new bar then 59
//! revisions of it (field-level `patch` with only the fields that changed); revision `k`
//! of 60 moves the close from the open towards the final close (the 60th equals the
//! final bar, as in `qp-capsule/examples/engine_cost.rs`). A **market tick** = one data
//! event on each instrument (30 events). A new bar makes the previous one final: the
//! authority then runs the detectors and the engine on it (`run_capsules`), writes one
//! engine entry per series and notifies the subscribers. On revisions the authority does
//! no capsule work (candidates are the replica's, §20; not measured here).
//!
//! Timing: `Session::new` (includes the engine catch-up over the history) and the
//! connections' `hello`/`subscribe` are outside the loop and reported separately. Fresh
//! session per run; a timer per market tick (30 events), split into bar-close ticks and
//! revision ticks; per run the mean; reported: median and spread (min–max) over `runs`.

use std::path::Path;
use std::time::Instant;

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

/// Events per bar (RP2: 60 = one new bar + 59 revisions); env `PER_BAR` overrides it (a
/// large value profiles revisions only).
fn per_bar() -> usize {
    std::env::var("PER_BAR")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(60)
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

fn pct(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let i = ((s.len() as f64 - 1.0) * p).round() as usize;
    s[i]
}

#[allow(clippy::cast_precision_loss)]
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

/// Revision `k` of `n` (1-based) of bar `b`; revision `n` is `b` itself.
fn revision(b: &Bar, k: usize, n: usize) -> Bar {
    if k == n {
        return b.clone();
    }
    #[allow(clippy::cast_precision_loss)]
    let f = k as f64 / n as f64;
    let wiggle = if k.is_multiple_of(2) { 0.25 } else { -0.25 } * (b.h - b.l) * (1.0 - f);
    let c = (b.o + (b.c - b.o) * f + wiggle).clamp(b.l, b.h);
    Bar {
        t: b.t,
        o: b.o,
        h: b.o.max(c) + (b.h - b.o.max(b.c)).max(0.0) * f,
        l: b.o.min(c) - (b.o.min(b.c) - b.l).max(0.0) * f,
        c,
        v: b.v * f,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    None,
    Caps,
    Flow,
}

impl Mode {
    fn parse(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "caps" => Self::Caps,
            _ => Self::Flow,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Caps => "caps",
            Self::Flow => "flow",
        }
    }
}

struct Market {
    symbols: Vec<String>,
    /// History bars per instrument (the last one is the open bar at start).
    history: Vec<Vec<Bar>>,
    /// Per market tick, the message per instrument (data event JSON or command JSON).
    ticks: Vec<Vec<String>>,
    /// Market ticks that append a new bar (a bar close).
    closes: Vec<bool>,
}

fn market(n: usize, history: usize, market_ticks: usize, bound: bool) -> Market {
    let per = per_bar();
    let new_bars = market_ticks.div_ceil(per);
    let m = qp_source::synthetic_market(n, history + new_bars, 1);
    let symbols: Vec<String> = m.iter().map(|i| i.symbol.clone()).collect();
    let hist: Vec<Vec<Bar>> = m.iter().map(|i| i.bars[..history].to_vec()).collect();
    let mut ticks = Vec::with_capacity(market_ticks);
    let mut closes = Vec::with_capacity(market_ticks);
    let mut prev: Vec<Bar> = hist.iter().map(|h| h.last().unwrap().clone()).collect();
    for k in 0..market_ticks {
        let bar_i = history + k / per;
        let rev = k % per + 1;
        closes.push(rev == 1);
        let mut row = Vec::with_capacity(n);
        for (i, inst) in m.iter().enumerate() {
            let b = revision(&inst.bars[bar_i], rev, per);
            let sym = &symbols[i];
            let msg = if bound {
                let change = if rev == 1 {
                    json!({"kind":"bar","bar":b})
                } else {
                    let p = &prev[i];
                    let mut f = serde_json::Map::new();
                    for (key, a, z) in [
                        ("o", p.o, b.o),
                        ("h", p.h, b.h),
                        ("l", p.l, b.l),
                        ("c", p.c, b.c),
                        ("v", p.v, b.v),
                    ] {
                        #[allow(clippy::float_cmp)]
                        if a != z {
                            f.insert(key.to_owned(), json!(z));
                        }
                    }
                    json!({"kind":"patch","t":b.t,"fields":f})
                };
                json!({"feed":{"symbol":sym,"interval":"1d","feed":"bist"},"seq":k + 1,"change":change})
                    .to_string()
            } else {
                let op = if rev == 1 {
                    json!({"op":"append_bars","series":format!("px{i:02}"),"bars":[b]})
                } else {
                    json!({"op":"update_last_bar","series":format!("px{i:02}"),"bar":b})
                };
                json!({"type":"command","id":format!("t{k}.{i}"),"ops":[op]}).to_string()
            };
            prev[i] = b;
            row.push(msg);
        }
        ticks.push(row);
    }
    Market {
        symbols,
        history: hist,
        ticks,
        closes,
    }
}

fn init(mk: &Market, mode: Mode, bound: bool) -> SessionInit {
    let mut series = Vec::new();
    for (i, sym) in mk.symbols.iter().enumerate() {
        let id = format!("px{i:02}");
        if bound {
            series.push(json!({"id":id,"instrument":{"symbol":sym},"interval":"1d",
                "source":{"kind":"bind","symbol":sym,"interval":"1d","window":{"last":10000},"feed":"bist"}}));
        } else {
            series.push(json!({"id":id,"instrument":{"symbol":sym},"interval":"1d",
                "source":{"kind":"inline"},"data":{"kind":"bars","bars":mk.history[i]}}));
        }
    }
    let mut v = json!({
        "document": {"schema_version":"0","id":"doc-s-d","title":"S-D","series":series},
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"arkin","author":{"kind":"human","name":"Arkın"},"role":"owner"},
            {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
            {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"},
            {"id":"feed","author":{"kind":"human","name":"feed"},"role":"editor"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0
    });
    if bound {
        let feeds: Vec<Value> = mk
            .symbols
            .iter()
            .zip(&mk.history)
            .map(|(s, h)| json!({"feed":{"symbol":s,"interval":"1d","feed":"bist"},"bars":h}))
            .collect();
        let bora: Vec<String> = mk.symbols.iter().map(|s| format!("bist/{s}")).collect();
        v["data"] = json!({"feeds":feeds,"entitlements":{
            "host":["*"],"arkin":["*"],"feed":["*"],"bora":bora,"can":[]}});
    }
    if mode != Mode::None {
        let ids: Vec<String> = (0..mk.symbols.len()).map(|i| format!("px{i:02}")).collect();
        let flow: Vec<Value> = if mode == Mode::Flow {
            ids.iter()
                .enumerate()
                .map(|(i, id)| json!({"series":id,"seed":i + 1}))
                .collect()
        } else {
            Vec::new()
        };
        v["capsules"] = json!({"series":ids,"flow":flow});
    }
    serde_json::from_value(v).expect("init")
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> usize {
    let t = Instant::now();
    let lines = s.handle_raw(conn, &msg.to_string());
    if std::env::var_os("QP_TIME_SEND").is_some() {
        eprintln!("send {conn} {}: {:.3} ms", msg["type"], us(t) / 1e3);
    }
    digest_lines(&lines);
    lines.len()
}

/// A fresh session with its connections: (session, `Session::new` µs, µs of the four
/// `hello`s + six `subscribe`s).
fn session(mk: &Market, mode: Mode, bound: bool) -> (Session, f64, f64) {
    let ini = init(mk, mode, bound);
    let t = Instant::now();
    let mut s = Session::new(ini, Path::new(".")).expect("session");
    let new_us = us(t);
    let t = Instant::now();
    for (conn, who) in [("a", "arkin"), ("b", "bora"), ("c", "can"), ("f", "feed")] {
        send(
            &mut s,
            conn,
            &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"headless"}}),
        );
        if conn != "f" {
            send(
                &mut s,
                conn,
                &json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}),
            );
            send(
                &mut s,
                conn,
                &json!({"type":"subscribe","id":"cp","topic":{"kind":"capsules","all":true}}),
            );
        }
    }
    (s, new_us, us(t))
}

/// FNV-1a of every outbound line (`conn|msg`) of a run, in order (output equality check
/// across code changes; env `QP_LINES_DIGEST=1`, off by default: it costs time).
static LINES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0xcbf2_9ce4_8422_2325);

fn digest_lines(lines: &[qp_schema::protocol::OutboundLine]) {
    if std::env::var_os("QP_LINES_DIGEST").is_none() {
        return;
    }
    let mut h = LINES.load(std::sync::atomic::Ordering::Relaxed);
    for l in lines {
        let s = format!("{}|{}\n", l.conn, serde_json::to_string(&l.msg).unwrap());
        for b in s.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    LINES.store(h, std::sync::atomic::Ordering::Relaxed);
}

/// One market tick; returns the outbound lines per connection (a, b, c, other).
fn tick(s: &mut Session, row: &[String], bound: bool, counts: &mut [usize; 4]) {
    for m in row {
        let lines = if bound {
            let ev: DataEvent = serde_json::from_str(m).expect("event");
            s.data_event(&ev).expect("data event applies")
        } else {
            s.handle_raw("f", m)
        };
        digest_lines(&lines);
        for l in &lines {
            let k = match l.conn.as_str() {
                "a" => 0,
                "b" => 1,
                "c" => 2,
                _ => 3,
            };
            counts[k] += 1;
        }
    }
}

struct Cfg {
    n: usize,
    history: usize,
    market_ticks: usize,
    runs: usize,
    bound: bool,
}

fn measure(cfg: &Cfg, mk: &Market, mode: Mode) {
    let k = format!(
        "{}.{}",
        if cfg.bound { "bound" } else { "inline" },
        mode.name()
    );
    #[allow(clippy::cast_precision_loss)]
    let ticks = cfg.market_ticks as f64;
    #[allow(clippy::cast_precision_loss)]
    let nf = cfg.n as f64;
    // Fresh session per run (built outside the timer); a timer per market tick.
    let mut loops = Vec::new();
    let mut revs = Vec::new();
    let mut closes = Vec::new();
    let mut rev_p99 = Vec::new();
    let mut news = Vec::new();
    let mut hellos = Vec::new();
    let mut digest = Vec::new();
    let mut counts = [0usize; 4];
    let mut end = String::new();
    let mut line_digests = Vec::new();
    for _ in 0..cfg.runs {
        LINES.store(0xcbf2_9ce4_8422_2325, std::sync::atomic::Ordering::Relaxed);
        let (mut s, new_us, hello_us) = session(mk, mode, cfg.bound);
        news.push(new_us);
        hellos.push(hello_us);
        counts = [0; 4];
        let mut rev = Vec::new();
        let mut close = Vec::new();
        for (row, is_close) in mk.ticks.iter().zip(&mk.closes) {
            let t = Instant::now();
            tick(&mut s, row, cfg.bound, &mut counts);
            let x = us(t);
            if *is_close {
                close.push(x);
            } else {
                rev.push(x);
            }
        }
        loops.push((rev.iter().sum::<f64>() + close.iter().sum::<f64>()) / ticks);
        revs.push(mean(&rev));
        rev_p99.push(pct(&rev, 0.99));
        if !close.is_empty() {
            closes.push(mean(&close));
        }
        digest.push(fnv(&serde_json::to_vec(s.document()).unwrap()));
        line_digests.push(format!(
            "{:016x} log {:016x}",
            LINES.load(std::sync::atomic::Ordering::Relaxed),
            fnv(&serde_json::to_vec(s.log()).unwrap())
        ));
        let d = s.document();
        end = format!(
            "log {} facts {} findings {} capsules {} live {} ledger {}",
            s.log().len(),
            d.facts.len(),
            d.findings.len(),
            d.capsules.len(),
            d.capsules
                .iter()
                .filter(|c| c.state == qp_schema::CapsuleState::Live)
                .count(),
            d.ledger.entries.len()
        );
    }
    let line = |name: &str, v: &[f64], scale: f64, unit: &str, extra: String| {
        println!(
            "{k}.{name} = median {:.3} min {:.3} max {:.3}{unit} (runs {}){extra}",
            pct(v, 0.5) / scale,
            pct(v, 0.0) / scale,
            pct(v, 1.0) / scale,
            v.len()
        );
    };
    line(
        "session_new_ms",
        &news,
        1e3,
        "",
        format!(
            " engine catch-up over {} bars x {} series included",
            cfg.history, cfg.n
        ),
    );
    line(
        "connect_ms",
        &hellos,
        1e3,
        "",
        " 4 hello + 6 subscribe after the catch-up".to_owned(),
    );
    line(
        "us_per_market_tick",
        &loops,
        1.0,
        "",
        format!(" all ticks; per event median {:.3}", pct(&loops, 0.5) / nf),
    );
    line(
        "revision_tick_us",
        &revs,
        1.0,
        "",
        format!(
            " mean per run; per event median {:.3}; p99 median {:.2}",
            pct(&revs, 0.5) / nf,
            pct(&rev_p99, 0.5)
        ),
    );
    if !closes.is_empty() {
        line(
            "close_tick_us",
            &closes,
            1.0,
            "",
            format!(
                " mean per run over {} closes; per instrument median {:.2}",
                mk.closes.iter().filter(|c| **c).count(),
                pct(&closes, 0.5) / nf
            ),
        );
    }
    let same = digest.windows(2).all(|w| w[0] == w[1]);
    println!(
        "{k}.check = doc {:016x} same_across_runs {same} lines a {} b {} c {} other {}",
        digest[0], counts[0], counts[1], counts[2], counts[3]
    );
    println!("{k}.end = {end}");
    if std::env::var_os("QP_LINES_DIGEST").is_some() {
        println!("{k}.lines_digest = {}", line_digests.join(", "));
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).map(String::as_str) == Some("hello") {
        // `hello <mode> <history> <principal> <n>`: n hellos (new connections), for a
        // profile of the connection cost.
        let mode = Mode::parse(&a[2]);
        let history: usize = a[3].parse().unwrap();
        let mk = market(30, history, 1, true);
        let (mut s, _, _) = session(&mk, mode, true);
        let n: usize = a[5].parse().unwrap();
        let t = Instant::now();
        for k in 0..n {
            send(
                &mut s,
                &format!("x{k}"),
                &json!({"type":"hello","protocol":"qp/0","principal":a[4],"surface":{"kind":"headless"}}),
            );
        }
        #[allow(clippy::cast_precision_loss)]
        let per = us(t) / 1e3 / n as f64;
        eprintln!("hello {}: {per:.3} ms each", a[4]);
        return;
    }
    if a.get(1).map(String::as_str) == Some("profile") {
        let mode = Mode::parse(&a[2]);
        let ticks: usize = a[3].parse().unwrap();
        let history: usize = a.get(4).and_then(|x| x.parse().ok()).unwrap_or(1000);
        let bound = a.get(5).is_none_or(|p| p != "inline");
        let mk = market(30, history, ticks, bound);
        let (mut s, _, _) = session(&mk, mode, bound);
        let mut c = [0; 4];
        // The first market tick (a bar close) before the profiled loop.
        tick(&mut s, &mk.ticks[0], bound, &mut c);
        eprintln!("loop start");
        let t = Instant::now();
        for row in &mk.ticks[1..] {
            tick(&mut s, row, bound, &mut c);
        }
        #[allow(clippy::cast_precision_loss)]
        let per = us(t) / ticks as f64;
        eprintln!(
            "profile {} ticks {ticks}: {per:.2} us/market tick",
            mode.name()
        );
        return;
    }
    let cfg = Cfg {
        n: a.get(1).and_then(|x| x.parse().ok()).unwrap_or(30),
        history: a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1000),
        market_ticks: a.get(3).and_then(|x| x.parse().ok()).unwrap_or(600),
        runs: a.get(4).and_then(|x| x.parse().ok()).unwrap_or(5),
        bound: a.get(6).is_none_or(|p| p != "inline"),
    };
    let modes: Vec<Mode> = a
        .get(5)
        .map_or("none,caps,flow", String::as_str)
        .split(',')
        .map(Mode::parse)
        .collect();
    println!("# R3/S-D capsule tick bench");
    println!(
        "config = instruments {} history {} market_ticks {} runs {} path {} per_bar {}",
        cfg.n,
        cfg.history,
        cfg.market_ticks,
        cfg.runs,
        if cfg.bound { "bound" } else { "inline" },
        per_bar()
    );
    let mk = market(cfg.n, cfg.history, cfg.market_ticks, cfg.bound);
    // Warm-up pass (not reported): allocator, caches, CPU frequency.
    {
        let (mut s, _, _) = session(&mk, Mode::None, cfg.bound);
        let mut c = [0; 4];
        for row in mk.ticks.iter().take(120) {
            tick(&mut s, row, cfg.bound, &mut c);
        }
    }
    for m in modes {
        measure(&cfg, &mk, m);
    }
}
