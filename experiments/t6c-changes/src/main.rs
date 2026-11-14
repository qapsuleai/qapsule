//! RD1/T6c experiment driver. Usage: `t6c-changes <start-doc.json> <out-dir>`.
//!
//! Alternatives measured per workload step:
//! - A1  `json_patch::diff` (crate 5.0.0) between document states
//! - A2  LCS / id-aware JSON Patch diff (best generic diff, jsondiffpatch-style)
//! - A3  JSON Patch emitted by the authority from what it did (no diffing)
//! - M   RFC 7396 merge patch (diff), applied with `json_patch::merge`
//! - B   `ChangeOp` list (protocol §5.3)
//! - Bop the requested `Op` list as log unit (protocol §5.1)
//! - C   hybrid: requested `Op` + A3 patch payload

mod model;
mod patch;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

use flate2::write::GzEncoder;
use flate2::Compression;
use json_patch::Patch;
use model::{Authority, ExpandCfg};
use qp_schema::protocol::{ChangeOp, Op};
use qp_schema::{Bar, Document, Layer, SeriesData};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gz(s: &[u8]) -> usize {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(s).expect("gz");
    e.finish().expect("gz").len()
}

fn time<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let r = std::hint::black_box(f());
    (r, t.elapsed().as_secs_f64() * 1e6)
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("nan"));
    v[v.len() / 2]
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("nan"));
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).expect("op json")
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Deterministic LCG (experiment data only; not calc code).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn bar_after(&mut self, prev: &Bar) -> Bar {
        let o = prev.c;
        self.bar_at(prev.t + 86_400_000, o)
    }
    fn bar_at(&mut self, t: i64, o: f64) -> Bar {
        let c = r2(o * (1.0 + (self.next() - 0.5) * 0.04));
        let h = r2(o.max(c) * (1.0 + self.next() * 0.01)).max(o.max(c));
        let l = r2(o.min(c) * (1.0 - self.next() * 0.01)).min(o.min(c));
        let v = r2(500_000.0 + self.next() * 1_500_000.0);
        Bar { t, o, h, l, c, v }
    }
}

fn px_bars(doc: &Document) -> &Vec<Bar> {
    match &doc.series[0].data {
        Some(SeriesData::Bars { bars }) => bars,
        _ => panic!("px bars"),
    }
}

