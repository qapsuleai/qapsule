#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_facts::{default_detectors, Detection, Detector, FlowPoint};
use qp_schema::Bar;

/// Seeded LCG (test noise only; detectors never see randomness).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let x = self.next() as f64;
        x / 8_589_934_592.0
    }
}

/// A fresh detector by its index in `default_detectors()`.
pub fn fresh(k: usize) -> Box<dyn Detector> {
    default_detectors().swap_remove(k)
}

pub fn n_detectors() -> usize {
    default_detectors().len()
}

/// Synthetic bars plus deterministic volume spikes (the generator's volume stays within
/// 0.5–2.5 M, so it never surges 2.5× its own average).
pub fn test_bars(n: usize, seed: u64) -> Vec<Bar> {
    let mut bars = qp_source::synthetic_bars(n, seed);
    let mut r = Lcg(seed ^ 0x5eed);
    for b in &mut bars {
        if r.next().is_multiple_of(53) {
            b.v *= 5.0;
        }
    }
    bars
}

/// A synthetic flow series aligned with `bars`: `net` noise, occasional spikes, and
/// gaps (missing points, a point without `net`).
pub fn test_flow(bars: &[Bar], seed: u64) -> Vec<Option<FlowPoint>> {
    let mut r = Lcg(seed ^ 0xf10e);
    bars.iter()
        .map(|b| {
            let k = r.next() % 100;
            if k < 5 {
                return None;
            }
            let mut values = BTreeMap::new();
            if k >= 7 {
                let mut net = (r.unit() - 0.5) * 2.0e6;
                if k >= 93 {
                    net *= 8.0;
                }
                values.insert("net".to_owned(), net.round());
            }
            values.insert("buy".to_owned(), 1.0);
            Some(FlowPoint { t: b.t, values })
        })
        .collect()
}

/// Bit-exact fingerprint of a detection list.
pub fn sig(dets: &[(u64, Detection)]) -> Vec<String> {
    dets.iter()
        .map(|(i, d)| {
            let ev: Vec<String> = d
                .evidence
                .iter()
                .map(|(k, v)| format!("{k}={:016x}", v.to_bits()))
                .collect();
            format!(
                "{i} {} {:?} {:?} {:?} {:?} s={:016x} [{}] {:?} {:?}",
                d.code,
                d.kind,
                d.direction,
                d.time,
                d.edge,
                d.strength.to_bits(),
                ev.join(","),
                d.geometry,
                d.condition
            )
        })
        .collect()
}

pub fn flow_ref(flow: &[Option<FlowPoint>], i: usize) -> Option<&FlowPoint> {
    flow.get(i).and_then(Option::as_ref)
}

/// Pure streaming: one `push` per bar.
pub fn streamed(
    det: &mut dyn Detector,
    bars: &[Bar],
    flow: &[Option<FlowPoint>],
) -> Vec<(u64, Detection)> {
    let mut out = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        for d in det.push(b, flow_ref(flow, i)) {
            out.push((i as u64, d));
        }
    }
    out
}

/// Batch over an `Option` flow series (`run_batch` takes a dense slice; a missing point
/// is a point without values, which every detector treats as "no observation").
pub fn batch(
    det: &mut dyn Detector,
    bars: &[Bar],
    flow: &[Option<FlowPoint>],
) -> Vec<(u64, Detection)> {
    let dense: Vec<FlowPoint> = bars
        .iter()
        .enumerate()
        .map(|(i, b)| {
            flow_ref(flow, i).cloned().unwrap_or(FlowPoint {
                t: b.t,
                values: BTreeMap::new(),
            })
        })
        .collect();
    qp_facts::run_batch(det, bars, Some(&dense))
}

/// A noisy intermediate version of a bar (and of its flow point).
pub fn wobble(b: &Bar, f: Option<&FlowPoint>, r: &mut Lcg) -> (Bar, Option<FlowPoint>) {
    let k = 1.0 + (r.unit() - 0.5) * 0.08;
    let c = b.c * k;
    let mut nb = b.clone();
    nb.c = c;
    nb.h = if c > b.h {
        c
    } else {
        b.h * (1.0 + r.unit() * 0.03)
    };
    nb.l = if c < b.l {
        c
    } else {
        b.l * (1.0 - r.unit() * 0.03)
    };
    nb.v = b.v * (r.unit() * 3.0);
    let nf = match (f, r.next() % 4) {
        (_, 0) => None,
        (Some(f), _) => {
            let mut f = f.clone();
            for v in f.values.values_mut() {
                *v *= (r.unit() - 0.3) * 10.0;
            }
            Some(f)
        }
        (None, _) => None,
    };
    (nb, nf)
}

/// Streaming with live revisions: each bar first arrives as a noisy version, is revised
/// 0–3 more times with noise, then revised to its final value; the final call's
/// detections are the bar's.
pub fn revised(
    det: &mut dyn Detector,
    bars: &[Bar],
    flow: &[Option<FlowPoint>],
    seed: u64,
) -> Vec<(u64, Detection)> {
    let mut r = Lcg(seed);
    let mut out = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        let f = flow_ref(flow, i);
        let (nb, nf) = wobble(b, f, &mut r);
        det.push(&nb, nf.as_ref());
        for _ in 0..(r.next() % 4) {
            let (nb, nf) = wobble(b, f, &mut r);
            det.update_last(&nb, nf.as_ref());
        }
        for d in det.update_last(b, f) {
            out.push((i as u64, d));
        }
    }
    out
}

/// Simple bar.
pub fn bar(i: usize, o: f64, h: f64, l: f64, c: f64, v: f64) -> Bar {
    Bar {
        t: 1_700_000_000_000 + i64::try_from(i).unwrap() * 86_400_000,
        o,
        h,
        l,
        c,
        v,
    }
}

/// Bars from closes: `o = c`, `h = c + 0.5`, `l = c - 0.5`, `v = 1000`.
pub fn from_closes(closes: &[f64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, &c)| bar(i, c, c + 0.5, c - 0.5, c, 1000.0))
        .collect()
}
