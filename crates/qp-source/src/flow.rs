//! Synthetic licensed flow feed (R3/T2): per bar, broker flow values of the kind an
//! unentitled user must never see (K33; `docs/CAPSULE.md` §9.7). Deterministic, integer
//! LCG + IEEE basic operations only (see [`crate::regimes`]).
//!
//! [`FlowRecord`] has exactly the fields of `qp_facts::FlowPoint` (`t`, `values`); this
//! crate does not depend on `qp-facts` (the data plane stays below the detectors), so the
//! conversion is the field-by-field move
//! `qp_facts::FlowPoint { t: r.t, values: r.values }`.
//!
//! Values per bar (TRY, rounded to 0.01):
//! - `net`: net buy of the largest broker (by absolute net), signed.
//! - `buy`, `sell`: that broker's buy and sell turnover (`buy - sell = net` up to rounding, both ≥ 0).
//! - `brokers_net_top5`: summed net buy of the five largest net brokers, signed.
//!
//! Model: a persistent institutional state (AR(1)) plus a component correlated with the
//! bar's return; on about 4% of bars an "institutional day" adds a large net position
//! (15–35% of the bar's turnover), aligned with the bar's direction two times out of three.

use std::collections::BTreeMap;

use qp_schema::Bar;

use crate::synthetic::{round2, Lcg};

/// Flow value keys, in the order documented above.
pub const FLOW_KEYS: [&str; 4] = ["net", "buy", "sell", "brokers_net_top5"];

/// One bar's flow values; field-compatible with `qp_facts::FlowPoint`.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowRecord {
    /// Bar time it belongs to (same as the bar's `t`).
    pub t: i64,
    /// Values by key ([`FLOW_KEYS`]).
    pub values: BTreeMap<String, f64>,
}

/// Probability of an institutional day.
const INSTITUTIONAL_PROB: f64 = 0.04;

/// The flow series for `bars` (one record per bar, same `t`), deterministic in `seed`.
#[must_use]
pub fn synthetic_flow(bars: &[Bar], seed: u64) -> Vec<FlowRecord> {
    let mut rng = Lcg(seed ^ 0xF10E_0000_0000_0001);
    // Warm the generator so nearby seeds diverge.
    for _ in 0..4 {
        rng.next_u32();
    }
    let u01 = |r: &mut Lcg| f64::from(r.next_u32()) / 4_294_967_296.0;
    let mut state = 0.0_f64;
    let mut prev_c: Option<f64> = None;
    let mut out = Vec::with_capacity(bars.len());
    for b in bars {
        let turnover = b.v.max(0.0);
        let ret = prev_c.map_or(0.0, |p| if p > 0.0 { b.c / p - 1.0 } else { 0.0 });
        prev_c = Some(b.c);
        // Return in units of a 2% move, clamped.
        let rz = (ret / 0.02).clamp(-3.0, 3.0);
        state = 0.8 * state + 0.2 * rng.unit();
        let mut share = 0.04 * rz + 0.06 * state + 0.03 * rng.unit();
        if u01(&mut rng) < INSTITUTIONAL_PROB {
            let size = 0.15 + 0.2 * u01(&mut rng);
            let aligned = u01(&mut rng) < 2.0 / 3.0;
            let dir = if rz == 0.0 {
                if rng.unit() < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            } else if aligned {
                rz.signum()
            } else {
                -rz.signum()
            };
            share += dir * size;
            state += dir * 0.3;
        }
        let share = share.clamp(-0.6, 0.6);
        let top5 = round2(turnover * share);
        // The largest broker carries 35–75% of the top-5 net.
        let net = round2(top5 * (0.35 + 0.4 * u01(&mut rng)));
        let gross = turnover * (0.03 + 0.09 * u01(&mut rng));
        let buy = round2(gross + net.max(0.0));
        let sell = round2(buy - net);
        let values = BTreeMap::from([
            ("net".to_owned(), net),
            ("buy".to_owned(), buy),
            ("sell".to_owned(), sell),
            ("brokers_net_top5".to_owned(), top5),
        ]);
        out.push(FlowRecord { t: b.t, values });
    }
    out
}
