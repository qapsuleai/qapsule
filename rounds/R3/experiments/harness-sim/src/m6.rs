//! M6 — server-fact trust path (SPEC M6).
//!
//! Setting: one trust-path instance per instrument stream (fresh from the factory),
//! observing every bar in order: `FlowObservation` (all raw flow values of the bar) and
//! the server facts of that bar (the `flow.institutional` detector, run on the same flow).
//!
//! 1. **Oracle-equivalence attack** (Kerckhoffs; the attacker knows the algorithm and
//!    every raw value except the target bar's). For a target bar `b`, its `net` is
//!    replaced by each of 2,001 grid values `v_k = net_b + 3·|net_b|·(k − 1000)/1000`
//!    (±300 %; if `|net_b| < 1`, the instrument's median `|net|` is the scale); `buy`,
//!    `sell`, `brokers_net_top5` of the bar move consistently (`buy − sell = net`, top-5
//!    scaled by `v/net`). The detector and the trust path run again **from scratch**; the
//!    grid value is kept iff every message emitted at steps `≥ b` is byte-identical
//!    (canonical form: t, bar, instrument, codes, tr, en, trust, shown bar, attachments)
//!    to the observed one and no message is missing or extra. The kept set's maximal runs
//!    of consecutive grid points are its **parts**; a part of `m` points has width
//!    `m · step`. Widths: sum of parts and covering width; ratio = width / spread where
//!    spread = p95 − p5 of the instrument's `|net|`; leaked bits = `max(0, log2(spread /
//!    width_sum))`. A kept set touching the grid edge is *saturated* (the true width is
//!    larger; bits are an upper bound).
//! 2. **Realistic attacker**: sees only messages and public bars. 2,000 candidate flows
//!    are drawn from the generator prior (`qp_source::synthetic_flow` on the same bars,
//!    seeded); each runs through detector + trust path; a candidate is kept for target `b`
//!    iff its messages about bars `b−3 … b+3` (in emission order, canonical form) equal
//!    the observed ones. Posterior = kept candidates' `net_b`: p5–p95 width / spread;
//!    prior width (all candidates) for comparison; coverage of the true value.
//! 3. **Display delay**: `shown_bar − bar` over every message.
//! 4. **Honesty**: `trust == Server`; no digit in `tr`/`en` (and, reported separately,
//!    in codes and attachments); wording: TR contains `sunucu` and (`doğrula` or `teyit`),
//!    EN contains `server` and `verif`.
//! 5. **Utility**: share of `flow.institutional` facts with ≥ 1 message about their bar.
//!
//! Targets: every bar with a `flow.institutional` fact, plus 3 seeded ordinary bars
//! (≥ bar 5, no fact) per instrument.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use qp_capsule::{FlowObservation, QualMessage, TrustPath};
use qp_facts::{to_fact, Detector, FlowPoint, InstitutionalFlow};
use qp_schema::capsule::Trust;
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::common::{
    self, f, fnv, mix, pct_sorted, percentile, ratio, to_json, write_file, DataKind, Lcg,
};
use crate::m3::{raw_location, read_raws, write_summary};
use crate::registry::{self, TrustFactory};

/// Grid points (odd; the middle one is the true value).
pub const GRID: usize = 2001;
/// Realistic attacker candidates.
pub const CANDIDATES: usize = 2000;
/// Realistic attacker window (bars on each side).
pub const WINDOW: u64 = 3;
/// Ordinary target bars per instrument.
pub const ORDINARY: usize = 3;

/// A trust-path factory that may be a closure (tests).
pub type PathMaker<'a> = &'a (dyn Fn() -> Box<dyn TrustPath> + Sync);