fn extend_bars(doc: &mut Document, n: usize) {
    let mut g = Lcg(0x5EED_7C6C);
    if let Some(SeriesData::Bars { bars }) = &mut doc.series[0].data {
        while bars.len() < n {
            let b = g.bar_after(bars.last().expect("bar"));
            bars.push(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Workload
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Act {
    Ops(Vec<Op>),
    UndoStep(usize),
}

#[derive(Clone)]
struct Step {
    name: &'static str,
    principal: &'static str,
    act: Act,
}

fn at_of(step: usize) -> i64 {
    1_760_000_000_000 + step as i64 * 60_000
}

fn workload(base: &Document) -> Vec<Step> {
    let bars = px_bars(base);
    let n = bars.len() as u64;
    let last = bars.last().expect("bar").clone();
    let mut g = Lcg(0xBA25);
    let b_new = g.bar_after(&last);
    let mut b_upd1 = g.bar_at(b_new.t, b_new.o);
    b_upd1.v = r2(b_new.v + 120_000.0);
    let mut b_upd2 = g.bar_at(b_new.t, b_new.o);
    b_upd2.v = r2(b_upd1.v + 80_000.0);
    let mut more = vec![];
    let mut p = b_upd2.clone();
    for _ in 0..3 {
        let b = g.bar_after(&p);
        more.push(b.clone());
        p = b;
    }
    let s = |name, principal, v: Value| Step {
        name,
        principal,
        act: Act::Ops(vec![op(v)]),
    };
    let u = |name, k| Step {
        name,
        principal: "arkin",
        act: Act::UndoStep(k),
    };
    vec![
        s("add_study sma20 (+show)", "arkin", json!({"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20.0},"show":{"view":"main","panel":0}})),
        s("add_study ema50 (+show)", "arkin", json!({"op":"add_study","id":"ema50","study":"ema","inputs":["px"],"params":{"period":50.0},"show":{"view":"main","panel":0}})),
        s("add_study rsi7 (+show with rsi14)", "scout", json!({"op":"add_study","id":"rsi7","study":"rsi","inputs":["px"],"params":{"period":7.0},"show":{"view":"main","with":"rsi14"}})),
        s("set_params ema50 period=34", "arkin", json!({"op":"set_params","series":"ema50","params":{"period":34.0}})),
        s("set_params rsi14 period=21", "arkin", json!({"op":"set_params","series":"rsi14","params":{"period":21.0}})),
        s("set_view_range main last 120", "arkin", json!({"op":"set_view_range","view":"main","from_bar":n-120,"to_bar":n-1})),
        s("add_layer rule30 at index 4", "arkin", json!({"op":"add_layer","view":"main","index":4,"layer":{"mark":"rule","panel":1,"encoding":{"color":"#9e9e9e","y":"30"}}})),
        s("remove_layer index 1 (bb20 line)", "arkin", json!({"op":"remove_layer","view":"main","index":1})),
        s("add_fact sma cross", "scout", json!({"op":"add_fact","id":"fact-sma-cross","kind":"cross_above","series":"sma20","bar":n-5,"text":"SMA(20) EMA(34)'ü yukarı kesti.","evidence":{"left":121.4,"right":121.1}})),
        s("add_finding + capsule", "scout", json!({"op":"add_finding","id":"fnd-trend","claim":"THYAO kısa vadeli yükseliş trendine girdi.","facts":["fact-sma-cross"],"confidence":0.6,"invalidation":{"kind":"compare","left":"sma20","op":"cross_below","right":"ema50"},"capsule":"THYAO: SMA20, EMA34'ün üstüne çıktı."})),
        s("update_finding invalidation+confidence", "scout", json!({"op":"update_finding","finding":"fnd-trend","confidence":0.7,"invalidation":{"kind":"threshold","series":"px","op":"lt","value":110.0}})),
        s("reject_finding fnd-overbought", "arkin", json!({"op":"reject_finding","finding":"fnd-overbought","note":"RSI(21) ile aşırı alım teyit edilmedi."})),
        s("append_bars px (1 bar)", "feed", json!({"op":"append_bars","series":"px","bars":[b_new]})),
        s("update_last_bar px", "feed", json!({"op":"update_last_bar","series":"px","bar":b_upd1})),
        s("update_last_bar px", "feed", json!({"op":"update_last_bar","series":"px","bar":b_upd2})),
        s("add_intent", "arkin", json!({"op":"add_intent","id":"intent-break","text":"THYAO 110'un altına inerse haber ver.","trigger":{"kind":"threshold","series":"px","op":"lt","value":110.0}})),
        s("remove macd (cascade layer)", "arkin", json!({"op":"remove","id":"macd"})),
        u("undo #4 (set_params ema50)", 4),
        s("add_study bb10 (+show)", "scout", json!({"op":"add_study","id":"bb10","study":"bollinger","inputs":["px"],"params":{"period":10.0},"show":{"view":"main","panel":0}})),
        u("undo #7 (add_layer rule30)", 7),
        u("undo #12 (reject_finding)", 12),
        s("append_bars px (3 bars)", "feed", json!({"op":"append_bars","series":"px","bars":more})),
        s("remove rsi7 (cascade layer)", "arkin", json!({"op":"remove","id":"rsi7"})),
        s("remove bb20 (no layer left)", "arkin", json!({"op":"remove","id":"bb20"})),
        u("undo #6 (set_view_range)", 6),
        s("set_params bb10 k=2.5", "arkin", json!({"op":"set_params","series":"bb10","params":{"k":2.5}})),
        s("approve_finding fnd-trend", "arkin", json!({"op":"approve_finding","finding":"fnd-trend","note":"Hacimle teyitli."})),
        s("add_study sma200 (+show)", "scout", json!({"op":"add_study","id":"sma200","study":"sma","inputs":["px"],"params":{"period":200.0},"show":{"view":"main","panel":0}})),
        u("undo #24 (remove bb20)", 24),
    ]
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

const ALTS: [&str; 7] = ["A1", "A2", "A3", "M", "B", "Bop", "C"];
const TIMES: [&str; 17] = [
    "B_expand",
    "B_inverse",
    "B_ser",
    "A3_emit",
    "auth_apply",
    "to_value",
    "A1_diff",
    "A2_diff",
    "M_diff",
    "B_parse",
    "B_apply",
    "A1_parse",
    "A1_apply",
    "A3_parse",
    "A3_apply",
    "M_parse",
    "M_apply",
];

#[derive(Default, Clone)]
struct StepRec {
    name: String,
    ok: bool,
    err: String,
    bytes: BTreeMap<&'static str, usize>,
    gz: BTreeMap<&'static str, usize>,
    inv_bytes: BTreeMap<&'static str, usize>,
    nops: BTreeMap<&'static str, usize>,
    times: BTreeMap<&'static str, f64>,
    msgs: BTreeMap<&'static str, String>,
    logentry_bytes: usize,
}

struct RunOut {
    recs: Vec<StepRec>,
    auth: Authority,
    /// Document value after each step (index = step number, 0 = start).
    states: Vec<Value>,
    seq_of_step: BTreeMap<usize, u64>,
}

#[allow(clippy::too_many_lines)]
fn run_pipeline(base: &Document, steps: &[Step], collect: bool, check: bool) -> RunOut {
    let mut auth = Authority::new(base.clone(), ExpandCfg::v1());
    let mut prev = serde_json::to_value(&auth.doc).expect("v");
    let mut mb = auth.doc.clone();
    let mut ma1 = prev.clone();
    let mut ma2 = prev.clone();
    let mut ma3 = prev.clone();
    let mut mm = prev.clone();
    let mut states = vec![];
    if collect {
        states.push(prev.clone());
    }
    let mut recs = vec![];
    let mut seq_of_step = BTreeMap::new();
    for (i, st) in steps.iter().enumerate() {
        let k = i + 1;
        let at = at_of(k);
        let ops = match &st.act {
            Act::Ops(o) => o.clone(),
            Act::UndoStep(t) => vec![Op::Undo {
                seq: seq_of_step.get(t).copied(),
            }],
        };
        let mut r = StepRec {
            name: format!("{k:02} {}", st.name),
            ..Default::default()
        };
        let (plan, t) = time(|| auth.plan(st.principal, at, &ops));
        r.times.insert("B_expand", t);
        let (changes, undoes) = match plan {
            Ok(p) => p,
            Err(e) => {
                r.err = e;
                if collect {
                    states.push(prev.clone());
                }
                recs.push(r);
                continue;
            }
        };
        r.ok = true;
        // Authority: emit A3 from each change against the state before it, then apply.
        let mut a3 = vec![];
        let (mut t_emit, mut t_apply, mut t_inv) = (0.0, 0.0, 0.0);
        let mut inverse = vec![];
        for c in &changes {
            let (mut i, t) = time(|| model::inverse_one(&auth.doc, c));
            t_inv += t;
            i.reverse();
            inverse.extend(i);
            let ((), t) = time(|| patch::emit_one(&auth.doc, c, &mut a3));
            t_emit += t;
            let (res, t) = time(|| model::apply(&mut auth.doc, c));
            res.expect("auth apply");
            t_apply += t;
        }
        inverse.reverse();
        let a3 = Patch(a3);
        r.times.insert("B_inverse", t_inv);
        r.times.insert("A3_emit", t_emit);
        r.times.insert("auth_apply", t_apply);
        let (s_b, t) = time(|| serde_json::to_string(&changes).expect("s"));
        r.times.insert("B_ser", t);
        let (after, t) = time(|| serde_json::to_value(&auth.doc).expect("v"));
        r.times.insert("to_value", t);
        let (a1, t) = time(|| json_patch::diff(&prev, &after));
        r.times.insert("A1_diff", t);
        let (a2, t) = time(|| patch::diff_lcs(&prev, &after));
        r.times.insert("A2_diff", t);
        let (m, t) = time(|| patch::merge_diff(&prev, &after));
        r.times.insert("M_diff", t);
        let s_a1 = serde_json::to_string(&a1).expect("s");
        let s_a2 = serde_json::to_string(&a2).expect("s");
        let s_a3 = serde_json::to_string(&a3).expect("s");
        let s_m = serde_json::to_string(&m).expect("s");
        let s_op = serde_json::to_string(&ops).expect("s");
        let s_c = serde_json::to_string(&json!({"request": ops, "patch": a3})).expect("s");
        // Client mirrors: decode + apply.
        let (cs, t) = time(|| serde_json::from_str::<Vec<ChangeOp>>(&s_b).expect("p"));
        r.times.insert("B_parse", t);
        let ((), t) = time(|| {
            for c in &cs {
                model::apply(&mut mb, c).expect("mirror");
            }
        });
        r.times.insert("B_apply", t);
        for (s, mirror, kp, ka) in [
            (&s_a1, &mut ma1, "A1_parse", "A1_apply"),
            (&s_a3, &mut ma3, "A3_parse", "A3_apply"),
        ] {
            let (p, t) = time(|| serde_json::from_str::<Patch>(s).expect("p"));
            r.times.insert(kp, t);
            let (res, t) = time(|| json_patch::patch(mirror, &p));
            res.expect("patch apply");
            r.times.insert(ka, t);
        }
        json_patch::patch(&mut ma2, &a2).expect("a2");
        let (mv, t) = time(|| serde_json::from_str::<Value>(&s_m).expect("p"));
        r.times.insert("M_parse", t);
        let ((), t) = time(|| json_patch::merge(&mut mm, &mv));
        r.times.insert("M_apply", t);
        if check {
            assert_eq!(ma1, after, "A1 mirror diverged at {}", r.name);
            assert_eq!(ma2, after, "A2 mirror diverged at {}", r.name);
            assert_eq!(ma3, after, "A3 mirror diverged at {}", r.name);
            assert_eq!(mm, after, "M mirror diverged at {}", r.name);
            assert_eq!(mb, auth.doc, "B mirror diverged at {}", r.name);
        }
        if collect {
            let inv_a1 = json_patch::diff(&after, &prev);
            let inv_a2 = patch::diff_lcs(&after, &prev);
            r.inv_bytes.insert("A1", serde_json::to_string(&inv_a1).expect("s").len());
            r.inv_bytes.insert("A2", serde_json::to_string(&inv_a2).expect("s").len());
            r.inv_bytes.insert("B", serde_json::to_string(&inverse).expect("s").len());
            for (alt, s) in [
                ("A1", &s_a1),
                ("A2", &s_a2),
                ("A3", &s_a3),
                ("M", &s_m),
                ("B", &s_b),
                ("Bop", &s_op),
                ("C", &s_c),
            ] {
                r.bytes.insert(alt, s.len());
                r.gz.insert(alt, gz(s.as_bytes()));
                r.msgs.insert(alt, s.clone());
            }
            r.nops.insert("A1", a1.len());
            r.nops.insert("A2", a2.len());
            r.nops.insert("A3", a3.len());
            r.nops.insert("B", changes.len());
            r.nops.insert("Bop", ops.len());
            states.push(after.clone());
        }
        let summary = st.name;
        let e = auth.record(st.principal, at, &ops, changes, inverse, undoes, summary);
        r.logentry_bytes = serde_json::to_string(e).expect("s").len();
        seq_of_step.insert(k, e.seq);
        prev = after;
        recs.push(r);
    }
    RunOut {
        recs,
        auth,
        states,
        seq_of_step,
    }
}

fn measure_workload(base: &Document, runs: usize, out: &str, tag: &str, log: &mut String) -> RunOut {
    let steps = workload(base);
    let first = run_pipeline(base, &steps, true, true);
    let mut times: Vec<BTreeMap<&'static str, Vec<f64>>> = vec![BTreeMap::new(); steps.len()];
    for _ in 0..runs {
        let r = run_pipeline(base, &steps, false, false);
        for (i, rec) in r.recs.iter().enumerate() {
            for (k, v) in &rec.times {
                times[i].entry(k).or_default().push(*v);
            }
        }
    }
    // CSV
    let mut csv = String::from("step,ok,err");
    for a in ALTS {
        let _ = write!(csv, ",{a}_bytes,{a}_gz");
    }
    csv.push_str(",A1_nops,A2_nops,A3_nops,B_nops,inv_A1,inv_A2,inv_B,logentry_B");
    for t in TIMES {
        let _ = write!(csv, ",{t}_us");
    }
    csv.push('\n');
    let mut tot: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut tot_t: BTreeMap<&str, f64> = BTreeMap::new();
    for (i, r) in first.recs.iter().enumerate() {
        let _ = write!(csv, "\"{}\",{},\"{}\"", r.name, r.ok, r.err);
        for a in ALTS {
            let b = r.bytes.get(a).copied().unwrap_or(0);
            let g = r.gz.get(a).copied().unwrap_or(0);
            let e = tot.entry(a).or_default();
            e.0 += b;
            e.1 += g;
            let _ = write!(csv, ",{b},{g}");
        }
        for k in ["A1", "A2", "A3", "B"] {
            let _ = write!(csv, ",{}", r.nops.get(k).copied().unwrap_or(0));
        }
        for k in ["A1", "A2", "B"] {
            let _ = write!(csv, ",{}", r.inv_bytes.get(k).copied().unwrap_or(0));
        }
        let _ = write!(csv, ",{}", r.logentry_bytes);
        for t in TIMES {
            let v = times[i].get_mut(t).map_or(0.0, |v| median(v));
            *tot_t.entry(t).or_default() += v;
            let _ = write!(csv, ",{v:.2}");
        }
        csv.push('\n');
    }
    std::fs::write(format!("{out}/steps_{tag}.csv"), csv).expect("write");
    // Stream (whole log) gzip per alternative.
    let _ = writeln!(log, "\n== workload {tag}: totals over {} steps ==", first.recs.len());
    let _ = writeln!(log, "alt   sum_bytes  sum_gz(per-msg)  stream_gz");
    for a in ALTS {
        let stream: String = first
            .recs
            .iter()
            .filter_map(|r| r.msgs.get(a))
            .map(|s| format!("{s}\n"))
            .collect();
        let (b, g) = tot[a];
        let _ = writeln!(log, "{a:<5} {b:>10} {g:>15} {:>10}", gz(stream.as_bytes()));
    }
    let _ = writeln!(log, "median-of-{runs} total times (us):");
    for t in TIMES {
        let _ = writeln!(log, "  {t:<10} {:>12.1}", tot_t[t]);
    }
    first
}

// ---------------------------------------------------------------------------
// Replay determinism
// ---------------------------------------------------------------------------

fn replay(base: &Document, run: &RunOut, log: &mut String) {
    let truth = serde_json::to_string(&run.auth.doc).expect("s");
    let _ = writeln!(log, "\n== replay determinism (start doc + log -> bytes) ==");
    // B
    let (doc, t) = time(|| {
        let mut d = base.clone();
        for e in &run.auth.log {
            let s = serde_json::to_string(&e.changes).expect("s");
            for c in serde_json::from_str::<Vec<ChangeOp>>(&s).expect("p") {
                model::apply(&mut d, &c).expect("replay");
            }
        }
        d
    });
    let ok = serde_json::to_string(&doc).expect("s") == truth;
    let _ = writeln!(log, "B  ChangeOp log: bytes equal = {ok} ({t:.0} us)");
    for a in ["A1", "A2", "A3", "M"] {
        let (v, t) = time(|| {
            let mut v = serde_json::to_value(base).expect("v");
            for r in run.recs.iter().filter(|r| r.ok) {
                let s = &r.msgs[a];
                if a == "M" {
                    json_patch::merge(&mut v, &serde_json::from_str::<Value>(s).expect("p"));
                } else {
                    json_patch::patch(&mut v, &serde_json::from_str::<Patch>(s).expect("p"))
                        .expect("replay");
                }
            }
            v
        });
        let raw = serde_json::to_string(&v).expect("s") == truth;
        let typed: Document = serde_json::from_value(v).expect("typed");
        let rt = serde_json::to_string(&typed).expect("s") == truth;
        let _ = writeln!(
            log,
            "{a:<3} patch log: raw Value bytes equal = {raw}; via typed Document = {rt} ({t:.0} us)"
        );
    }
    for (name, cfg) in [("v1", ExpandCfg::v1()), ("v2", ExpandCfg::v2())] {
        let mut a = Authority::new(base.clone(), cfg);
        let mut failed = 0;
        let mut differ = 0;
        let ((), t) = time(|| {
            for e in &run.auth.log {
                match a.execute(&e.author, e.at, &e.request, &e.summary) {
                    Ok(n) => {
                        if n.changes != e.changes {
                            differ += 1;
                        }
                    }
                    Err(_) => {
                        failed += 1;
                        a.skip();
                    }
                }
            }
        });
        let eq = serde_json::to_string(&a.doc).expect("s") == truth;
        let _ = writeln!(
            log,
            "Bop Op log replayed with expansion {name}: bytes equal = {eq}; entries with different changes = {differ}; failed = {failed} ({t:.0} us)"
        );
    }
}

// ---------------------------------------------------------------------------
// Undo robustness
// ---------------------------------------------------------------------------

fn ideal_without(base: &Document, steps: &[Step], upto: usize, excluded: &[usize]) -> Value {
    let mut a = Authority::new(base.clone(), ExpandCfg::v1());
    for (i, st) in steps.iter().enumerate().take(upto) {
        let k = i + 1;
        match &st.act {
            Act::Ops(ops) if !excluded.contains(&k) => {
                if a.execute(st.principal, at_of(k), ops, st.name).is_err() {
                    a.skip();
                }
            }
            _ => a.skip(),
        }
    }
    serde_json::to_value(&a.doc).expect("v")
}

fn undo_analysis(base: &Document, run: &RunOut, out: &str, log: &mut String) {
    let steps = workload(base);
    let mut csv = String::from("undo_step,target,B,A1_inverse,A2_inverse,A2_guarded,A2_guarded_id,B_exact_bytes,inv_bytes_B,inv_bytes_A2,inv_bytes_A2_guarded\n");
    let mut undone_ok: Vec<usize> = vec![];
    let _ = writeln!(log, "\n== undo of earlier entries (outcome vs ideal 'as if target never happened') ==");
    for (i, st) in steps.iter().enumerate() {
        let s = i + 1;
        let Act::UndoStep(t) = st.act else { continue };
        let mut excl = undone_ok.clone();
        excl.push(t);
        let ideal = patch::normalize(&ideal_without(base, &steps, s - 1, &excl));
        let current = &run.states[s - 1];
        let (before_t, after_t) = (&run.states[t - 1], &run.states[t]);
        let classify = |res: Result<Value, String>| -> String {
            match res {
                Err(_) => "refused".into(),
                Ok(v) => {
                    if serde_json::from_value::<Document>(v.clone()).is_err() {
                        "WRONG(corrupt)".into()
                    } else if patch::normalize(&v) == ideal {
                        "correct".into()
                    } else {
                        "WRONG".into()
                    }
                }
            }
        };
        let try_patch = |p: &Patch| -> Result<Value, String> {
            let mut v = current.clone();
            json_patch::patch(&mut v, p).map_err(|e| e.to_string())?;
            Ok(v)
        };
        let inv1 = json_patch::diff(after_t, before_t);
        let inv2 = patch::diff_lcs(after_t, before_t);
        let inv2g = patch::guard(after_t, &inv2);
        let inv2s = patch::guard_strong(after_t, &inv2);
        let rec = &run.recs[i];
        let b = if rec.ok {
            undone_ok.push(t);
            classify(Ok(run.states[s].clone()))
        } else {
            format!("refused ({})", rec.err.split(':').next().unwrap_or(""))
        };
        let exact = rec.ok
            && run.states[s] == ideal_without(base, &steps, s - 1, &{
                let mut e = excl.clone();
                e.dedup();
                e
            });
        let inv_b = serde_json::to_string(&run.auth.log[(run.seq_of_step[&t] - 1) as usize].inverse)
            .expect("s")
            .len();
        let row = format!(
            "\"{}\",{t},{b},{},{},{},{},{exact},{inv_b},{},{}",
            rec.name,
            classify(try_patch(&inv1)),
            classify(try_patch(&inv2)),
            classify(try_patch(&inv2g)),
            classify(try_patch(&inv2s)),
            serde_json::to_string(&inv2).expect("s").len(),
            serde_json::to_string(&inv2g).expect("s").len(),
        );
        let _ = writeln!(log, "{row}");
        csv.push_str(&row);
        csv.push('\n');
        let _ = writeln!(log, "   A2 inverse: {}", serde_json::to_string(&inv2).expect("s"));
        let _ = writeln!(
            log,
            "   B  inverse: {}",
            serde_json::to_string(&run.auth.log[(run.seq_of_step[&t] - 1) as usize].inverse)
                .expect("s")
        );
    }
    std::fs::write(format!("{out}/undo.csv"), csv).expect("w");
}

// ---------------------------------------------------------------------------
// Concurrency
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum XOp {
    Std(Op),
    /// Identity-keyed layer removal (hypothetical protocol extension; idempotent).
    RemoveLayerMatch { view: String, layer: Layer },
}

fn to_identity(doc: &Document, o: &Op) -> XOp {
    match o {
        Op::RemoveLayer { view, index } => {
            let v = doc.views.iter().find(|v| &v.id == view).expect("view");
            XOp::RemoveLayerMatch {
                view: view.clone(),
                layer: v.layers[*index as usize].clone(),
            }
        }
        _ => XOp::Std(o.clone()),
    }
}

fn exec_x(a: &mut Authority, principal: &str, at: i64, x: &XOp) -> Result<(), String> {
    match x {
        XOp::Std(o) => a.execute(principal, at, std::slice::from_ref(o), "x").map(|_| ()),
        XOp::RemoveLayerMatch { view, layer } => {
            let v = a.doc.views.iter().find(|v| &v.id == view).ok_or("view")?;
            match v.layers.iter().position(|l| l == layer) {
                None => Ok(()), // already gone: idempotent
                Some(i) => {
                    let o = Op::RemoveLayer {
                        view: view.clone(),
                        index: i as u32,
                    };
                    a.execute(principal, at, &[o], "x").map(|_| ())
                }
            }
        }
    }
}

struct Scn {
    name: &'static str,
    x: (&'static str, Op),
    y: (&'static str, Op),
}

#[allow(clippy::too_many_lines)]
fn concurrency(base_doc: &Document, out: &str, log: &mut String) {
    let base_val = serde_json::to_value(base_doc).expect("v");
    let bars = px_bars(base_doc);
    let mut g = Lcg(0xC0C0);
    let nb = g.bar_after(bars.last().expect("b"));
    let lay = |i: usize| base_doc.views[0].layers[i].clone();
    let _ = writeln!(
        log,
        "\n== concurrency: base layers = {:?}; series = {:?}",
        base_doc.views[0]
            .layers
            .iter()
            .map(|l| format!("{:?}:{}", l.mark, l.series.clone().unwrap_or("-".into())))
            .collect::<Vec<_>>(),
        base_doc.series.iter().map(|s| s.id.clone()).collect::<Vec<_>>()
    );
    let _ = lay;
    let scns = vec![
        Scn { name: "S1 two remove_layer by index (bb20=1, rule70=3)", x: ("arkin", op(json!({"op":"remove_layer","view":"main","index":1}))), y: ("scout", op(json!({"op":"remove_layer","view":"main","index":3}))) },
        Scn { name: "S2 both remove the same layer (bb20=1)", x: ("arkin", op(json!({"op":"remove_layer","view":"main","index":1}))), y: ("scout", op(json!({"op":"remove_layer","view":"main","index":1}))) },
        Scn { name: "S3 add_layer at 0 vs remove_layer rsi14=2", x: ("arkin", op(json!({"op":"add_layer","view":"main","index":0,"layer":{"mark":"area","series":"px","panel":0,"encoding":{}}}))), y: ("scout", op(json!({"op":"remove_layer","view":"main","index":2}))) },
        Scn { name: "S4 remove series macd vs set_params bb20", x: ("arkin", op(json!({"op":"remove","id":"macd"}))), y: ("scout", op(json!({"op":"set_params","series":"bb20","params":{"period":30.0}}))) },
        Scn { name: "S5 set_params ema50 vs remove series sma20", x: ("arkin", op(json!({"op":"set_params","series":"ema50","params":{"period":34.0}}))), y: ("scout", op(json!({"op":"remove","id":"sma20"}))) },
        Scn { name: "S6 two feeds append the same bar", x: ("feed", op(json!({"op":"append_bars","series":"px","bars":[nb]}))), y: ("feed", op(json!({"op":"append_bars","series":"px","bars":[nb]}))) },
        Scn { name: "S7 reject_finding vs update_finding confidence", x: ("arkin", op(json!({"op":"reject_finding","finding":"fnd-overbought","note":"no"}))), y: ("scout", op(json!({"op":"update_finding","finding":"fnd-overbought","confidence":0.4}))) },
        Scn { name: "S8 reject_finding vs approve_finding", x: ("arkin", op(json!({"op":"reject_finding","finding":"fnd-overbought","note":"no"}))), y: ("owner2", op(json!({"op":"approve_finding","finding":"fnd-overbought","note":"yes"}))) },
        Scn { name: "S9 update_finding vs remove rsi7 (disjoint control)", x: ("scout", op(json!({"op":"update_finding","finding":"fnd-overbought","invalidation":{"kind":"threshold","series":"px","op":"lt","value":100.0}}))), y: ("arkin", op(json!({"op":"remove","id":"rsi7"}))) },
    ];
    let alts = ["A-blind", "A-test", "A-test-id", "A-blind+validate", "A-test+validate", "A-test-id+validate", "B-op", "B-op+base", "B-id"];
    let mut counts: BTreeMap<&str, (u32, u32, u32)> = BTreeMap::new();
    let mut csv = String::from("scenario,order");
    for a in alts {
        let _ = write!(csv, ",{a}");
    }
    csv.push('\n');
    let cfg = ExpandCfg::v1();
    let base_viol = patch::validate_doc(base_doc, &cfg).len();
    for sc in &scns {
        for order in [0, 1] {
            let (f, s) = if order == 0 { (&sc.x, &sc.y) } else { (&sc.y, &sc.x) };
            let (at1, at2) = (at_of(100), at_of(101));
            // Oracle: identity semantics, sequential.
            let mut o = Authority::new(base_doc.clone(), cfg.clone());
            let fx = to_identity(base_doc, &f.1);
            let sx = to_identity(base_doc, &s.1);
            exec_x(&mut o, f.0, at1, &fx).expect("oracle first");
            let first_only = patch::normalize(&serde_json::to_value(&o.doc).expect("v"));
            let _oracle_second_ok = exec_x(&mut o, s.0, at2, &sx).is_ok();
            let oracle = patch::normalize(&serde_json::to_value(&o.doc).expect("v"));
            let classify = |v: &Value, second_refused: bool| -> &'static str {
                if serde_json::from_value::<Document>(v.clone()).is_err() {
                    return "WRONG";
                }
                let n = patch::normalize(v);
                if n == oracle {
                    "correct"
                } else if n == first_only && second_refused {
                    "refused"
                } else {
                    "WRONG"
                }
            };
            // Client patches: what each writer computes against the base it saw.
            let client_patch = |p: &(&str, Op), at: i64| -> Patch {
                let a = Authority::new(base_doc.clone(), cfg.clone());
                let (cs, _) = a.plan(p.0, at, std::slice::from_ref(&p.1)).expect("client plan");
                let mut d = base_doc.clone();
                let mut out = vec![];
                for c in &cs {
                    patch::emit_one(&d, c, &mut out);
                    model::apply(&mut d, c).expect("a");
                }
                Patch(out)
            };
            let (pf, ps) = (client_patch(f, at1), client_patch(s, at2));
            let mut row = format!("\"{}\",{}", sc.name, if order == 0 { "X,Y" } else { "Y,X" });
            for alt in alts {
                let res: &str = match alt {
                    a if a.starts_with("A-") => {
                        let guarded = alt.starts_with("A-test");
                        let strong = alt.starts_with("A-test-id");
                        let validate = alt.ends_with("validate");
                        let mut v = base_val.clone();
                        let mut refused = false;
                        for (k, p) in [&pf, &ps].into_iter().enumerate() {
                            let p = if strong {
                                patch::guard_strong(&base_val, p)
                            } else if guarded {
                                patch::guard(&base_val, p)
                            } else {
                                p.clone()
                            };
                            let mut trial = v.clone();
                            let mut ok = json_patch::patch(&mut trial, &p).is_ok();
                            if ok && validate {
                                ok = match serde_json::from_value::<Document>(trial.clone()) {
                                    Ok(d) => patch::validate_doc(&d, &cfg).len() <= base_viol,
                                    Err(_) => false,
                                };
                            }
                            if ok {
                                v = trial;
                            } else if k == 1 {
                                refused = true;
                            }
                        }
                        classify(&v, refused)
                    }
                    "B-op" | "B-op+base" => {
                        let mut a = Authority::new(base_doc.clone(), cfg.clone());
                        a.execute(f.0, at1, std::slice::from_ref(&f.1), "f").expect("first");
                        let mut refused = false;
                        if alt == "B-op+base" {
                            let b = Authority::new(base_doc.clone(), cfg.clone());
                            let (cs, _) = b.plan(s.0, at2, std::slice::from_ref(&s.1)).expect("p");
                            let mine: std::collections::BTreeSet<String> =
                                cs.iter().flat_map(model::touched).collect();
                            if !mine.is_disjoint(&a.touched[0]) {
                                refused = true;
                            }
                        }
                        if !refused {
                            refused = a.execute(s.0, at2, std::slice::from_ref(&s.1), "s").is_err();
                        }
                        classify(&serde_json::to_value(&a.doc).expect("v"), refused)
                    }
                    _ => {
                        let mut a = Authority::new(base_doc.clone(), cfg.clone());
                        exec_x(&mut a, f.0, at1, &fx).expect("first");
                        let refused = exec_x(&mut a, s.0, at2, &sx).is_err();
                        classify(&serde_json::to_value(&a.doc).expect("v"), refused)
                    }
                };
                let c = counts.entry(alt).or_default();
                match res {
                    "correct" => c.0 += 1,
                    "refused" => c.1 += 1,
                    _ => c.2 += 1,
                }
                let _ = write!(row, ",{res}");
            }
            let _ = writeln!(log, "{row}");
            csv.push_str(&row);
            csv.push('\n');
            if order == 0 {
                let _ = writeln!(log, "   X patch: {}", serde_json::to_string(&pf).expect("s"));
                let _ = writeln!(log, "   Y patch: {}", serde_json::to_string(&ps).expect("s"));
            }
        }
    }
    let _ = writeln!(log, "totals (correct, refused, WRONG) over {} runs:", scns.len() * 2);
    for a in alts {
        let c = counts[a];
        let _ = writeln!(log, "  {a:<18} {:>2} {:>2} {:>2}", c.0, c.1, c.2);
    }
    std::fs::write(format!("{out}/concurrency.csv"), csv).expect("w");
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validation(start: &Document, big: &Document, out: &str, log: &mut String) {
    let cfg = ExpandCfg::v1();
    let bars = px_bars(start);
    let n = bars.len();
    let last = bars[n - 1].clone();
    let mut early = last.clone();
    early.t -= 86_400_000;
    let mut moved = last.clone();
    moved.t += 3_600_000;
    let finding0 = serde_json::to_value(&start.findings[0]).expect("v");
    let mut bad_finding = finding0.clone();
    bad_finding["id"] = json!("fnd-x");
    bad_finding["facts"] = json!(["fact-nope"]);
    // Base for V8: start + reject.
    let mut rejected = Authority::new(start.clone(), cfg.clone());
    rejected
        .execute("arkin", at_of(1), &[op(json!({"op":"reject_finding","finding":"fnd-overbought"}))], "r")
        .expect("reject");
    let rejected = rejected.doc;
    let t = |i: usize| bars[i].t as f64;
    type Case = (&'static str, &'static str, Value, Value, bool);
    let cases: Vec<Case> = vec![
        ("V1 layer refers to missing series", "arkin", json!({"op":"add_layer","view":"main","layer":{"mark":"line","series":"nope","panel":0,"encoding":{}}}), json!([{"op":"add","path":"/views/0/layers/-","value":{"mark":"line","series":"nope","panel":0,"encoding":{}}}]), false),
        ("V2 remove series in use (rsi14)", "arkin", json!({"op":"remove","id":"rsi14"}), json!([{"op":"remove","path":"/views/0/layers/2"},{"op":"remove","path":"/series/1"}]), false),
        ("V3 set_params on a bar series", "arkin", json!({"op":"set_params","series":"px","params":{"period":5.0}}), json!([{"op":"add","path":"/series/0/source/params","value":{"period":5.0}}]), false),
        ("V4 finding refers to missing fact", "scout", json!({"op":"add_finding","id":"fnd-x","claim":"x","facts":["fact-nope"],"confidence":0.5,"invalidation":{"kind":"text","text":"x"}}), json!([{"op":"add","path":"/findings/-","value":bad_finding}]), false),
        ("V5 confidence 1.5", "scout", json!({"op":"update_finding","finding":"fnd-overbought","confidence":1.5}), json!([{"op":"replace","path":"/findings/0/confidence","value":1.5}]), false),
        ("V6 appended bar not after last", "feed", json!({"op":"append_bars","series":"px","bars":[early]}), json!([{"op":"add","path":"/series/0/data/bars/-","value":early}]), false),
        ("V7 update_last_bar changes t", "feed", json!({"op":"update_last_bar","series":"px","bar":moved}), json!([{"op":"replace","path":format!("/series/0/data/bars/{}", n-1),"value":moved}]), false),
        ("V8 approve an already rejected finding", "arkin", json!({"op":"approve_finding","finding":"fnd-overbought"}), json!([{"op":"replace","path":"/capsules/0/state","value":"approved"},{"op":"add","path":"/ledger/entries/-","value":{"finding":"fnd-overbought","state":"approved","outcome":"pending","recorded_at":1760000120000_i64}}]), true),
        ("V9 unknown study param", "arkin", json!({"op":"set_params","series":"rsi14","params":{"lenght":7.0}}), json!([{"op":"add","path":"/series/1/source/params/lenght","value":7.0}]), false),
        ("V10 feed writes a finding", "feed", json!({"op":"update_finding","finding":"fnd-overbought","confidence":0.9}), json!([{"op":"replace","path":"/findings/0/confidence","value":0.9}]), false),
        ("V11 remove layer index 99", "arkin", json!({"op":"remove_layer","view":"main","index":99}), json!([{"op":"remove","path":"/views/0/layers/99"}]), false),
        ("V12 reversed view range", "arkin", json!({"op":"set_view_range","view":"main","from_bar":150,"to_bar":100}), json!([{"op":"add","path":"/views/0/x/domain","value":[t(150), t(100)]}]), false),
        ("V13 negative period", "arkin", json!({"op":"set_params","series":"rsi14","params":{"period":-5.0}}), json!([{"op":"replace","path":"/series/1/source/params/period","value":-5.0}]), false),
    ];
    let mut csv = String::from("case,B_expand,A_patch_error,A_typed_serde,A_json_schema,A_post_state_domain,A_feed_acl,A_any\n");
    let _ = writeln!(log, "\n== validation of invalid requests (caught = true) ==");
    let mut tot = [0u32; 7];
    for (name, who, o, p, on_rejected) in &cases {
        let base = if *on_rejected { &rejected } else { start };
        let a = Authority::new(base.clone(), cfg.clone());
        let b = a.plan(who, at_of(2), &[op(o.clone())]).is_err();
        let p: Patch = serde_json::from_value(p.clone()).expect("patch");
        let mut v = serde_json::to_value(base).expect("v");
        let perr = json_patch::patch(&mut v, &p).is_err();
        let (serde_err, schema_err, domain_err) = if perr {
            (false, false, false)
        } else {
            let typed = serde_json::from_value::<Document>(v.clone());
            let schema = qp_schema::validate("Document", &v).is_err();
            let dom = match &typed {
                Ok(d) => patch::validate_doc(d, &cfg).len() > patch::validate_doc(base, &cfg).len(),
                Err(_) => false,
            };
            (typed.is_err(), schema, dom)
        };
        let acl = *who == "feed" && !patch::feed_acl_ok(&p);
        let any = perr || serde_err || schema_err || domain_err || acl;
        for (k, x) in [b, perr, serde_err, schema_err, domain_err, acl, any].iter().enumerate() {
            tot[k] += u32::from(*x);
        }
        let row = format!("\"{name}\",{b},{perr},{serde_err},{schema_err},{domain_err},{acl},{any}");
        let _ = writeln!(log, "{row}");
        csv.push_str(&row);
        csv.push('\n');
    }
    let _ = writeln!(log, "totals: B={} | A: patch_err={} serde={} schema={} domain={} acl={} any={} (of {})", tot[0], tot[1], tot[2], tot[3], tot[4], tot[5], tot[6], cases.len());
    std::fs::write(format!("{out}/validation.csv"), csv).expect("w");
    // Cost of post-state validation per change (A) vs per-op validation (B expand).
    let _ = writeln!(log, "post-state validation cost per change (median of 21):");
    for (tag, d) in [("200 bars", start), ("10000 bars", big)] {
        let mut ts = [vec![], vec![], vec![]];
        let v = serde_json::to_value(d).expect("v");
        let schema = qp_schema::json_schema("Document").expect("schema");
        let validator = jsonschema::validator_for(&schema).expect("compile");
        for _ in 0..21 {
            ts[0].push(time(|| serde_json::from_value::<Document>(v.clone()).is_ok()).1);
            ts[1].push(time(|| validator.is_valid(&v)).1);
            ts[2].push(time(|| patch::validate_doc(d, &cfg).len()).1);
        }
        let _ = writeln!(
            log,
            "  {tag:<10} typed from_value(clone)={:.0} us, JSON Schema (precompiled)={:.0} us, domain={:.0} us",
            median(&mut ts[0]),
            median(&mut ts[1]),
            median(&mut ts[2])
        );
    }
}

// ---------------------------------------------------------------------------
// Hot path: 60 live bar updates/second on 10 000 bars
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn hot_path(big: &Document, out: &str, log: &mut String) {
    let ticks = 600;
    let mut auth = Authority::new(big.clone(), ExpandCfg::v1());
    // Open a forming bar first.
    let mut g = Lcg(0x60F0);
    let first = g.bar_after(px_bars(&auth.doc).last().expect("b"));
    auth.execute("feed", at_of(1), &[Op::AppendBars { series: "px".into(), bars: vec![first.clone()] }], "open")
        .expect("open");
    let mut prev = serde_json::to_value(&auth.doc).expect("v");
    let mut mirror_b = auth.doc.clone();
    let mut mirror_a = prev.clone();
    let names = [
        "B expand+validate+inverse", "B serialize", "B client parse+apply",
        "A3 emit+serialize", "A3 client parse+apply",
        "A1 to_value+diff (crate)", "A1 clone(prev)+diff", "A1 client parse+apply",
        "A2 to_value+diff (LCS)", "M to_value+merge_diff",
    ];
    let mut t: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    let mut sizes: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut examples: BTreeMap<&str, String> = BTreeMap::new();
    let mut cur = first;
    for k in 0..ticks {
        let mut b = cur.clone();
        let c = r2(b.c * (1.0 + (g.next() - 0.5) * 0.002));
        b.c = c;
        b.h = b.h.max(c);
        b.l = b.l.min(c);
        b.v = r2(b.v + 1000.0 + g.next() * 5000.0);
        cur = b.clone();
        let ops = [Op::UpdateLastBar { series: "px".into(), bar: b }];
        let ((cs, _), t1) = time(|| auth.plan("feed", at_of(2 + k), &ops).expect("plan"));
        let (_inv, t1b) = time(|| model::inverse_one(&auth.doc, &cs[0]));
        let (s_b, t2) = time(|| serde_json::to_string(&cs).expect("s"));
        let (s_a3, t3) = time(|| {
            let mut out = vec![];
            patch::emit_one(&auth.doc, &cs[0], &mut out);
            serde_json::to_string(&Patch(out)).expect("s")
        });
        model::apply(&mut auth.doc, &cs[0]).expect("apply");
        let ((after, a1), t4) = time(|| {
            let after = serde_json::to_value(&auth.doc).expect("v");
            let d = json_patch::diff(&prev, &after);
            (after, d)
        });
        let (_, t5) = time(|| {
            let snapshot = prev.clone();
            json_patch::diff(&snapshot, &after)
        });
        let (a2, t6) = time(|| {
            let after2 = serde_json::to_value(&auth.doc).expect("v");
            patch::diff_lcs(&prev, &after2)
        });
        let (m, t7) = time(|| {
            let after2 = serde_json::to_value(&auth.doc).expect("v");
            patch::merge_diff(&prev, &after2)
        });
        let s_a1 = serde_json::to_string(&a1).expect("s");
        let s_a2 = serde_json::to_string(&a2).expect("s");
        let s_m = serde_json::to_string(&m).expect("s");
        let ((), t8) = time(|| {
            for c in serde_json::from_str::<Vec<ChangeOp>>(&s_b).expect("p") {
                model::apply(&mut mirror_b, &c).expect("m");
            }
        });
        let mut mirror_a3 = std::mem::take(&mut mirror_a);
        let ((), t9) = time(|| {
            let p: Patch = serde_json::from_str(&s_a3).expect("p");
            json_patch::patch(&mut mirror_a3, &p).expect("m");
        });
        // A1 apply on a separate mirror copy would need another 10k Value; measure on
        // the same mirror by applying A1 after undoing A3 is overkill: A1 and A3 touch
        // the same bar, so applying A1 on the A3 mirror is idempotent and representative.
        let ((), t10) = time(|| {
            let p: Patch = serde_json::from_str(&s_a1).expect("p");
            json_patch::patch(&mut mirror_a3, &p).expect("m");
        });
        mirror_a = mirror_a3;
        for (n, v) in names.iter().zip([t1 + t1b, t2, t8, t3, t9, t4, t5, t10, t6, t7]) {
            t.entry(n).or_default().push(v);
        }
        for (alt, s) in [("B set_last_bar", &s_b), ("A3 replace bar", &s_a3), ("A1 diff", &s_a1), ("A2 diff", &s_a2), ("M merge", &s_m)] {
            let e = sizes.entry(alt).or_default();
            e.0 += s.len();
            e.1 += gz(s.as_bytes());
            if k == 0 {
                examples.insert(alt, if s.len() > 300 { format!("{}… ({} bytes)", &s[..300], s.len()) } else { s.clone() });
            }
        }
        prev = after;
    }
    assert_eq!(mirror_a, prev, "hot path A mirror");
    assert_eq!(mirror_b, auth.doc, "hot path B mirror");
    let _ = writeln!(log, "\n== hot path: {ticks} update_last_bar on {} bars ==", px_bars(&auth.doc).len());
    let mut csv = String::from("metric,p50_us,p99_us,cpu_at_60hz_pct\n");
    for n in names {
        let v = t.get_mut(n).expect("t");
        let (p50, p99) = (pct(v, 0.5), pct(v, 0.99));
        let line = format!("{n},{p50:.2},{p99:.2},{:.3}", p50 * 60.0 / 1e6 * 100.0);
        let _ = writeln!(log, "  {line}");
        csv.push_str(&line);
        csv.push('\n');
    }
    csv.push_str("\nalt,avg_bytes,avg_gz_bytes,bytes_per_s_at_60hz\n");
    for (alt, (b, g)) in &sizes {
        let line = format!("{alt},{},{},{}", b / ticks, g / ticks, b / ticks * 60);
        let _ = writeln!(log, "  {line}");
        csv.push_str(&line);
        csv.push('\n');
    }
    for (alt, s) in &examples {
        let _ = writeln!(log, "  example {alt}: {s}");
    }
    std::fs::write(format!("{out}/hotpath.csv"), csv).expect("w");
}

// ---------------------------------------------------------------------------
// Agent-writability and intent metrics
// ---------------------------------------------------------------------------

fn count_state_refs(v: &Value) -> (usize, usize) {
    // (numeric path indices, provenance/time fields a writer must invent)
    let mut idx = 0;
    let mut stamp = 0;
    fn walk(v: &Value, idx: &mut usize, stamp: &mut usize) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if k == "path" {
                        if let Some(s) = x.as_str() {
                            *idx += s.split('/').filter(|p| p.parse::<usize>().is_ok()).count();
                        }
                    }
                    if k == "index" {
                        *idx += 1;
                    }
                    if ["provenance", "recorded_at"].contains(&k.as_str()) {
                        *stamp += 1;
                    }
                    walk(x, idx, stamp);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, idx, stamp)),
            _ => {}
        }
    }
    walk(v, &mut idx, &mut stamp);
    (idx, stamp)
}

fn writability(run: &RunOut, out: &str, log: &mut String) {
    let mut csv = String::from("step,Bop_bytes,Bop_idx,Bop_stamps,A3_bytes,A3_ops,A3_idx,A3_stamps,B_bytes,B_idx,B_stamps\n");
    let mut tot = [0usize; 9];
    for r in run.recs.iter().filter(|r| r.ok) {
        let mut row = format!("\"{}\"", r.name);
        let mut k = 0;
        for alt in ["Bop", "A3", "B"] {
            let s = &r.msgs[alt];
            let v: Value = serde_json::from_str(s).expect("v");
            let (i, st) = count_state_refs(&v);
            if alt == "A3" {
                let _ = write!(row, ",{},{},{i},{st}", s.len(), r.nops["A3"]);
            } else {
                let _ = write!(row, ",{},{i},{st}", s.len());
            }
            tot[k] += s.len();
            tot[k + 1] += i;
            tot[k + 2] += st;
            k += 3;
        }
        csv.push_str(&row);
        csv.push('\n');
    }
    let _ = writeln!(log, "\n== agent-writability (sum over applied steps): bytes / index refs / provenance+timestamp fields ==");
    let _ = writeln!(log, "  Bop (Op request)  {:>7} / {:>3} / {:>3}", tot[0], tot[1], tot[2]);
    let _ = writeln!(log, "  A3 (JSON Patch)   {:>7} / {:>3} / {:>3}", tot[3], tot[4], tot[5]);
    let _ = writeln!(log, "  B (ChangeOp)      {:>7} / {:>3} / {:>3}", tot[6], tot[7], tot[8]);
    std::fs::write(format!("{out}/writability.csv"), csv).expect("w");
}

fn examples(run: &RunOut, out: &str) {
    let mut ex = serde_json::Map::new();
    for r in &run.recs {
        if !r.ok {
            continue;
        }
        let mut m = serde_json::Map::new();
        for a in ["Bop", "B", "A1", "A2", "A3", "M"] {
            let s = &r.msgs[a];
            let v: Value = if s.len() > 2500 {
                Value::String(format!("<{} bytes, truncated> {}", s.len(), &s[..400]))
            } else {
                serde_json::from_str(s).expect("v")
            };
            m.insert(a.into(), v);
        }
        ex.insert(r.name.clone(), Value::Object(m));
    }
    std::fs::write(
        format!("{out}/examples.json"),
        serde_json::to_string_pretty(&Value::Object(ex)).expect("s") + "\n",
    )
    .expect("w");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start: Document =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).expect("read")).expect("doc");
    let out = &args[2];
    std::fs::create_dir_all(out).expect("mkdir");
    let mut big = start.clone();
    extend_bars(&mut big, 10_000);
    let mut log = String::new();
    let _ = writeln!(log, "t6c-changes: start doc {} bytes ({} bars); big doc {} bytes ({} bars)",
        serde_json::to_string(&start).expect("s").len(), px_bars(&start).len(),
        serde_json::to_string(&big).expect("s").len(), px_bars(&big).len());
    let small = measure_workload(&start, 21, out, "N200", &mut log);
    eprintln!("workload N200 done");
    let large = measure_workload(&big, 7, out, "N10000", &mut log);
    eprintln!("workload N10000 done");
    let _ = writeln!(log, "\nper-step outcome (N200): ");
    for r in &small.recs {
        let _ = writeln!(log, "  {} -> {}", r.name, if r.ok { "applied".to_owned() } else { format!("REFUSED: {}", r.err) });
    }
    replay(&start, &small, &mut log);
    replay(&big, &large, &mut log);
    undo_analysis(&start, &small, out, &mut log);
    // Concurrency base: start doc + first three workload steps (7 series, 8 layers).
    let mut cb = Authority::new(start.clone(), ExpandCfg::v1());
    for (i, st) in workload(&start).iter().take(3).enumerate() {
        if let Act::Ops(o) = &st.act {
            cb.execute(st.principal, at_of(i + 1), o, st.name).expect("cbase");
        }
    }
    concurrency(&cb.doc, out, &mut log);
    validation(&start, &big, out, &mut log);
    hot_path(&big, out, &mut log);
    writability(&small, out, &mut log);
    examples(&small, out);
    std::fs::write(format!("{out}/run.log"), &log).expect("w");
    print!("{log}");
}
