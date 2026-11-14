//! Shared by the main bench and the `plainjson` bench (included with `#[path]`).
//! Needs `crate::schema` to re-export the qp-schema types (`Bar`, `Document`, protocol types).
//!
//! Everything here is deterministic: fixed seeds, no clocks except in `median_ns`.

use std::collections::BTreeMap;
use std::time::Instant;

use crate::schema::{
    Bar, ChangeOp, Document, Event, QueryResult, SeriesData, SeriesValues, ServerMsg,
};

pub const FIXTURE: &str = include_str!("../../../fixtures/protocol/docs/thyao.json");
pub const N_BARS: usize = 10_000;
pub const TICKS_PER_SEC: usize = 60;

/// xorshift64* — deterministic, seedable.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

pub fn fixture_doc() -> Document {
    serde_json::from_str(FIXTURE).expect("fixture parses")
}

pub fn fixture_bars(doc: &Document) -> Vec<Bar> {
    doc.series
        .iter()
        .find_map(|s| s.bars())
        .expect("fixture has bars")
        .to_vec()
}

/// Extends `base` to `n` bars with a seeded random walk (prices and volumes rounded to 2
/// decimals like the fixture; daily step).
pub fn extend_bars(base: &[Bar], n: usize, seed: u64) -> Vec<Bar> {
    let mut rng = Rng::new(seed);
    let mut out = base.to_vec();
    out.truncate(n);
    while out.len() < n {
        let prev = out.last().unwrap().clone();
        out.push(next_bar(&prev, &mut rng, 86_400_000));
    }
    out
}

pub fn next_bar(prev: &Bar, rng: &mut Rng, step_ms: i64) -> Bar {
    let o = r2(prev.c * (1.0 + 0.004 * (rng.unit() - 0.5)));
    let c = r2(o * (1.0 + 0.03 * (rng.unit() - 0.5)));
    let h = r2(o.max(c) * (1.0 + 0.01 * rng.unit()));
    let l = r2(o.min(c) * (1.0 - 0.01 * rng.unit()));
    let v = r2(500_000.0 + 2_000_000.0 * rng.unit());
    Bar {
        t: prev.t + step_ms,
        o,
        h,
        l,
        c,
        v,
    }
}

/// Wilder RSI; `None` during warmup (first `period` bars).
pub fn rsi(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; closes.len()];
    if closes.len() <= period {
        return out;
    }
    let (mut g, mut l) = (0.0, 0.0);
    for i in 1..=period {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 {
            g += d
        } else {
            l -= d
        }
    }
    g /= period as f64;
    l /= period as f64;
    let val = |g: f64, l: f64| {
        if l == 0.0 {
            100.0
        } else {
            100.0 - 100.0 / (1.0 + g / l)
        }
    };
    out[period] = Some(val(g, l));
    for i in period + 1..closes.len() {
        let d = closes[i] - closes[i - 1];
        let (gi, li) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        g = (g * (period as f64 - 1.0) + gi) / period as f64;
        l = (l * (period as f64 - 1.0) + li) / period as f64;
        out[i] = Some(val(g, l));
    }
    out
}

fn ema(xs: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let k = 2.0 / (period as f64 + 1.0);
    let mut out = vec![None; xs.len()];
    let first = match xs.iter().position(|x| x.is_some()) {
        Some(p) => p,
        None => return out,
    };
    if xs.len() < first + period {
        return out;
    }
    let seed: f64 = xs[first..first + period]
        .iter()
        .map(|x| x.unwrap())
        .sum::<f64>()
        / period as f64;
    out[first + period - 1] = Some(seed);
    let mut e = seed;
    for i in first + period..xs.len() {
        e = xs[i].unwrap() * k + e * (1.0 - k);
        out[i] = Some(e);
    }
    out
}

/// MACD(12, 26, 9): (macd, signal, hist).
pub fn macd(closes: &[f64]) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let c: Vec<Option<f64>> = closes.iter().map(|&x| Some(x)).collect();
    let (f, s) = (ema(&c, 12), ema(&c, 26));
    let m: Vec<Option<f64>> = f.iter().zip(&s).map(|(a, b)| Some((*a)? - (*b)?)).collect();
    let sig = ema(&m, 9);
    let hist = m
        .iter()
        .zip(&sig)
        .map(|(a, b)| Some((*a)? - (*b)?))
        .collect();
    (m, sig, hist)
}

pub fn notify(changes: Vec<ChangeOp>, seq: u64, at: i64) -> ServerMsg {
    ServerMsg::Notify {
        sub: "sub-px".into(),
        event: Event::Change {
            seq,
            at,
            author: "feed".into(),
            approved_by: None,
            cause: None,
            changes,
        },
    }
}