/// One message as recorded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MsgRaw {
    /// Instrument index.
    pub inst: u32,
    /// Observe step that emitted it.
    pub step: u32,
    /// Bar it is about.
    pub bar: u64,
    /// Shown bar.
    pub shown_bar: u64,
    /// `trust == Server`.
    pub server: bool,
    /// Codes.
    pub codes: Vec<String>,
    /// TR text.
    pub tr: String,
    /// EN text.
    pub en: String,
    /// Attachments.
    pub attachments: BTreeMap<String, String>,
}

/// Oracle attack result of one target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleRaw {
    /// Instrument index.
    pub inst: u32,
    /// Target bar.
    pub bar: u64,
    /// Has a `flow.institutional` fact.
    pub fact: bool,
    /// True `net`.
    pub truth: f64,
    /// Grid step.
    pub step: f64,
    /// Kept runs `[first k, last k]`.
    pub parts: Vec<[u32; 2]>,
    /// The true grid point was kept (determinism sanity check).
    pub truth_kept: bool,
}

/// Realistic attacker result of one target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealRaw {
    /// Instrument index.
    pub inst: u32,
    /// Target bar.
    pub bar: u64,
    /// Has a fact.
    pub fact: bool,
    /// True net.
    pub truth: f64,
    /// Kept candidates.
    pub kept: u32,
    /// Posterior p5, p95 (kept ≥ 2).
    pub post: Option<[f64; 2]>,
    /// Prior p5, p95.
    pub prior: [f64; 2],
}

/// Raw record of one dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M6Raw {
    /// Dataset.
    pub dataset: String,
    /// Alternative.
    pub alt: String,
    /// Trust path name.
    pub path: String,
    /// Instruments.
    pub instruments: Vec<String>,
    /// p95 − p5 of `|net|` per instrument.
    pub spreads: Vec<f64>,
    /// `[inst, bar]` of every `flow.institutional` fact.
    pub facts: Vec<[u64; 2]>,
    /// Observed messages.
    pub messages: Vec<MsgRaw>,
    /// Oracle attack.
    pub oracle: Vec<OracleRaw>,
    /// Realistic attacker.
    pub realistic: Vec<RealRaw>,
}

/// Canonical byte form of a message.
#[must_use]
pub fn canon(m: &QualMessage) -> String {
    format!(
        "{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}",
        m.t, m.bar, m.instrument, m.codes, m.tr, m.en, m.trust, m.shown_bar, m.attachments
    )
}

struct Stream<'a> {
    symbol: &'a str,
    series: &'a str,
    bars: &'a [Bar],
}

fn observation(s: &Stream<'_>, i: usize, fp: &FlowPoint) -> FlowObservation {
    FlowObservation {
        t: s.bars[i].t,
        bar: i as u64,
        instrument: s.symbol.to_owned(),
        values: fp.values.clone(),
    }
}

/// Run detector + trust path over the stream; `sink(step, msg)` returns `false` to abort.
/// Returns `false` when aborted.
#[allow(clippy::needless_range_loop)]
fn run_path(
    tp: &mut dyn TrustPath,
    s: &Stream<'_>,
    flow: &[FlowPoint],
    obs: &[FlowObservation],
    facts_out: Option<&mut Vec<u64>>,
    mut sink: impl FnMut(usize, QualMessage) -> bool,
) -> bool {
    let mut det = InstitutionalFlow::default().detector();
    let mut fo = facts_out;
    for i in 0..s.bars.len() {
        let ds = det.push(&s.bars[i], flow.get(i));
        let facts: Vec<qp_schema::Fact> = ds
            .iter()
            .map(|d| {
                to_fact(
                    &det,
                    d,
                    s.symbol,
                    s.series,
                    i as u64,
                    s.bars[i].t,
                    qp_capsule::pipeline::ENGINE_VERSION,
                )
            })
            .collect();
        if !facts.is_empty() {
            if let Some(v) = fo.as_deref_mut() {
                v.push(i as u64);
            }
        }
        for m in tp.observe(&obs[i], &facts) {
            if !sink(i, m) {
                return false;
            }
        }
    }
    true
}

