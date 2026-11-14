//! Generator of the R2 view fixtures (`fixtures/views/`, `rounds/R2/tasks/VIEWS.md`).
//!
//! Shared by `examples/make_views.rs` (writes the files) and `tests/views.rs` (checks
//! that the committed files are exactly what this generator produces). Every view is a
//! conformance transcript (`docs/PROTOCOL.md` §15): an init document plus the protocol
//! messages of VIEWS.md; no view is written by hand.
//!
//! Synthetic data, all with IEEE basic operations only (`+ - * /`, `sqrt`, `round`), so
//! the numbers are the same on every target:
//!
//! - `round2(x) = round(x * 100) / 100`.
//! - **Fan (V3)**: last bar of `fixtures/bars/sample.json` (bar 199, close
//!   `c = 124.69`). Point `k = 0..=20` at the bar's time (`k = 0`) and the next 20
//!   weekdays (00:00 UTC, Saturday/Sunday skipped); for quantile `z`
//!   `v = round2(c * (1 + 0.0015 k + 0.012 z sqrt(k)))`, with `z = ±1.6449` (q05/q95),
//!   `±0.8416` (q20/q80), `±0.3853` (q35/q65).
//! - **Growth (V4)**: the 200 bar times of `sample.json`; `trend_0 = 10`,
//!   `trend_i = trend_(i-1) * 1.0187` (≈ 10 → ≈ 400); `w1..w4` in `[-1, 1)` from the
//!   64-bit LCG `s = s * 6364136223846793005 + 1442695040888963407` (wrapping, seed 42),
//!   `w = (s >> 11) / 2^53 * 2 - 1`; `c = round2(trend (1 + 0.015 w1))`,
//!   `o = previous c` (`round2(9.96)` for bar 0), `h = round2(max(o, c) (1 + 0.008 |w2|))`,
//!   `l = round2(min(o, c) (1 - 0.008 |w3|))`, `v = round2(1e6 (1 + 0.5 w4))`.

#![allow(dead_code)]

use std::path::Path;

use serde_json::{json, Value};

/// Views rendered from the base document at 1200x600 (id, transcript stem, view id).
pub const VIEWS: &[(&str, &str)] = &[
    ("v0-r0", "r0"),
    ("v1-three-panels", "three"),
    ("v2-bollinger-band", "boll"),
    ("v3-fan", "fan"),
    ("v4-log", "logv"),
    ("v5-conditional", "cond"),
];

/// Depth views: (transcript stem, surface, width, height). Finding `fnd-overbought`.
pub const DEPTHS: &[(&str, &str, u32, u32)] = &[
    ("d1-capsule", "phone", 390, 300),
    ("d2-chart", "desktop", 1200, 600),
];

/// The finding the depth views present.
pub const FINDING: &str = "fnd-overbought";

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

const DAY_MS: i64 = 86_400_000;

/// 0 = Sunday … 6 = Saturday (1970-01-01 was a Thursday).
fn weekday(t: i64) -> i64 {
    (t.div_euclid(DAY_MS) + 4).rem_euclid(7)
}

struct Lcg(u64);

impl Lcg {
    #[allow(clippy::cast_precision_loss)]
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / 9_007_199_254_740_992.0 * 2.0 - 1.0
    }
}

fn read(root: &Path, rel: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))
}

fn sample_bars(root: &Path) -> Result<Vec<Value>, String> {
    read(root, "fixtures/bars/sample.json")?["data"]["bars"]
        .as_array()
        .cloned()
        .ok_or_else(|| "sample.json has no bars".to_owned())
}

fn document(id: &str, title: &str, symbol: &str, bars: Vec<Value>) -> Value {
    json!({
        "schema_version": "0",
        "id": id,
        "title": title,
        "series": [{
            "id": "px",
            "instrument": {"symbol": symbol},
            "interval": "1d",
            "source": {"kind": "inline"},
            "data": {"kind": "bars", "bars": bars}
        }]
    })
}

