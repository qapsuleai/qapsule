//! R3/T3 detectors:
//! - batch (`run_batch`) = pure streaming = streaming with seeded live revisions, bit for
//!   bit, for every detector over synthetic series (several seeds);
//! - every detector fires on synthetic data; no look-ahead (a prefix gives the same
//!   detections);
//! - `to_fact` ids are deterministic;
//! - hand-built series with the exact bar each detector must fire on;
//! - a candidate that disappears when the bar is revised.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{
    bar, batch, fresh, from_closes, n_detectors, revised, sig, streamed, test_bars, test_flow,
};
use qp_compute::{run, Bollinger, Field, Rsi, Streaming};
use qp_facts::{
    default_detectors, run_batch, to_fact, BandSqueeze, Detection, Detector, FlowPoint,
    InstitutionalFlow, MaStack, RangeBreakout, RsiDivergence, RsiExtreme, VolumeSurge,
};
use qp_schema::capsule::{Direction, Edge, Geometry, LicenseClass};
use qp_schema::Bar;

const SEEDS: [u64; 3] = [1, 7, 42];

fn codes(dets: &[(u64, Detection)]) -> Vec<(u64, &'static str, Edge)> {
    dets.iter().map(|(i, d)| (*i, d.code, d.edge)).collect()
}

#[test]
fn batch_equals_streaming_equals_revised() {
    for seed in SEEDS {
        let bars = test_bars(2_000, seed);
        let flow = test_flow(&bars, seed);
        for k in 0..n_detectors() {
            let b = batch(fresh(k).as_mut(), &bars, &flow);
            let s = streamed(fresh(k).as_mut(), &bars, &flow);
            let r = revised(fresh(k).as_mut(), &bars, &flow, seed * 31 + k as u64);
            let id = fresh(k).id();
            assert_eq!(sig(&b), sig(&s), "{id} seed {seed}: batch vs streaming");
            assert_eq!(sig(&b), sig(&r), "{id} seed {seed}: batch vs revised");
        }
    }
}

#[test]
fn every_detector_fires_on_synthetic_data() {
    for seed in SEEDS {
        let bars = test_bars(2_000, seed);
        let flow = test_flow(&bars, seed);
        for k in 0..n_detectors() {
            let mut det = fresh(k);
            let d = batch(det.as_mut(), &bars, &flow);
            assert!(!d.is_empty(), "{} seed {seed}: no detection", det.id());
            for (i, x) in &d {
                assert!(
                    usize::try_from(*i).unwrap() >= det.warmup(),
                    "{} fired at {i} inside its warmup {}",
                    det.id(),
                    det.warmup()
                );
                assert!((0.0..=1.0).contains(&x.strength), "{} strength", det.id());
                assert!(x.evidence.values().all(|v| v.is_finite()));
                // Geometry keys point into the evidence.
                for g in &x.geometry {
                    let keys: Vec<&String> = match g {
                        Geometry::Level { key, .. } => vec![key],
                        Geometry::Band { upper, lower, .. } => vec![upper, lower],
                        Geometry::Points { points } => points.iter().map(|p| &p.key).collect(),
                    };
                    assert!(keys.iter().all(|k| x.evidence.contains_key(*k)));
                }
                if let Some(c) = &x.condition {
                    assert!(
                        x.evidence.contains_key(&c.key),
                        "{} condition key",
                        det.id()
                    );
                }
            }
        }
    }
}

#[test]
fn state_edges_alternate() {
    // A state detector never enters twice without an exit (edges, not levels).
    let bars = test_bars(2_000, 3);
    let flow = test_flow(&bars, 3);
    for k in 0..n_detectors() {
        let mut det = fresh(k);
        let d = batch(det.as_mut(), &bars, &flow);
        let mut open: Option<Direction> = None;
        for (i, x) in &d {
            match x.edge {
                Edge::Point => {}
                Edge::Enter => {
                    assert!(open.is_none(), "{} double enter at {i}", det.id());
                    open = Some(x.direction);
                }
                Edge::Exit => {
                    assert_eq!(open, Some(x.direction), "{} orphan exit at {i}", det.id());
                    open = None;
                }
            }
        }
    }
}