fn set_net(fp: &mut FlowPoint, v: f64) {
    let net = fp.values.get("net").copied().unwrap_or(0.0);
    let buy = fp.values.get("buy").copied().unwrap_or(0.0);
    let top5 = fp.values.get("brokers_net_top5").copied().unwrap_or(0.0);
    let buy2 = buy + v.max(0.0) - net.max(0.0);
    fp.values.insert("net".to_owned(), v);
    fp.values.insert("buy".to_owned(), buy2);
    fp.values.insert("sell".to_owned(), buy2 - v);
    let top = if net != 0.0 {
        top5 * (v / net)
    } else {
        v * 1.8
    };
    fp.values.insert("brokers_net_top5".to_owned(), top);
}

fn net_of(fp: &FlowPoint) -> f64 {
    fp.values.get("net").copied().unwrap_or(0.0)
}

/// Grid of target `truth` with fallback scale.
#[must_use]
pub fn grid(truth: f64, fallback: f64) -> (f64, Vec<f64>) {
    let scale = if truth.abs() >= 1.0 {
        truth.abs()
    } else {
        fallback.max(1.0)
    };
    let half = (GRID / 2) as f64;
    let step = 3.0 * scale / half;
    let v = (0..GRID)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let kk = k as f64 - half;
            if kk == 0.0 {
                truth
            } else {
                truth + step * kk
            }
        })
        .collect();
    (step, v)
}

struct InstResult {
    spread: f64,
    facts: Vec<u64>,
    messages: Vec<MsgRaw>,
    oracle: Vec<OracleRaw>,
    realistic: Vec<RealRaw>,
}