fn growth_bars(times: &[i64]) -> Vec<Value> {
    let mut rng = Lcg(42);
    let mut trend = 10.0_f64;
    let mut prev_c = round2(9.96);
    let mut out = Vec::with_capacity(times.len());
    for (i, t) in times.iter().enumerate() {
        if i > 0 {
            trend *= 1.0187;
        }
        let (w1, w2, w3, w4) = (rng.next(), rng.next(), rng.next(), rng.next());
        let c = round2(trend * (1.0 + 0.015 * w1));
        let o = prev_c;
        let h = round2(o.max(c) * (1.0 + 0.008 * w2.abs()));
        let l = round2(o.min(c) * (1.0 - 0.008 * w3.abs()));
        let v = round2(1e6 * (1.0 + 0.5 * w4));
        out.push(json!({"t": t, "o": o, "h": h, "l": l, "c": c, "v": v}));
        prev_c = c;
    }
    out
}

/// The six fan quantile series (id, z).
const QUANTILES: &[(&str, f64)] = &[
    ("q05", -1.6449),
    ("q20", -0.8416),
    ("q35", -0.3853),
    ("q65", 0.3853),
    ("q80", 0.8416),
    ("q95", 1.6449),
];

#[allow(clippy::cast_precision_loss)]
fn fan_points(t0: i64, c: f64, z: f64) -> Vec<Value> {
    let mut t = t0;
    let mut out = vec![json!({"t": t0, "v": c})];
    for k in 1..=20 {
        t += DAY_MS;
        while matches!(weekday(t), 0 | 6) {
            t += DAY_MS;
        }
        let k = f64::from(k);
        let v = round2(c * (1.0 + 0.0015 * k + 0.012 * z * k.sqrt()));
        out.push(json!({"t": t, "v": v}));
    }
    out
}

fn owner_init(doc: &str) -> Value {
    json!({
        "document": doc,
        "principals": [{"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"}],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0
    })
}

fn hello(surface: &str) -> Value {
    json!({
        "send": {"conn": "a", "msg": {"type": "hello", "protocol": "qp/0", "principal": "arkin",
                 "surface": {"kind": surface}}},
        "expect": [{"conn": "a", "msg": {"type": "welcome", "principal": "arkin", "role": "owner"}}]
    })
}

fn command(id: &str, ops: &Value) -> Value {
    json!({
        "send": {"conn": "a", "msg": {"type": "command", "id": id, "ops": ops}},
        "expect": [{"conn": "a", "msg": {"type": "ack", "id": id, "status": "applied"}}]
    })
}

fn transcript(
    name: &str,
    description: &str,
    covers: &[&str],
    init: Value,
    steps: Vec<Value>,
    views: &[&str],
) -> Value {
    let views: Vec<Value> = views.iter().map(|v| json!({"id": v})).collect();
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init,
        "steps": steps,
        "final": {"document": {"views": views}}
    })
}

const BASE: &str = "fixtures/views/docs/base.json";
const GROWTH: &str = "fixtures/views/docs/growth.json";
const THYAO: &str = "fixtures/protocol/init/thyao.json";