#[test]
fn no_look_ahead() {
    let bars = test_bars(1_200, 9);
    let flow = test_flow(&bars, 9);
    for k in 0..n_detectors() {
        let full = batch(fresh(k).as_mut(), &bars, &flow);
        for cut in [150, 401, 777] {
            let part = batch(fresh(k).as_mut(), &bars[..cut], &flow[..cut]);
            let want: Vec<_> = full
                .iter()
                .filter(|(i, _)| *i < cut as u64)
                .cloned()
                .collect();
            assert_eq!(sig(&part), sig(&want), "{} cut {cut}", fresh(k).id());
        }
    }
}

#[test]
fn fact_ids_are_deterministic() {
    let bars = test_bars(800, 5);
    let flow = test_flow(&bars, 5);
    let make = || {
        let mut facts = Vec::new();
        for mut det in default_detectors() {
            for (i, d) in batch(det.as_mut(), &bars, &flow) {
                let t = bars[usize::try_from(i).unwrap()].t;
                facts.push(to_fact(det.as_ref(), &d, "THYAO", "THYAO:1d", i, t, "test"));
            }
        }
        facts
    };
    let a = make();
    let b = make();
    assert_eq!(a, b);
    let ids: BTreeSet<&str> = a.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids.len(), a.len(), "fact ids are unique");
    for f in &a {
        let m = f.meaning.as_ref().unwrap();
        assert_eq!(
            f.id,
            format!(
                "fx.THYAO.{}.{}.{}",
                m.detector,
                &f.kind[m.detector.len() + 1..],
                f.bar
            )
        );
        assert_eq!(m.server, m.detector == "flow.institutional");
        if m.server {
            assert_eq!(m.license.class, LicenseClass::Licensed);
            assert_eq!(m.license.entitlement.as_deref(), Some("flow/*"));
        }
    }
}

#[test]
fn default_detector_order_and_ids() {
    let ids: Vec<_> = default_detectors().iter().map(|d| d.id()).collect();
    assert_eq!(
        ids,
        [
            "continuation.ma_stack",
            "reversal.rsi_divergence",
            "breakout.range",
            "squeeze.bandwidth",
            "extreme.rsi",
            "flow.volume_surge",
            "flow.institutional",
        ]
    );
    assert!(default_detectors().iter().all(|d| d.version() == 1));
}

// --- hand-built series ------------------------------------------------------------

#[test]
fn hand_ma_stack() {
    // Straight line up: close > sma20 > sma50 and sma20 rising as soon as sma50 is warm
    // (bar 49); then a straight line down.
    let mut closes: Vec<f64> = (0..70).map(|i| 100.0 + f64::from(i)).collect();
    let top = closes[69];
    closes.extend((1..=40).map(|i| top - 2.0 * f64::from(i)));
    let bars = from_closes(&closes);
    let d = run_batch(&mut MaStack::default().detector(), &bars, None);
    assert_eq!(d[0].0, 49);
    assert_eq!((d[0].1.code, d[0].1.edge), ("bull", Edge::Enter));
    assert_eq!(d[0].1.evidence["close"], 149.0);
    assert_eq!(d[0].1.evidence["sma20"], 139.5);
    assert_eq!(d[0].1.evidence["sma50"], 124.5);
    assert_eq!(d[0].1.strength, 1.0);
    // Bar 73 is the first close below sma20 (161 < 162): bull ends there.
    assert_eq!(
        (d[1].0, d[1].1.code, d[1].1.edge),
        (73, "bull_end", Edge::Exit)
    );
    assert!(d.iter().skip(2).all(|(_, x)| x.code.starts_with("bear")));
}