#[allow(clippy::too_many_lines)]
fn run_instrument(
    ii: u32,
    inp: &qp_capsule::InstrumentInput,
    seed: u64,
    make: PathMaker<'_>,
    candidates: usize,
) -> InstResult {
    let s = Stream {
        symbol: &inp.symbol,
        series: &inp.series,
        bars: &inp.bars,
    };
    let flow = inp.flow.clone().unwrap_or_default();
    let obs: Vec<FlowObservation> = flow
        .iter()
        .enumerate()
        .map(|(i, p)| observation(&s, i, p))
        .collect();
    let abs: Vec<f64> = flow.iter().map(|p| net_of(p).abs()).collect();
    let spread = percentile(&abs, 0.95).unwrap_or(0.0) - percentile(&abs, 0.05).unwrap_or(0.0);
    let med = percentile(&abs, 0.5).unwrap_or(1.0);

    // Observed run.
    let mut facts = Vec::new();
    let mut observed: Vec<(usize, QualMessage)> = Vec::new();
    {
        let mut tp = make();
        run_path(tp.as_mut(), &s, &flow, &obs, Some(&mut facts), |i, m| {
            observed.push((i, m));
            true
        });
    }
    let canon_obs: Vec<(usize, String)> = observed.iter().map(|(i, m)| (*i, canon(m))).collect();
    let messages: Vec<MsgRaw> = observed
        .iter()
        .map(|(i, m)| MsgRaw {
            inst: ii,
            step: u32::try_from(*i).unwrap_or(0),
            bar: m.bar,
            shown_bar: m.shown_bar,
            server: m.trust == Trust::Server,
            codes: m.codes.clone(),
            tr: m.tr.clone(),
            en: m.en.clone(),
            attachments: m.attachments.clone(),
        })
        .collect();

    // Targets.
    let mut targets: Vec<(u64, bool)> = facts.iter().map(|&b| (b, true)).collect();
    let mut rng = Lcg::new(seed ^ (u64::from(ii) << 32) ^ 0x7A6E);
    let n = s.bars.len();
    let mut picked = 0;
    let mut tries = 0;
    while picked < ORDINARY && n > 6 && tries < 1000 {
        tries += 1;
        let b = 5 + rng.below(n - 5) as u64;
        if targets.iter().any(|t| t.0 == b) {
            continue;
        }
        targets.push((b, false));
        picked += 1;
    }
    targets.sort_unstable();

    // Oracle attack.
    let mut oracle = Vec::new();
    for &(b, is_fact) in &targets {
        let bi = usize::try_from(b).unwrap_or(0);
        let truth = net_of(&flow[bi]);
        let (step, values) = grid(truth, med);
        let start = canon_obs
            .iter()
            .position(|x| x.0 >= bi)
            .unwrap_or(canon_obs.len());
        let mut fl = flow.clone();
        let mut ob = obs.clone();
        let mut kept = vec![false; GRID];
        for (k, &v) in values.iter().enumerate() {
            let mut fp = flow[bi].clone();
            set_net(&mut fp, v);
            ob[bi].values.clone_from(&fp.values);
            fl[bi] = fp;
            let mut tp = make();
            let mut pos = start;
            let ok = run_path(tp.as_mut(), &s, &fl, &ob, None, |i, m| {
                if i < bi {
                    return true;
                }
                if pos >= canon_obs.len() || canon_obs[pos].1 != canon(&m) {
                    return false;
                }
                pos += 1;
                true
            });
            kept[k] = ok && pos == canon_obs.len();
        }
        let mut parts = Vec::new();
        let mut k = 0;
        while k < GRID {
            if kept[k] {
                let st = k;
                while k + 1 < GRID && kept[k + 1] {
                    k += 1;
                }
                parts.push([
                    u32::try_from(st).unwrap_or(0),
                    u32::try_from(k).unwrap_or(0),
                ]);
            }
            k += 1;
        }
        oracle.push(OracleRaw {
            inst: ii,
            bar: b,
            fact: is_fact,
            truth,
            step,
            parts,
            truth_kept: kept[GRID / 2],
        });
    }

    // Realistic attacker.
    let window_sig = |msgs: &[(u64, u64)], b: u64| -> Vec<u64> {
        msgs.iter()
            .filter(|(bar, _)| *bar + WINDOW >= b && *bar <= b + WINDOW)
            .map(|x| x.1)
            .collect()
    };
    let obs_sig: Vec<(u64, u64)> = observed
        .iter()
        .map(|(_, m)| (m.bar, fnv(&canon(m))))
        .collect();
    let mut cand_nets: Vec<Vec<f64>> = vec![Vec::with_capacity(candidates); targets.len()];
    let mut kept_nets: Vec<Vec<f64>> = vec![Vec::new(); targets.len()];
    let target_sigs: Vec<Vec<u64>> = targets.iter().map(|t| window_sig(&obs_sig, t.0)).collect();
    for c in 0..candidates {
        let cseed = mix(seed ^ (u64::from(ii) << 40) ^ (c as u64) ^ 0xCA7D_0000);
        let cf = common::to_flow_points(qp_source::synthetic_flow(s.bars, cseed));
        let cobs: Vec<FlowObservation> = cf
            .iter()
            .enumerate()
            .map(|(i, p)| observation(&s, i, p))
            .collect();
        let mut msgs: Vec<(u64, u64)> = Vec::new();
        let mut tp = make();
        run_path(tp.as_mut(), &s, &cf, &cobs, None, |_, m| {
            msgs.push((m.bar, fnv(&canon(&m))));
            true
        });
        for (ti, t) in targets.iter().enumerate() {
            let v = net_of(&cf[usize::try_from(t.0).unwrap_or(0)]);
            cand_nets[ti].push(v);
            if window_sig(&msgs, t.0) == target_sigs[ti] {
                kept_nets[ti].push(v);
            }
        }
    }
    let realistic = targets
        .iter()
        .enumerate()
        .map(|(ti, &(b, is_fact))| {
            let mut pr = cand_nets[ti].clone();
            pr.sort_by(f64::total_cmp);
            let mut po = kept_nets[ti].clone();
            po.sort_by(f64::total_cmp);
            RealRaw {
                inst: ii,
                bar: b,
                fact: is_fact,
                truth: net_of(&flow[usize::try_from(b).unwrap_or(0)]),
                kept: u32::try_from(po.len()).unwrap_or(0),
                post: (po.len() >= 2).then(|| [pct_sorted(&po, 0.05), pct_sorted(&po, 0.95)]),
                prior: if pr.is_empty() {
                    [0.0, 0.0]
                } else {
                    [pct_sorted(&pr, 0.05), pct_sorted(&pr, 0.95)]
                },
            }
        })
        .collect();

    InstResult {
        spread,
        facts,
        messages,
        oracle,
        realistic,
    }
}