#[allow(clippy::too_many_lines)]
fn transcripts(root: &Path) -> Result<Vec<(String, Value)>, String> {
    let bars = sample_bars(root)?;
    let last = bars.last().ok_or("no bars")?;
    let t0 = last["t"].as_i64().ok_or("bar t")?;
    let c = last["c"].as_f64().ok_or("bar c")?;
    let mut out = Vec::new();

    out.push((
        "v0-r0".to_owned(),
        transcript(
            "v0-r0",
            "V0: the R0 scene (candles + SMA(20)) built only from protocol messages; renders byte-identical to fixtures/golden/sample.png at 1200x600.",
            &["grammar", "add_study", "put_view"],
            owner_init(BASE),
            vec![
                hello("desktop"),
                command("c1", &json!([
                    {"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20}},
                    {"op":"put_view","view":{"id":"r0","x":{"kind":"time"},
                      "panels":[{"id":0,"y":{"kind":"linear"}}],
                      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
                                {"id":"sma","mark":"line","series":"sma20","panel":0}]}}
                ])),
            ],
            &["r0"],
        ),
    ));

    out.push((
        "v1-three-panels".to_owned(),
        transcript(
            "v1-three-panels",
            "V1 (a): price, RSI and MACD histogram panels; layers inserted by id; 70/30 rules bind to the named thresholds rsi.overbought/rsi.oversold; histogram colour by sign (macd.zero).",
            &["grammar", "add_study", "put_view", "insert_layer"],
            owner_init(BASE),
            vec![
                hello("desktop"),
                command("c1", &json!([
                    {"op":"add_study","id":"rsi14","study":"rsi","inputs":["px"]},
                    {"op":"add_study","id":"macd","study":"macd","inputs":["px"]},
                    {"op":"put_view","view":{"id":"three","x":{"kind":"time"},
                      "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
                                {"id":1,"name":"rsi","y":{"kind":"linear","domain":[0,100]}},
                                {"id":2,"name":"macd","y":{"kind":"linear"}}],
                      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}}
                ])),
                command("c2", &json!([
                    {"op":"insert_layer","view":"three","layer":{"id":"rsi","mark":"line","series":"rsi14","panel":1}},
                    {"op":"insert_layer","view":"three","layer":{"id":"ob","mark":"rule","panel":1,"encoding":{"y":70}}},
                    {"op":"insert_layer","view":"three","layer":{"id":"os","mark":"rule","panel":1,"encoding":{"y":30}}},
                    {"op":"insert_layer","view":"three","layer":{"id":"hist","mark":"bar","series":"macd","panel":2,
                      "encoding":{"y":"histogram",
                        "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}}
                ])),
            ],
            &["three"],
        ),
    ));

    out.push((
        "v2-bollinger-band".to_owned(),
        transcript(
            "v2-bollinger-band",
            "V2 (b): Bollinger band as a filled band between two outputs, under the candles; middle line; theme colour accent.",
            &["grammar", "add_study", "put_view"],
            owner_init(BASE),
            vec![
                hello("desktop"),
                command("c1", &json!([
                    {"op":"add_study","id":"bb","study":"bollinger","inputs":["px"]},
                    {"op":"put_view","view":{"id":"boll","x":{"kind":"time"},
                      "panels":[{"id":0,"y":{"kind":"linear"}}],
                      "layers":[
                        {"id":"band","mark":"band","series":"bb","panel":0,
                         "encoding":{"y":"upper","y2":"lower","color":"accent","opacity":0.18}},
                        {"id":"mid","mark":"line","series":"bb","panel":0,"encoding":{"y":"middle","color":"accent"}},
                        {"id":"candles","mark":"candle","series":"px","panel":0}]}}
                ])),
            ],
            &["boll"],
        ),
    ));

    let mut fan_ops: Vec<Value> = QUANTILES
        .iter()
        .map(|(id, z)| {
            json!({"op":"put_series","series":{"id":id,"source":{"kind":"inline"},
                "data":{"kind":"points","points":fan_points(t0, c, *z)}}})
        })
        .collect();
    fan_ops.push(json!({"op":"put_view","view":{"id":"fan","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}}],
      "layers":[
        {"id":"candles","mark":"candle","series":"px","panel":0},
        {"id":"b90","mark":"band","series":"q05","panel":0,
         "encoding":{"y":"v","y2":{"series":"q95","field":"v"},"color":"accent","opacity":0.15}},
        {"id":"b60","mark":"band","series":"q20","panel":0,
         "encoding":{"y":"v","y2":{"series":"q80","field":"v"},"color":"accent","opacity":0.22}},
        {"id":"b30","mark":"band","series":"q35","panel":0,
         "encoding":{"y":"v","y2":{"series":"q65","field":"v"},"color":"accent","opacity":0.30}}]}}));
    out.push((
        "v3-fan".to_owned(),
        transcript(
            "v3-fan",
            "V3 (c): uncertainty fan; six synthetic inline point series q05..q95 from the last bar 20 sessions ahead (formula in crates/qp-cli/tests/support/views_gen.rs); bands between two different series; x extends past the last bar.",
            &["grammar", "put_series", "put_view"],
            owner_init(BASE),
            vec![hello("desktop"), command("c1", &Value::Array(fan_ops))],
            &["fan"],
        ),
    ));

    out.push((
        "v4-log".to_owned(),
        transcript(
            "v4-log",
            "V4 (d): log value scale on the synthetic growth document (~10 -> ~400); the view is put with a linear scale and switched to log by id with set_scale.",
            &["grammar", "put_view", "set_scale"],
            owner_init(GROWTH),
            vec![
                hello("desktop"),
                command("c1", &json!([
                    {"op":"put_view","view":{"id":"logv","x":{"kind":"time"},
                      "panels":[{"id":0,"y":{"kind":"linear"}}],
                      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}}
                ])),
                command("c2", &json!([{"op":"set_scale","view":"logv","scale":"y0","kind":"log"}])),
            ],
            &["logv"],
        ),
    ));

    out.push((
        "v5-conditional".to_owned(),
        transcript(
            "v5-conditional",
            "V5 (e): conditional colours: area between close and SMA(50) coloured up/down by which is above (split at the crossing); RSI line coloured by value steps at 30/70 (rsi.* thresholds).",
            &["grammar", "add_study", "put_view"],
            owner_init(BASE),
            vec![
                hello("desktop"),
                command("c1", &json!([
                    {"op":"add_study","id":"sma50","study":"sma","inputs":["px"],"params":{"period":50}},
                    {"op":"add_study","id":"rsi14","study":"rsi","inputs":["px"]},
                    {"op":"put_view","view":{"id":"cond","x":{"kind":"time"},
                      "panels":[{"id":0,"y":{"kind":"linear"}},{"id":1,"y":{"kind":"linear","domain":[0,100]}}],
                      "layers":[
                        {"id":"fill","mark":"area","series":"px","panel":0,
                         "encoding":{"y":"c","y2":{"series":"sma50","field":"sma"},"opacity":0.3,
                           "color":{"when":{"field":"c","op":"ge","than":{"series":"sma50","field":"sma"}},
                                    "then":"up","else":"down"}}},
                        {"id":"close","mark":"line","series":"px","panel":0,"encoding":{"y":"c","color":"text"}},
                        {"id":"rsi","mark":"line","series":"rsi14","panel":1,
                         "encoding":{"color":{"field":"rsi","thresholds":[30,70],"colors":["up","muted","down"]}}}]}}
                ])),
            ],
            &["cond"],
        ),
    ));

    out.push((
        "d1-capsule".to_owned(),
        transcript(
            "d1-capsule",
            "D1: the presenter at capsule depth (phone): one panel, the fact's series over the last max_points (60) bars as a line, a rule at the fact's level (origin: the fact), a point at the fact bar. Rendered with `qp render --present fnd-overbought --surface phone`.",
            &["presenter", "present", "depth"],
            json!(THYAO),
            vec![
                hello("phone"),
                json!({
                    "send": {"conn": "a", "msg": {"type": "query", "id": "q1",
                             "query": {"kind": "present", "finding": FINDING}}},
                    "expect": [{"conn": "a", "msg": {"type": "result", "id": "q1",
                        "data": {"kind": "presentation", "presentation": {
                            "finding": FINDING, "depth": "capsule", "surface": "phone",
                            "capsule_view": "$any"}}}}]
                }),
            ],
            &["main"],
        ),
    ));

    out.push((
        "d2-chart".to_owned(),
        transcript(
            "d2-chart",
            "D2: the presenter at chart depth (desktop): show_finding builds the presenter's view (candles, RSI panel, level rule, highlight on the fact bar) and binds it to the capsule; present returns it. Rendered with `qp render --present fnd-overbought --surface desktop`.",
            &["presenter", "show_finding", "present", "depth"],
            json!(THYAO),
            vec![
                hello("desktop"),
                command(
                    "c1",
                    &json!([{"op":"show_finding","finding":FINDING,"view":"v-overbought"}]),
                ),
                json!({
                    "send": {"conn": "a", "msg": {"type": "query", "id": "q1",
                             "query": {"kind": "present", "finding": FINDING}}},
                    "expect": [{"conn": "a", "msg": {"type": "result", "id": "q1",
                        "data": {"kind": "presentation", "presentation": {
                            "finding": FINDING, "depth": "chart", "surface": "desktop",
                            "view": {"id": "v-overbought"}}}}}]
                }),
            ],
            &["main", "v-overbought"],
        ),
    ));
    Ok(out)
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("JSON");
    s.push('\n');
    s
}

/// Every generated file: (path relative to the repository root, content).
///
/// # Errors
/// The sample bars cannot be read.
pub fn files(root: &Path) -> Result<Vec<(String, String)>, String> {
    let bars = sample_bars(root)?;
    let times: Vec<i64> = bars.iter().filter_map(|b| b["t"].as_i64()).collect();
    let mut out = vec![
        (
            BASE.to_owned(),
            pretty(&document(
                "doc-base",
                "base: fixtures/bars/sample.json",
                "SAMPLE",
                bars,
            )),
        ),
        (
            GROWTH.to_owned(),
            pretty(&document(
                "doc-growth",
                "growth: synthetic ~10 -> ~400",
                "GROWTH",
                growth_bars(&times),
            )),
        ),
    ];
    for (stem, t) in transcripts(root)? {
        out.push((format!("fixtures/views/{stem}.json"), pretty(&t)));
    }
    Ok(out)
}
