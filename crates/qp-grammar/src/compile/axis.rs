//! A panel's value domain and its axis ticks.

use super::*;

/// Automatic or fixed domain of a panel scale (`docs/GRAMMAR.md` §3).
pub(super) fn domain(scale: &Scale, values: &[f64], i: usize) -> Result<(f64, f64), GrammarError> {
    let log = scale.kind == ScaleKind::Log;
    if let Some([a, b]) = scale.domain {
        if !(a.is_finite() && b.is_finite() && a < b && (b - a).is_finite()) || (log && a <= 0.0) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/y/domain"),
                format!(
                    "domain [{a}, {b}] is invalid (need min < max{})",
                    if log { ", min > 0 on a log scale" } else { "" }
                ),
            ));
        }
        return Ok((a, b));
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        if log && *v <= 0.0 {
            continue;
        }
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if lo > hi {
        return Ok(if log { (1.0, 10.0) } else { (0.0, 1.0) });
    }
    if log {
        let (llo, lhi) = (
            detmath::log10(lo).unwrap_or(0.0),
            detmath::log10(hi).unwrap_or(0.0),
        );
        let span = (lhi - llo).max(1e-9);
        Ok((
            detmath::pow10(llo - span * 0.05),
            detmath::pow10(lhi + span * 0.05),
        ))
    } else {
        // Spans that overflow (values near ±f64::MAX) are clamped so the domain stays
        // finite; ordinary data takes R0's exact formula.
        let span = (hi - lo).clamp(1e-9, f64::MAX);
        let (a, b) = (
            (lo - span * 0.05).max(-f64::MAX),
            (hi + span * 0.05).min(f64::MAX),
        );
        if a < b && (b - a).is_finite() {
            return Ok((a, b));
        }
        let (a, b) = (a.max(-f64::MAX / 2.0), b.min(f64::MAX / 2.0));
        Ok(if a < b { (a, b) } else { (0.0, 1.0) })
    }
}

/// Values a prepared layer contributes to its panel's automatic domain.
pub(super) fn domain_values(pl: &Prepared, out: &mut Vec<f64>) {
    if pl.geom == Geom::RuleV {
        return;
    }
    for t in &pl.traces {
        for r in &t.rows {
            out.extend(r.v.iter().flatten().copied());
        }
    }
}

/// Grid values of a panel: linear nice steps (R0), or 1-2-5 × 10^k on a log scale.
pub(super) fn ticks(y: &YMap, divs: f64) -> Vec<f64> {
    let linear = |out: &mut Vec<f64>| {
        // Bounded integer loop (security review): `k += 1.0` stops advancing once
        // |k| >= 2^53, so the tick index is an integer with a hard cap.
        let step = nice_step((y.max - y.min) / divs);
        if !(step.is_finite() && step > 0.0) {
            return;
        }
        let k0 = (y.min / step).ceil();
        if !k0.is_finite() || k0.abs() >= MAX_EXACT_TICK {
            return;
        }
        for i in 0..MAX_TICKS {
            let v = (k0 + f64::from(i)) * step;
            if v > y.max {
                break;
            }
            out.push(v);
        }
    };
    let mut out = Vec::new();
    if y.kind != ScaleKind::Log {
        linear(&mut out);
        return out;
    }
    let (Some(a), Some(b)) = (detmath::log10(y.min), detmath::log10(y.max)) else {
        return out;
    };
    #[allow(clippy::cast_possible_truncation)]
    let (e0, e1) = (a.floor() as i32, b.ceil() as i32);
    let mut all = Vec::new();
    let mut ones = Vec::new();
    for e in e0..=e1 {
        let p = detmath::pow10_int(e);
        for m in [1.0, 2.0, 5.0] {
            let v = m * p;
            if v >= y.min && v <= y.max {
                all.push(v);
                if m == 1.0 {
                    ones.push(v);
                }
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    if all.len() < 2 {
        linear(&mut out);
    } else if all.len() as f64 > 2.0 * divs && ones.len() >= 2 {
        out = ones;
    } else {
        out = all;
    }
    out
}