#[test]
fn hand_rsi_divergence() {
    let mut c = vec![100.0; 20];
    c.extend((1..=11).map(|i| 100.0 + 2.0 * f64::from(i))); // bars 20..=30 → 122
    c.extend([119.0, 116.0]); // 31, 32
    c.extend([119.0, 117.0, 120.0, 118.0, 121.0, 119.0, 122.0, 124.0]); // 33..=40
    c.extend([121.0, 118.0]); // 41, 42
    let bars = from_closes(&c);
    let d = run_batch(&mut RsiDivergence::default().detector(), &bars, None);
    assert_eq!(codes(&d), [(42, "bearish", Edge::Point)]);
    let x = &d[0].1;
    assert_eq!(x.direction, Direction::Down);
    assert_eq!(x.evidence["p1_bar"], 30.0);
    assert_eq!(x.evidence["p2_bar"], 40.0);
    assert_eq!(x.evidence["p1_price"], 122.5);
    assert_eq!(x.evidence["p2_price"], 124.5);
    // RSI values are the engine's, bit for bit.
    let rsi = run(
        &mut Streaming::new(Rsi::new(14).unwrap(), Field::Close),
        &[&c],
    )
    .unwrap()
    .remove(0);
    assert_eq!(x.evidence["p1_rsi"].to_bits(), rsi[30].unwrap().to_bits());
    assert_eq!(x.evidence["p2_rsi"].to_bits(), rsi[40].unwrap().to_bits());
    assert_eq!(x.evidence["p1_rsi"], 100.0);
}

#[test]
fn hand_breakout_and_candidate() {
    let mut bars: Vec<Bar> = (0..25)
        .map(|i| bar(i, 100.0, 101.0, 99.0, 100.0, 1000.0))
        .collect();
    bars[22] = bar(22, 100.0, 102.5, 99.5, 102.0, 1000.0);
    let d = run_batch(&mut RangeBreakout::default().detector(), &bars, None);
    assert_eq!(codes(&d), [(22, "up", Edge::Point)]);
    let x = &d[0].1;
    assert_eq!(x.evidence["level"], 101.0);
    assert_eq!(x.evidence["close"], 102.0);
    assert_eq!(
        x.geometry,
        [Geometry::Level {
            key: "level".into(),
            from_bar: 2,
            to_bar: 22
        }]
    );
    // ATR(14): every TR is 2 until bar 22 (TR = max(3, 2.5, 0.5) = 3).
    assert_eq!(x.evidence["atr"], (2.0 * 13.0 + 3.0) / 14.0);

    // Candidate: the live version of bar 22 breaks out, the final one does not.
    let mut det = RangeBreakout::default().detector();
    let mut out = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        if i == 22 {
            let live = det.push(b, None);
            assert_eq!(live.len(), 1, "candidate on the open bar");
            let fin = bar(22, 100.0, 101.0, 99.5, 100.5, 1000.0);
            out.extend(det.update_last(&fin, None).into_iter().map(|d| (22u64, d)));
        } else {
            out.extend(det.push(b, None).into_iter().map(|d| (i as u64, d)));
        }
    }
    assert!(out.is_empty(), "the candidate vanished with the final bar");
}

#[test]
fn hand_squeeze() {
    // Alternating closes with a shrinking amplitude: the bandwidth falls every bar, so it
    // is at its 120-bar low the first time that low is defined (bar 19 + 119 = 138).
    // At bar 150 one far close widens the band beyond 1.5x the entry width.
    let mut c: Vec<f64> = (0..150)
        .map(|i| {
            let a = 10.0 - 0.05 * f64::from(i);
            if i % 2 == 0 {
                100.0 + a
            } else {
                100.0 - a
            }
        })
        .collect();
    c.push(130.0);
    c.push(100.0);
    let bars = from_closes(&c);
    let d = run_batch(&mut BandSqueeze::default().detector(), &bars, None);
    assert_eq!(
        codes(&d),
        [
            (138, "squeeze", Edge::Enter),
            (150, "squeeze_end", Edge::Exit)
        ]
    );
    let bb = run(
        &mut Streaming::new(Bollinger::new(20, 2.0).unwrap(), Field::Close),
        &[&c],
    )
    .unwrap();
    let x = &d[0].1;
    assert_eq!(x.evidence["bw"].to_bits(), bb[3][138].unwrap().to_bits());
    assert_eq!(x.evidence["upper"].to_bits(), bb[1][138].unwrap().to_bits());
    assert_eq!(x.evidence["bw_min"], x.evidence["bw"]);
    assert_eq!(x.direction, Direction::Neutral);
    assert_eq!(
        d[1].1.geometry,
        [Geometry::Band {
            upper: "upper".into(),
            lower: "lower".into(),
            from_bar: 119,
            to_bar: 150
        }]
    );
    assert_eq!(d[1].1.evidence["bw_entry"], x.evidence["bw"]);
}