/// Run one synthetic dataset (seed).
#[must_use]
pub fn run_dataset(
    ds: &common::Dataset,
    seed: u64,
    alt: &str,
    make: PathMaker<'_>,
    candidates: usize,
) -> M6Raw {
    let jobs: Vec<(u32, &qp_capsule::InstrumentInput)> = ds
        .inputs
        .iter()
        .enumerate()
        .map(|(i, x)| (u32::try_from(i).unwrap_or(0), x))
        .collect();
    let res = common::par_map(&jobs, |(i, inp)| {
        run_instrument(*i, inp, seed, make, candidates)
    });
    let mut raw = M6Raw {
        dataset: ds.name.clone(),
        alt: alt.to_owned(),
        path: make().name().to_owned(),
        instruments: ds.inputs.iter().map(|x| x.symbol.clone()).collect(),
        spreads: Vec::new(),
        facts: Vec::new(),
        messages: Vec::new(),
        oracle: Vec::new(),
        realistic: Vec::new(),
    };
    for (i, r) in res.into_iter().enumerate() {
        raw.spreads.push(r.spread);
        raw.facts.extend(r.facts.iter().map(|&b| [i as u64, b]));
        raw.messages.extend(r.messages);
        raw.oracle.extend(r.oracle);
        raw.realistic.extend(r.realistic);
    }
    raw
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Distribution of a "smaller is worse" ratio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RatioDist {
    /// Targets.
    pub n: u64,
    /// Median.
    pub median: Option<f64>,
    /// 10th percentile.
    pub p10: Option<f64>,
    /// Worst (minimum).
    pub worst: Option<f64>,
}

impl RatioDist {
    fn of(v: &[f64]) -> Self {
        Self {
            n: v.len() as u64,
            median: percentile(v, 0.5),
            p10: percentile(v, 0.1),
            worst: v.iter().copied().reduce(f64::min),
        }
    }
}

/// Oracle attack scores.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OracleScore {
    /// Width (sum of parts) / spread: all, fact targets, ordinary targets.
    pub width_sum: [RatioDist; 3],
    /// Covering width / spread (all).
    pub width_cover: RatioDist,
    /// Leaked bits (sum of parts): median, p90, max.
    pub bits: [Option<f64>; 3],
    /// Targets with more than one part.
    pub multi_part: u64,
    /// Saturated targets (kept set touches the grid edge).
    pub saturated: u64,
    /// Targets where the true value was not kept (non-determinism).
    pub truth_not_kept: u64,
}

/// Realistic attacker scores.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RealisticScore {
    /// Targets.
    pub targets: u64,
    /// Kept candidates: median, p10, min.
    pub kept: [Option<f64>; 3],
    /// Targets with ≥ 20 kept.
    pub with_20: u64,
    /// Posterior width / spread (targets with ≥ 20 kept).
    pub post: RatioDist,
    /// Prior width / spread.
    pub prior: RatioDist,
    /// True value inside the posterior p5–p95 (targets with ≥ 20 kept).
    pub coverage: Option<f64>,
}

