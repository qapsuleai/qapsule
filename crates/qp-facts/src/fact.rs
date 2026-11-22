//! Detections to facts ([`run_batch`], [`fact_id`], [`to_fact`]) and the small value
//! helpers the detectors share (out of `lib.rs`).

use std::collections::BTreeMap;

use qp_schema::capsule::FactMeaning;
use qp_schema::{Author, Bar, Fact, Provenance};

use crate::{Detection, Detector, FlowPoint};

/// Evidence map from `(key, value)` pairs.
pub(crate) fn evidence(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

/// `x` clipped to `[0, 1]`; `NaN` → 0.
pub(crate) fn clip01(x: f64) -> f64 {
    if x > 1.0 {
        1.0
    } else if x > 0.0 {
        x
    } else {
        0.0
    }
}

/// Bar index as `f64` (evidence); exact below 2^53.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn bar_f64(i: u64) -> f64 {
    i as f64
}

/// Run a detector over all bars: `(bar index, detection)` in bar order. Defined as
/// streaming every bar, so batch = streaming by construction.
pub fn run_batch(
    det: &mut dyn Detector,
    bars: &[Bar],
    flow: Option<&[FlowPoint]>,
) -> Vec<(u64, Detection)> {
    let mut out = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        let f = flow.and_then(|fl| fl.get(i));
        for d in det.push(b, f) {
            out.push((i as u64, d));
        }
    }
    out
}

/// Deterministic fact id: `fx.<instrument>.<detector>.<code>.<bar>`.
#[must_use]
pub fn fact_id(instrument: &str, detector: &str, code: &str, bar: u64) -> String {
    format!("fx.{instrument}.{detector}.{code}.{bar}")
}

/// Build the typed fact for a detection. `series` is the bar series id; `t` the bar time;
/// `engine_version` goes into the provenance. The `evidence_ref` is left empty: the
/// authority stamps it (RP2 §18.5).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn to_fact(
    det: &dyn Detector,
    d: &Detection,
    instrument: &str,
    series: &str,
    bar: u64,
    t: i64,
    engine_version: &str,
) -> Fact {
    let license = det.license();
    let server = matches!(license.class, qp_schema::capsule::LicenseClass::Licensed);
    Fact {
        id: fact_id(instrument, det.id(), d.code, bar),
        kind: format!("{}.{}", det.id(), d.code),
        series: series.to_owned(),
        bar,
        t,
        text: format!("{} {} at bar {bar}", det.id(), d.code),
        evidence: d.evidence.clone(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind: d.kind,
            direction: d.direction,
            family: det.family(),
            time: d.time,
            edge: d.edge,
            detector: det.id().to_owned(),
            detector_version: det.version(),
            instrument: instrument.to_owned(),
            strength: d.strength,
            license,
            geometry: d.geometry.clone(),
            condition: d.condition.clone(),
            server,
        }),
        provenance: Provenance {
            author: Author::Engine {
                name: "qp-facts".to_owned(),
                version: engine_version.to_owned(),
            },
            at: t,
            based_on: vec![series.to_owned()],
            approved_by: None,
        },
    }
}
