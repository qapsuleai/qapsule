//! Properties of the mühür path: honesty, determinism, the by-construction leakage bound,
//! receipts and entitled replay.

use std::collections::BTreeMap;

use m6_muhur::{
    replay, trust_path, verify_receipts, Cell, MuhurPath, Params, DEMO_RECEIPT_KEY, KEY_NET,
};
use qp_capsule::{FlowObservation, QualMessage, TrustPath};
use qp_schema::capsule::Trust;

fn canon(m: &QualMessage) -> String {
    format!(
        "{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}",
        m.t, m.bar, m.instrument, m.codes, m.tr, m.en, m.trust, m.shown_bar, m.attachments
    )
}

/// One synthetic instrument's observations.
fn stream(seed: u64, inst: usize) -> Vec<FlowObservation> {
    let market = qp_source::synthetic_market(inst + 1, 300, seed);
    let i = &market[inst];
    let flow = qp_source::synthetic_flow(&i.bars, seed.wrapping_mul(31).wrapping_add(inst as u64));
    i.bars
        .iter()
        .zip(flow)
        .enumerate()
        .map(|(k, (b, f))| FlowObservation {
            t: b.t,
            bar: k as u64,
            instrument: i.symbol.clone(),
            values: f.values,
        })
        .collect()
}

fn transcript(obs: &[FlowObservation]) -> Vec<String> {
    let mut p = trust_path();
    obs.iter()
        .flat_map(|o| p.observe(o, &[]))
        .map(|m| canon(&m))
        .collect()
}

fn with_net(obs: &[FlowObservation], b: usize, v: f64) -> Vec<FlowObservation> {
    let mut o = obs.to_vec();
    o[b].values.insert(KEY_NET.to_owned(), v);
    o
}

fn messages(obs: &[FlowObservation]) -> Vec<QualMessage> {
    let mut p = trust_path();
    obs.iter().flat_map(|o| p.observe(o, &[])).collect()
}

#[test]
fn honest_and_digit_free() {
    let mut n = 0;
    for seed in [1u64, 2] {
        for inst in 0..4 {
            for m in messages(&stream(seed, inst)) {
                n += 1;
                assert_eq!(m.trust, Trust::Server);
                assert_eq!(m.shown_bar, m.bar);
                for s in [&m.tr, &m.en] {
                    assert!(!s.chars().any(char::is_numeric), "digit in {s}");
                }
                for s in m
                    .codes
                    .iter()
                    .chain(m.attachments.keys())
                    .chain(m.attachments.values())
                {
                    assert!(!s.chars().any(char::is_numeric), "digit in {s}");
                }
                let tr = m.tr.to_lowercase();
                assert!(tr.contains("sunucu") && (tr.contains("doğrula") || tr.contains("teyit")));
                let en = m.en.to_lowercase();
                assert!(en.contains("server") && en.contains("verif"));
                // K34: says which entitlement the evidence needs and what it rests on.
                assert_eq!(
                    m.attachments.get("entitlement").map(String::as_str),
                    Some("flow/*")
                );
                assert!(m.attachments.contains_key("evidence"));
            }
        }
    }
    assert!(n > 50, "too few messages: {n}");
}

#[test]
fn deterministic_and_fresh() {
    let obs = stream(3, 1);
    assert_eq!(transcript(&obs), transcript(&obs));
    // A fresh path per call: running one stream does not change the next.
    let mut a = trust_path();
    for o in &stream(4, 0) {
        let _ = a.observe(o, &[]);
    }
    assert_eq!(transcript(&obs), transcript(&obs));
}

#[test]
fn lattice_is_exact() {
    for k in -20..40 {
        let x = 2f64.powi(k);
        assert_eq!(Cell::of(x).level, k);
        assert_eq!(Cell::of(-x).level, k);
        assert_eq!(Cell::of(f64::from_bits(x.to_bits() - 1)).level, k - 1);
        assert_eq!(Cell::of(f64::from_bits((2.0 * x).to_bits() - 1)).level, k);
    }
    assert!(Cell::of(3.0).up && !Cell::of(-3.0).up && !Cell::of(0.0).up);
}

/// The guaranteed indistinguishability set of target `b` (NOTES.md §3): values that the
/// construction maps to the same stored cell and the same statement.
fn guaranteed(obs: &[FlowObservation], b: usize) -> Vec<f64> {
    let p = Params::default();
    let mut path = MuhurPath::new(p);
    for o in &obs[..b] {
        let _ = path.observe(o, &[]);
    }
    // Reference that bar b will use (bar-epoch = 1: recomputed on b).
    let mut probe = path.clone();
    let _ = probe.observe(&obs[b], &[]);
    let r = probe.reference();
    let v = obs[b].values[KEY_NET];
    let c = Cell::of(v);
    let pts = |lo: f64, hi: f64, sign: f64| -> Vec<f64> {
        (0..=16)
            .map(|i| sign * (lo + (hi - lo) * f64::from(i) / 16.0))
            .map(|x| {
                if x.abs() >= hi {
                    sign * f64::from_bits(hi.to_bits() - 1)
                } else {
                    x
                }
            })
            .collect()
    };
    match r {
        Some(r) if c.level >= r + p.loud => {
            let lo = 2f64.powi(r + p.loud);
            pts(lo, 64.0 * lo, if c.up { 1.0 } else { -1.0 })
        }
        Some(r) if c.level < r - p.low => {
            let hi = 2f64.powi(r - p.low);
            let mut a = pts(0.0, hi, 1.0);
            a.extend(pts(0.0, hi, -1.0));
            a
        }
        Some(_) => {
            // Mid cell: both signs (the sign of a silent bar is never used).
            let lo = 2f64.powi(c.level);
            let mut a = pts(lo, 2.0 * lo, 1.0);
            a.extend(pts(lo, 2.0 * lo, -1.0));
            a
        }
        None => {
            // Warm-up: own cell, both signs.
            if c.level == m6_muhur::ZERO_LEVEL {
                return vec![0.0];
            }
            let lo = 2f64.powi(c.level);
            let mut a = pts(lo, 2.0 * lo, 1.0);
            a.extend(pts(lo, 2.0 * lo, -1.0));
            a
        }
    }
}