/// Dataset score.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetScore {
    /// Name.
    pub dataset: String,
    /// Messages.
    pub messages: u64,
    /// Facts.
    pub facts: u64,
    /// Oracle.
    pub oracle: OracleScore,
    /// Realistic.
    pub realistic: RealisticScore,
    /// Delay histogram: negative, 0, 1, 2, 3, 4, 5–9, ≥ 10.
    pub delay_hist: Vec<u64>,
    /// Mean, max delay.
    pub delay: [Option<f64>; 2],
    /// Share with trust = server.
    pub server_share: Option<f64>,
    /// Messages with a digit in tr or en.
    pub digit_text: u64,
    /// Messages with a digit in a code.
    pub digit_codes: u64,
    /// Messages with a digit in an attachment.
    pub digit_attach: u64,
    /// TR / EN server-verified wording share.
    pub wording: [Option<f64>; 2],
    /// Facts with ≥ 1 message about their bar / facts.
    pub utility: Option<f64>,
}

/// Summary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M6Summary {
    /// Alternative.
    pub alt: String,
    /// Data.
    pub data: String,
    /// Path name.
    pub path: String,
    /// Per dataset.
    pub datasets: Vec<DatasetScore>,
    /// Pooled over datasets.
    pub pooled: DatasetScore,
}

fn has_digit(s: &str) -> bool {
    s.chars().any(char::is_numeric)
}

/// TR wording check.
#[must_use]
pub fn tr_verified(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("sunucu") && (l.contains("doğrula") || l.contains("teyit"))
}

/// EN wording check.
#[must_use]
pub fn en_verified(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("server") && l.contains("verif")
}