pub fn tick_set_last(bar: Bar, seq: u64) -> ServerMsg {
    let at = bar.t;
    notify(
        vec![ChangeOp::SetLastBar {
            series: "px".into(),
            bar,
        }],
        seq,
        at,
    )
}

pub fn append_bars(bars: Vec<Bar>, seq: u64) -> ServerMsg {
    let at = bars.last().map(|b| b.t).unwrap_or(0);
    notify(
        vec![ChangeOp::AppendBars {
            series: "px".into(),
            bars,
        }],
        seq,
        at,
    )
}

pub fn values_msg(
    series: &str,
    bars: &[Bar],
    outputs: BTreeMap<String, Vec<Option<f64>>>,
) -> ServerMsg {
    ServerMsg::Result {
        id: "q-17".into(),
        seq: 4242,
        data: QueryResult::Values {
            values: SeriesValues {
                series: series.into(),
                from_bar: 0,
                t: bars.iter().map(|b| b.t).collect(),
                outputs,
            },
        },
    }
}

/// All messages and the document under test.
pub struct Corpus {
    pub ticks_set_last: Vec<ServerMsg>,
    pub ticks_append1: Vec<ServerMsg>,
    pub values_rsi: ServerMsg,
    pub values_macd: ServerMsg,
    pub append_10k: ServerMsg,
    pub document: Document,
}

pub fn corpus() -> Corpus {
    let doc0 = fixture_doc();
    let bars = extend_bars(&fixture_bars(&doc0), N_BARS, 0x5EED_0001);
    let closes: Vec<f64> = bars.iter().map(|b| b.c).collect();

    // 60 live ticks on the last bar (one second of a 60 Hz feed), and 60 one-bar appends.
    let mut rng = Rng::new(0x5EED_0002);
    let last = bars.last().unwrap().clone();
    let mut ticks_set_last = Vec::new();
    let mut cur = last.clone();
    for i in 0..TICKS_PER_SEC {
        let c = r2(cur.c * (1.0 + 0.001 * (rng.unit() - 0.5)));
        cur = Bar {
            h: cur.h.max(c),
            l: cur.l.min(c),
            c,
            v: r2(cur.v + 1000.0 * rng.unit()),
            ..cur
        };
        ticks_set_last.push(tick_set_last(cur.clone(), 100_000 + i as u64));
    }
    let mut ticks_append1 = Vec::new();
    let mut prev = last;
    for i in 0..TICKS_PER_SEC {
        let b = next_bar(&prev, &mut rng, 1_000);
        ticks_append1.push(append_bars(vec![b.clone()], 200_000 + i as u64));
        prev = b;
    }

    let mut o = BTreeMap::new();
    o.insert("rsi".to_string(), rsi(&closes, 14));
    let values_rsi = values_msg("rsi14", &bars, o);
    let (m, s, h) = macd(&closes);
    let mut o = BTreeMap::new();
    o.insert("macd".to_string(), m);
    o.insert("signal".to_string(), s);
    o.insert("hist".to_string(), h);
    let values_macd = values_msg("macd", &bars, o);
    let append_10k = append_bars(bars.clone(), 300_000);

    let mut document = doc0;
    for s in &mut document.series {
        if let Some(SeriesData::Bars { bars: b }) = &mut s.data {
            *b = bars.clone();
        }
    }
    Corpus {
        ticks_set_last,
        ticks_append1,
        values_rsi,
        values_macd,
        append_10k,
        document,
    }
}

// ---------------------------------------------------------------------------
// f64 exactness
// ---------------------------------------------------------------------------

#[allow(clippy::excessive_precision)]
pub fn tricky_values() -> Vec<f64> {
    vec![
        0.1,
        -0.0,
        0.0,
        5e-324,
        -5e-324,
        1.5e-323,
        2.225073858507201e-308,
        2.2250738585072014e-308,
        f64::MIN_POSITIVE,
        1.7976931348623157e308,
        -1.7976931348623157e308,
        1.7976931348623155e308,
        8.98846567431158e307,
        1e300,
        1e-300,
        0.30000000000000004,
        0.1 + 0.2,
        1.0 / 3.0,
        std::f64::consts::PI,
        std::f64::consts::E,
        123456789.12345679,
        9007199254740993.0,
        9007199254740991.0,
        4.35,
        100.29,
        0.000123456789,
        1e21,
        1e22,
        1e23,
        1.5e-7,
        2.5,
        f64::EPSILON,
        3.4028234663852886e38,
        72.83959265032422,
        69.35847609603053,
        -1e-310,
        2.4703282292062327e-324,
        4.9406564584124654e-324,
        1.7e308,
        5e-310,
        0.5,
        1e15 + 0.3,
        2.0f64.powi(-1074),
        2.0f64.powi(1023),
        1.0 - f64::EPSILON / 2.0,
    ]
}