#[test]
fn leakage_bound_by_construction() {
    // Every value in the guaranteed set gives a byte-identical transcript (from the
    // target bar on, and indeed the whole run).
    let mut checked = 0;
    for seed in [5u64, 6] {
        for inst in 0..3 {
            let obs = stream(seed, inst);
            let base = transcript(&obs);
            for b in (2..obs.len()).step_by(7) {
                for v in guaranteed(&obs, b) {
                    assert_eq!(
                        transcript(&with_net(&obs, b, v)),
                        base,
                        "seed {seed} inst {inst} bar {b} value {v}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 1000);
}

#[test]
fn reads_only_net() {
    let obs = stream(7, 2);
    let base = transcript(&obs);
    let mut o = obs.clone();
    for x in &mut o {
        for k in ["buy", "sell", "brokers_net_top5"] {
            x.values.insert(k.to_owned(), 1.0e9);
        }
    }
    assert_eq!(transcript(&o), base);
    // A bar without `net` is no observation.
    let mut o = obs.clone();
    o[40].values.remove(KEY_NET);
    let mut p = trust_path();
    let none = p.observe(&o[40], &[]);
    assert!(none.is_empty());
}

#[test]
fn receipts_verify_and_detect_tampering() {
    let msgs = messages(&stream(8, 0));
    assert!(msgs.len() > 5);
    assert_eq!(verify_receipts(&msgs, DEMO_RECEIPT_KEY), None);
    // Wrong key.
    assert_eq!(verify_receipts(&msgs, b"other"), Some(0));
    // Flip direction of one message.
    let mut t = msgs.clone();
    let i = 3;
    let flip: Vec<String> = t[i]
        .codes
        .iter()
        .map(|c| match c.as_str() {
            "net_buy" => "net_sell".to_owned(),
            "net_sell" => "net_buy".to_owned(),
            x => x.to_owned(),
        })
        .collect();
    t[i].codes = flip;
    assert_eq!(verify_receipts(&t, DEMO_RECEIPT_KEY), Some(i));
    // Moving a message to another bar is detected (the bar is in the statement).
    let mut d = msgs.clone();
    d[2].bar += 1;
    assert_eq!(verify_receipts(&d, DEMO_RECEIPT_KEY), Some(2));
    // Dropping a message is not visible to the receipt check; the entitled replay
    // catches it (`entitled_replay_reproduces_and_commits`).
    // Edited text.
    let mut e = msgs;
    e[1].en.push('!');
    assert_eq!(verify_receipts(&e, DEMO_RECEIPT_KEY), Some(1));
}

#[test]
fn entitled_replay_reproduces_and_commits() {
    let obs = stream(9, 1);
    let mut p = MuhurPath::new(Params::default());
    let served: Vec<QualMessage> = obs.iter().flat_map(|o| p.observe(o, &[])).collect();
    let (again, log) = replay(&obs, Params::default());
    assert_eq!(served, again);
    assert_eq!(log, p.audit_log());
    // The audit log commits to the raw value: a different raw value in the same cell
    // gives the same user transcript but a different commitment.
    let b = 60;
    let v = obs[b].values[KEY_NET];
    let v2 = f64::from_bits(v.to_bits() ^ 1);
    assert_eq!(Cell::of(v), Cell::of(v2));
    let (msgs2, log2) = replay(&with_net(&obs, b, v2), Params::default());
    assert_eq!(msgs2, served);
    let at = |l: &[m6_muhur::AuditEntry]| {
        l.iter()
            .find(|e| e.bar == b as u64)
            .map(|e| e.evidence_commitment.clone())
    };
    assert_ne!(at(&log), at(&log2));
    // Commitment text is letters only and never in a user message.
    let c = at(&log).unwrap_or_default();
    assert!(c.chars().all(|x| x.is_ascii_lowercase() || x == '-'));
    let all: BTreeMap<String, String> = served.iter().flat_map(|m| m.attachments.clone()).collect();
    assert!(!all.values().any(|x| *x == c));
}

#[test]
fn at_most_low_plus_loud_plus_three_transcripts() {
    // Information bound: after warm-up, over every value of the target's `net` the
    // whole transcript takes at most `low + loud + 3` distinct forms (2 loud signs, one
    // merged low cell, `low + loud` mid levels), i.e. ≤ log2(7) ≈ 2.81 bits for the
    // default parameters, whatever the attacker knows.
    let p = Params::default();
    let bound = usize::try_from(p.low + p.loud + 3).unwrap_or(0);
    let mut sweep = vec![0.0];
    for k in -60..=160 {
        let x = 1.5f64.powi(k) * 1.0e-3;
        sweep.push(x);
        sweep.push(-x);
    }
    let mut worst = 0;
    for seed in [10u64, 11] {
        for inst in 0..2 {
            let obs = stream(seed, inst);
            for b in (p.min_obs..obs.len()).step_by(23) {
                let mut seen = std::collections::BTreeSet::new();
                for &v in &sweep {
                    seen.insert(transcript(&with_net(&obs, b, v)).join("\n"));
                }
                assert!(seen.len() <= bound, "bar {b}: {} transcripts", seen.len());
                worst = worst.max(seen.len());
            }
        }
    }
    assert!(worst >= 3, "sweep too narrow: {worst}");
}