fn score_parts(raws: &[&M6Raw], name: &str) -> DatasetScore {
    let mut ws = [Vec::new(), Vec::new(), Vec::new()];
    let mut wc = Vec::new();
    let mut bits = Vec::new();
    let mut multi = 0;
    let mut sat = 0;
    let mut not_kept = 0;
    let mut kept = Vec::new();
    let mut with20 = 0u64;
    let mut post = Vec::new();
    let mut prior = Vec::new();
    let mut cov = 0u64;
    let mut hist = vec![0u64; 8];
    let mut delays = Vec::new();
    let (mut msgs, mut server, mut dt, mut dc, mut da, mut wtr, mut wen) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut facts = 0u64;
    let mut served = 0u64;
    let mut targets = 0u64;
    for r in raws {
        for o in &r.oracle {
            let spread = r.spreads[o.inst as usize];
            let pts: u64 = o.parts.iter().map(|p| u64::from(p[1] - p[0] + 1)).sum();
            #[allow(clippy::cast_precision_loss)]
            let w_sum = pts as f64 * o.step;
            let w_cov = match (o.parts.first(), o.parts.last()) {
                #[allow(clippy::cast_precision_loss)]
                (Some(a), Some(b)) => f64::from(b[1] - a[0] + 1) * o.step,
                _ => 0.0,
            };
            if spread > 0.0 {
                let rs = w_sum / spread;
                ws[0].push(rs);
                ws[if o.fact { 1 } else { 2 }].push(rs);
                wc.push(w_cov / spread);
                if w_sum > 0.0 {
                    bits.push((spread / w_sum).log2().max(0.0));
                }
            }
            if o.parts.len() > 1 {
                multi += 1;
            }
            if o.parts.first().is_some_and(|p| p[0] == 0)
                || o.parts.last().is_some_and(|p| p[1] as usize == GRID - 1)
            {
                sat += 1;
            }
            if !o.truth_kept {
                not_kept += 1;
            }
        }
        for x in &r.realistic {
            targets += 1;
            let spread = r.spreads[x.inst as usize];
            kept.push(f64::from(x.kept));
            if spread > 0.0 {
                prior.push((x.prior[1] - x.prior[0]) / spread);
            }
            if x.kept >= 20 {
                with20 += 1;
                if let Some(p) = x.post {
                    if spread > 0.0 {
                        post.push((p[1] - p[0]) / spread);
                    }
                    if p[0] <= x.truth && x.truth <= p[1] {
                        cov += 1;
                    }
                }
            }
        }
        for m in &r.messages {
            msgs += 1;
            server += u64::from(m.server);
            dt += u64::from(has_digit(&m.tr) || has_digit(&m.en));
            dc += u64::from(m.codes.iter().any(|c| has_digit(c)));
            da += u64::from(
                m.attachments
                    .iter()
                    .any(|(k, v)| has_digit(k) || has_digit(v)),
            );
            wtr += u64::from(tr_verified(&m.tr));
            wen += u64::from(en_verified(&m.en));
            let d = i128::from(m.shown_bar) - i128::from(m.bar);
            let slot = match d {
                d if d < 0 => 0,
                0..=4 => usize::try_from(d).unwrap_or(0) + 1,
                5..=9 => 6,
                _ => 7,
            };
            hist[slot] += 1;
            #[allow(clippy::cast_precision_loss)]
            delays.push(d as f64);
        }
        for fa in &r.facts {
            facts += 1;
            if r.messages
                .iter()
                .any(|m| u64::from(m.inst) == fa[0] && m.bar == fa[1])
            {
                served += 1;
            }
        }
    }
    let mut sb = bits.clone();
    sb.sort_by(f64::total_cmp);
    DatasetScore {
        dataset: name.to_owned(),
        messages: msgs,
        facts,
        oracle: OracleScore {
            width_sum: [
                RatioDist::of(&ws[0]),
                RatioDist::of(&ws[1]),
                RatioDist::of(&ws[2]),
            ],
            width_cover: RatioDist::of(&wc),
            bits: [
                (!sb.is_empty()).then(|| pct_sorted(&sb, 0.5)),
                (!sb.is_empty()).then(|| pct_sorted(&sb, 0.9)),
                sb.last().copied(),
            ],
            multi_part: multi,
            saturated: sat,
            truth_not_kept: not_kept,
        },
        realistic: RealisticScore {
            targets,
            kept: [
                percentile(&kept, 0.5),
                percentile(&kept, 0.1),
                kept.iter().copied().reduce(f64::min),
            ],
            with_20: with20,
            post: RatioDist::of(&post),
            prior: RatioDist::of(&prior),
            coverage: ratio(cov, with20),
        },
        delay_hist: hist,
        delay: [
            common::mean(&delays),
            delays.iter().copied().reduce(f64::max),
        ],
        server_share: ratio(server, msgs),
        digit_text: dt,
        digit_codes: dc,
        digit_attach: da,
        wording: [ratio(wtr, msgs), ratio(wen, msgs)],
        utility: ratio(served, facts),
    }
}

/// Score.
#[must_use]
pub fn score(alt: &str, data: &str, raws: &[M6Raw]) -> M6Summary {
    let all: Vec<&M6Raw> = raws.iter().collect();
    M6Summary {
        alt: alt.to_owned(),
        data: data.to_owned(),
        path: raws.first().map(|r| r.path.clone()).unwrap_or_default(),
        datasets: raws.iter().map(|r| score_parts(&[r], &r.dataset)).collect(),
        pooled: score_parts(&all, "havuz"),
    }
}