/// Tricky values + `n_random` finite random bit patterns + price-like full-precision values.
pub fn exactness_values(n_random: usize) -> Vec<f64> {
    let mut v = tricky_values();
    let mut rng = Rng::new(0x5EED_00F1);
    while v.len() < n_random {
        let x = f64::from_bits(rng.next_u64());
        if x.is_finite() {
            v.push(x);
        }
        v.push(100.0 * (1.0 + rng.unit()) * (1.0 + 1e-9 * rng.unit()));
    }
    v
}

/// Packs values into one append_bars message (5 per bar) and one values message.
pub fn exactness_msgs(vals: &[f64]) -> (ServerMsg, ServerMsg) {
    let mut bars = Vec::new();
    for (i, ch) in vals.chunks(5).enumerate() {
        let g = |k: usize| ch.get(k).copied().unwrap_or(0.0);
        bars.push(Bar {
            t: 1_700_000_000_000 + i as i64,
            o: g(0),
            h: g(1),
            l: g(2),
            c: g(3),
            v: g(4),
        });
    }
    let mut o = BTreeMap::new();
    o.insert("x".to_string(), vals.iter().map(|&x| Some(x)).collect());
    let t: Vec<Bar> = (0..vals.len())
        .map(|i| Bar {
            t: i as i64,
            o: 0.0,
            h: 0.0,
            l: 0.0,
            c: 0.0,
            v: 0.0,
        })
        .collect();
    (append_bars(bars, 1), values_msg("x", &t, o))
}

const NONE_BITS: u64 = 0x7FF8_DEAD_BEEF_0000;

fn bar_bits(b: &Bar, out: &mut Vec<u64>) {
    out.extend([
        b.t as u64,
        b.o.to_bits(),
        b.h.to_bits(),
        b.l.to_bits(),
        b.c.to_bits(),
        b.v.to_bits(),
    ]);
}

/// Every number carried by a hot-path message, as raw bits (exact comparison incl. -0.0).
pub fn msg_bits(m: &ServerMsg) -> Vec<u64> {
    let mut out = Vec::new();
    match m {
        ServerMsg::Notify {
            event: Event::Change {
                seq, at, changes, ..
            },
            ..
        } => {
            out.extend([*seq, *at as u64]);
            for c in changes {
                match c {
                    ChangeOp::AppendBars { bars, .. } => {
                        bars.iter().for_each(|b| bar_bits(b, &mut out))
                    }
                    ChangeOp::SetLastBar { bar, .. } => bar_bits(bar, &mut out),
                    _ => {}
                }
            }
        }
        ServerMsg::Result {
            seq,
            data: QueryResult::Values { values },
            ..
        } => {
            out.extend([*seq, values.from_bar]);
            out.extend(values.t.iter().map(|&t| t as u64));
            for v in values.outputs.values() {
                out.extend(v.iter().map(|x| x.map(f64::to_bits).unwrap_or(NONE_BITS)));
            }
        }
        _ => {}
    }
    out
}

pub fn doc_bits(d: &Document) -> Vec<u64> {
    let mut out = Vec::new();
    for s in &d.series {
        if let Some(b) = s.bars() {
            b.iter().for_each(|b| bar_bits(b, &mut out));
        }
    }
    out
}

/// Number of positions whose bits differ (or `usize::MAX` on length mismatch).
pub fn bit_mismatches(a: &[u64], b: &[u64]) -> usize {
    if a.len() != b.len() {
        return usize::MAX;
    }
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

/// Median nanoseconds per call of `f`. Each sample runs enough iterations to last
/// at least ~0.5 ms; 31 samples (11 when a single call takes over 20 ms).
pub fn median_ns<F: FnMut()>(mut f: F) -> f64 {
    for _ in 0..3 {
        f();
    }
    let t0 = Instant::now();
    f();
    let single = (t0.elapsed().as_nanos() as f64).max(1.0);
    let iters = ((500_000.0 / single).ceil() as usize).max(1);
    let samples = if single > 20e6 { 11 } else { 31 };
    let mut v: Vec<f64> = (0..samples)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_nanos() as f64 / iters as f64
        })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

pub fn platform() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "wasm32-wasip1 (node)"
    } else {
        "native"
    }
}
