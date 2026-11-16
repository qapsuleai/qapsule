//! K42 probes: directional ledger key recomputed independently; look-ahead by corrupting
//! the bars after a cutoff.
use std::collections::BTreeMap;

use qp_capsule::engine::{direction_name, EngineConfig};
use qp_capsule::{simulate, InstrumentInput};
use qp_facts::default_detectors;
use qp_schema::capsule::Stage;

fn inputs(seed: u64) -> Vec<InstrumentInput> {
    qp_source::synthetic_market(10, 420, seed)
        .into_iter()
        .map(|s| InstrumentInput { symbol: s.symbol.clone(), series: format!("{}.1d", s.symbol.to_lowercase()), bars: s.bars, flow: None })
        .collect()
}

fn main() {
    let inp = inputs(11);
    let sim = simulate(&inp, &default_detectors, EngineConfig::r3());
    // closes: (ledger_key, close t, realized)
    let closes: Vec<(String, i64, bool, String, String)> = sim.events.iter()
        .filter(|e| matches!(e.transition.stage, Stage::Realized | Stage::Invalidated | Stage::Expired))
        .map(|e| (e.story.ledger_key.clone(), e.transition.t, e.transition.stage == Stage::Realized, e.story.kind.name().to_owned(), direction_name(e.story.direction).to_owned()))
        .collect();
    let births: Vec<_> = sim.events.iter().filter(|e| e.transition.stage == Stage::Born).collect();
    let (mut key_ok, mut key_bad, mut stat_ok, mut stat_bad, mut undirected_differs) = (0, 0, 0, 0, 0);
    let mut keys = BTreeMap::<String, usize>::new();
    for b in &births {
        let s = &b.story;
        let prefix = format!("{}/{}/", s.kind.name(), direction_name(s.direction));
        if s.ledger_key.starts_with(&prefix) { key_ok += 1 } else { key_bad += 1; eprintln!("bad key {} for {prefix}", s.ledger_key) }
        *keys.entry(s.ledger_key.clone()).or_default() += 1;
        let det = s.ledger_key.rsplit('/').next().unwrap();
        let (mut n, mut r, mut n_any, mut r_any) = (0u32, 0u32, 0u32, 0u32);
        for (k, t, real, kind, _dir) in &closes {
            if *t < b.transition.t {
                if *k == s.ledger_key { n += 1; if *real { r += 1 } }
                if kind == s.kind.name() && k.ends_with(det) { n_any += 1; if *real { r_any += 1 } }
            }
        }
        if s.ledger_at_birth.n == n && s.ledger_at_birth.realized == r { stat_ok += 1 } else { stat_bad += 1; if stat_bad < 4 { eprintln!("stat {} at {}: engine {:?} recomputed n={n} r={r}", s.ledger_key, b.transition.t, s.ledger_at_birth) } }
        if (n_any, r_any) != (n, r) { undirected_differs += 1 }
    }
    println!("A23 births {} key prefix ok {key_ok} bad {key_bad}; ledger_at_birth recomputed from same-key closes strictly before birth: ok {stat_ok} bad {stat_bad}; births where an undirected count would differ: {undirected_differs}", births.len());
    println!("A23 distinct keys: {}", keys.keys().cloned().collect::<Vec<_>>().join(", "));

    // Look-ahead: corrupt every bar after the cutoff index, rerun.
    let cut = 300usize;
    let t_cut = inp[0].bars[cut].t;
    let mut bad_inp = inp.clone();
    for (k, x) in bad_inp.iter_mut().enumerate() {
        let base = x.bars[cut].c;
        for (i, b) in x.bars.iter_mut().enumerate().skip(cut + 1) {
            // mirror the path and add a deterministic zig-zag
            let f = |p: f64| (2.0 * base - p).max(base * 0.2) * (1.0 + 0.03 * (((i + k) % 7) as f64 - 3.0) / 3.0);
            let (o, c) = (f(b.o), f(b.c));
            let (h, l) = (f(b.l).max(o).max(c), f(b.h).min(o).min(c));
            *b = qp_schema::Bar { t: b.t, o, h: h.max(l), l: l.min(h), c, v: b.v * 1.7 };
        }
    }
    let sim2 = simulate(&bad_inp, &default_detectors, EngineConfig::r3());
    let before = |s: &qp_capsule::SimResult| s.events.iter().filter(|e| e.transition.t <= t_cut).map(|e| serde_json_string(e)).collect::<Vec<_>>();
    let (a, b) = (before(&sim), before(&sim2));
    let after_differs = sim.events.len() != sim2.events.len() || sim.events.iter().zip(&sim2.events).any(|(x, y)| serde_json_string(x) != serde_json_string(y));
    println!("look-ahead: events up to cutoff (bar {cut}): {} vs {} equal={} ; whole run differs after cutoff={after_differs}", a.len(), b.len(), a == b);
    // ledger stats as of the cutoff for every key equal
    let mut eq = 0; let mut ne = 0;
    for k in keys.keys() {
        if sim.ledger_stat(k, t_cut + 1) == sim2.ledger_stat(k, t_cut + 1) { eq += 1 } else { ne += 1 }
    }
    println!("look-ahead: ledger_stat(key, t_cut] equal for {eq} keys, differ {ne}");
    // births at/before cutoff: ledger_at_birth and importance equal
    let pick = |s: &qp_capsule::SimResult| s.events.iter().filter(|e| e.transition.stage == Stage::Born && e.transition.t <= t_cut).map(|e| (e.capsule.clone(), e.story.ledger_at_birth, e.story.importance.to_bits(), e.story.tier)).collect::<Vec<_>>();
    println!("look-ahead: births <= cutoff with ledger_at_birth/importance/tier equal: {}", pick(&sim) == pick(&sim2));
}

fn serde_json_string<T: std::fmt::Debug>(x: &T) -> String { format!("{x:?}") }