/// Markdown.
#[must_use]
pub fn markdown(s: &M6Summary) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# M6 — `{}` ({})\n", s.alt, s.data);
    let _ = writeln!(
        o,
        "Güven yolu `{}`. Izgara {GRID} nokta (±%300), gerçekçi saldırgan {CANDIDATES} aday, pencere ±{WINDOW} bar.\n",
        s.path
    );
    let rows: Vec<&DatasetScore> = std::iter::once(&s.pooled)
        .chain(s.datasets.iter())
        .collect();
    let _ = writeln!(
        o,
        "## 1. Kâhin eşdeğerlik saldırısı (genişlik / doğal yayılım; küçük = kötü)\n"
    );
    let _ = writeln!(o, "| Veri | Hedef | Medyan | p10 | En kötü | Olgulu medyan | Olgusuz medyan | Kapsayan medyan | Bit medyan | Bit p90 | Bit en çok | Çok parçalı | Doymuş | Gerçek tutulmadı |");
    let _ = writeln!(
        o,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for d in &rows {
        let x = &d.oracle;
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            d.dataset,
            x.width_sum[0].n,
            f(x.width_sum[0].median, 4),
            f(x.width_sum[0].p10, 4),
            f(x.width_sum[0].worst, 6),
            f(x.width_sum[1].median, 4),
            f(x.width_sum[2].median, 4),
            f(x.width_cover.median, 4),
            f(x.bits[0], 2),
            f(x.bits[1], 2),
            f(x.bits[2], 2),
            x.multi_part,
            x.saturated,
            x.truth_not_kept
        );
    }
    let _ = writeln!(
        o,
        "\n## 2. Gerçekçi saldırgan (sonsal p5–p95 genişliği / yayılım)\n"
    );
    let _ = writeln!(o, "| Veri | Hedef | Tutulan aday medyan | p10 | en az | ≥20 tutulan | Sonsal medyan | p10 | en kötü | Önsel medyan | Kapsama |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|---|---|");
    for d in &rows {
        let x = &d.realistic;
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            d.dataset,
            x.targets,
            f(x.kept[0], 0),
            f(x.kept[1], 0),
            f(x.kept[2], 0),
            x.with_20,
            f(x.post.median, 3),
            f(x.post.p10, 3),
            f(x.post.worst, 3),
            f(x.prior.median, 3),
            f(x.coverage, 3)
        );
    }
    let _ = writeln!(o, "\n## 3–5. Gecikme, dürüstlük, fayda\n");
    let _ = writeln!(o, "| Veri | İleti | Gecikme <0 / 0 / 1 / 2 / 3 / 4 / 5–9 / ≥10 | Ort. | En çok | trust=server | Metinde rakam | Kodda rakam | Ekte rakam | TR ifade | EN ifade | Olgu | Fayda |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for d in &rows {
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            d.dataset,
            d.messages,
            d.delay_hist
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(" / "),
            f(d.delay[0], 2),
            f(d.delay[1], 0),
            f(d.server_share, 3),
            d.digit_text,
            d.digit_codes,
            d.digit_attach,
            f(d.wording[0], 3),
            f(d.wording[1], 3),
            d.facts,
            f(d.utility, 3)
        );
    }
    o
}

/// `m6 run` (synthetic only: no real licensed flow exists locally).
///
/// # Errors
/// Unknown alternative, real data requested, I/O.
pub fn cmd_run(alt: &str, data: DataKind, seeds: &[u64]) -> Result<String, String> {
    let fac: TrustFactory = registry::find(&registry::trust_paths(), alt)
        .ok_or(format!("bilinmeyen M6 alternatifi: {alt}"))?;
    if data != DataKind::Synthetic {
        return Err("M6 yalnız sentetik lisanslı akışla koşar (gerçek akış verisi yok)".to_owned());
    }
    let make = move || fac();
    let dir = raw_location("M6", alt, data)?;
    for &seed in seeds {
        let ds = common::synthetic(seed);
        let r = run_dataset(&ds, seed, alt, &make, CANDIDATES);
        write_file(
            &dir.join(common::raw_file_name(data, &r.dataset)),
            &to_json(&r),
        )
        .map_err(|e| e.to_string())?;
    }
    cmd_score(alt, data)
}

/// `m6 score`.
///
/// # Errors
/// Missing raw, I/O.
pub fn cmd_score(alt: &str, data: DataKind) -> Result<String, String> {
    let dir = raw_location("M6", alt, data)?;
    let raws: Vec<M6Raw> = read_raws(&dir, data)?;
    let s = score(alt, data.name(), &raws);
    write_summary("M6", alt, data, &to_json(&s), &markdown(&s))
}