#[test]
fn hand_rsi_extreme() {
    // Flat to bar 14 (RSI 50), up from bar 15 (RSI 100: no loss), crash at bar 20.
    let mut c = vec![100.0; 15];
    c.extend((1..=5).map(|i| 100.0 + f64::from(i)));
    c.push(5.0);
    c.push(5.0);
    let bars = from_closes(&c);
    let d = run_batch(&mut RsiExtreme::default().detector(), &bars, None);
    assert_eq!(
        codes(&d),
        [
            (15, "overbought", Edge::Enter),
            (20, "overbought_end", Edge::Exit),
            (20, "oversold", Edge::Enter)
        ]
    );
    let x = &d[0].1;
    assert_eq!(x.evidence["prev"], 50.0);
    assert_eq!(x.evidence["value"], 100.0);
    assert_eq!(x.evidence["level"], 70.0);
    assert_eq!(x.strength, 1.0);
    assert_eq!(x.direction, Direction::Up);
    assert_eq!(d[2].1.direction, Direction::Down);
}

#[test]
fn hand_volume_surge() {
    let mut bars: Vec<Bar> = (0..25)
        .map(|i| bar(i, 100.0, 101.0, 99.0, 100.0, 1000.0))
        .collect();
    bars[22] = bar(22, 100.0, 101.0, 99.0, 100.5, 3000.0);
    bars[23] = bar(23, 100.0, 101.0, 99.0, 99.0, 2500.0); // 2500 / avg(1100) < 2.5
    let d = run_batch(&mut VolumeSurge::default().detector(), &bars, None);
    assert_eq!(codes(&d), [(22, "surge", Edge::Point)]);
    let x = &d[0].1;
    assert_eq!(x.direction, Direction::Up);
    assert_eq!(x.evidence["vol_avg"], 1000.0);
    assert_eq!(x.evidence["ratio"], 3.0);
    assert_eq!(x.strength, (3.0 - 2.5) / 2.5);
}

#[test]
fn hand_institutional_flow() {
    let nets = [
        Some(10.0),
        Some(-10.0),
        Some(10.0),
        None,
        Some(-10.0),
        Some(10.0),
        Some(25.0),
        Some(21.0),
        Some(-30.0),
    ];
    let bars: Vec<Bar> = (0..nets.len())
        .map(|i| bar(i, 100.0, 101.0, 99.0, 100.0, 1000.0))
        .collect();
    let flow: Vec<Option<FlowPoint>> = nets
        .iter()
        .zip(&bars)
        .map(|(n, b)| {
            n.map(|n| FlowPoint {
                t: b.t,
                values: BTreeMap::from([("net".to_owned(), n)]),
            })
        })
        .collect();
    let mut det = InstitutionalFlow::default().detector();
    let d = streamed(&mut det, &bars, &flow);
    // Bar 6: window {10,10,10,10,10} (bar 3 has no point) → avg 10, 25 > 20.
    // Bar 7: window {10,10,10,10,25} → avg 13, 21 < 26. Bar 8: {10,10,10,25,21} → 15.2,
    // 30 < 30.4.
    assert_eq!(codes(&d), [(6, "net_buy", Edge::Point)]);
    assert_eq!(d[0].1.evidence["net"], 25.0);
    assert_eq!(d[0].1.evidence["avg5"], 10.0);
    assert_eq!(d[0].1.direction, Direction::Up);
    assert_eq!(det.license().class, LicenseClass::Licensed);
}
