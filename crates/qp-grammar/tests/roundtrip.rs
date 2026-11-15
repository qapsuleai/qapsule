//! Data → pixel → data round trip (criterion 7): for every item, the centre of its hit
//! box maps back through the scene's `XMap`/`YMap` inverses to the item's bar slot and to
//! a value within the item's values (one pixel of tolerance). Fixed proptest seed.

mod common;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qp_grammar::scale::{pixel_span, slot_at, value_at};
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::Mark;
use qp_schema::scene::SceneDesc;
use qp_schema::{Bar, Point};

fn bars_from(steps: &[f64], start: f64) -> Vec<Bar> {
    let mut c = start;
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let o = c;
            c = (c * (1.0 + s)).max(0.01);
            let (hi, lo) = (o.max(c), o.min(c));
            Bar {
                t: 1_735_776_000_000 + i as i64 * 86_400_000,
                o,
                h: hi * (1.0 + s.abs() / 2.0),
                l: lo * (1.0 - s.abs() / 2.0),
                c,
                v: 1000.0 + (i % 7) as f64 * 100.0,
            }
        })
        .collect()
}

/// Checks every item of `s`; returns how many were checked.
pub fn check_round_trip(s: &SceneDesc) -> usize {
    let mut n = 0;
    for p in &s.panels {
        for l in &p.layers {
            for it in &l.items {
                let cx = f64::from(it.hit.left + it.hit.right) / 2.0;
                let cy = f64::from(it.hit.top + it.hit.bottom) / 2.0;
                if let Some(bar) = it.bar {
                    let slot = slot_at(&s.x, cx).expect("inside plot");
                    let k = it.bars.unwrap_or(1);
                    assert!(
                        slot >= bar && slot < bar + k,
                        "layer {} item bar {bar} (x{k}) maps back to slot {slot}",
                        l.id
                    );
                }
                let vertical_rule = l.mark == Mark::Rule && it.bar.is_some();
                if it.values.is_empty() || vertical_rule {
                    n += 1;
                    continue;
                }
                let v = value_at(&p.y, cy);
                let lo = it.values.values().copied().fold(f64::INFINITY, f64::min);
                let hi = it
                    .values
                    .values()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let tol = 1.5 * pixel_span(&p.y, v) + 1e-9 * v.abs();
                assert!(
                    v >= lo - tol && v <= hi + tol,
                    "layer {} item {:?}: centre value {v} outside [{lo}, {hi}] ± {tol}",
                    l.id,
                    it.bar
                );
                n += 1;
            }
        }
    }
    n
}

fn provider(bars: &[Bar]) -> MemProvider {
    let mut p = common::provider_with(bars.to_vec(), "RT");
    let last = bars.last().unwrap();
    let pts: Vec<Point> = (0..=10)
        .map(|k| Point {
            t: last.t + k * 86_400_000,
            v: Some(last.c * (1.0 + 0.01 * k as f64)),
        })
        .collect();
    p = p.with_points("fut", pts);
    p
}

fn view(log: bool) -> qp_schema::grammar::View {
    let kind = if log { "log" } else { "linear" };
    common::view(&format!(
        r#"{{"id":"rt","x":{{"kind":"time"}},
          "panels":[{{"id":0,"y":{{"kind":"{kind}"}}}},{{"id":1,"y":{{"kind":"linear"}}}},{{"id":2,"y":{{"kind":"linear"}}}}],
          "layers":[
            {{"id":"band","mark":"band","series":"bb","panel":0,"encoding":{{"y":"upper","y2":"lower"}}}},
            {{"id":"candles","mark":"candle","series":"px","panel":0}},
            {{"id":"sma","mark":"line","series":"sma20","panel":0}},
            {{"id":"fut","mark":"line","series":"fut","panel":0}},
            {{"id":"dots","mark":"point","series":"sma50","panel":0}},
            {{"id":"area","mark":"area","series":"px","panel":1,"encoding":{{"y":"c","y2":{{"series":"sma20","field":"sma"}}}}}},
            {{"id":"vol","mark":"bar","series":"px","panel":1,"encoding":{{"y":"v"}}}},
            {{"id":"macd","mark":"line","series":"macd","panel":2}},
            {{"id":"hist","mark":"bar","series":"macd","panel":2,"encoding":{{"y":"histogram"}}}},
            {{"id":"zero","mark":"rule","panel":2,"encoding":{{"y":0}},
              "origin":{{"/encoding/y":{{"kind":"threshold","name":"macd.zero"}}}}}}]}}"#
    ))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        rng_seed: RngSeed::Fixed(0x0052_2026),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn hit_box_centres_map_back(
        steps in prop::collection::vec(-0.05f64..0.05, 60..400),
        start in 5.0f64..500.0,
        log in any::<bool>(),
        from in 0u64..50,
        span in 10u64..400,
        w in 400u32..1600,
        h in 300u32..900,
    ) {
        let bars = bars_from(&steps, start);
        let p = provider(&bars);
        let mut o = CompileOptions::new(w, h);
        if span < 380 {
            o.range = Some((from, from + span));
        }
        let s = compile(&view(log), &p, &o).unwrap();
        prop_assume!(s.lod.is_none());
        let n = check_round_trip(&s);
        prop_assert!(n > 0);
    }

    #[test]
    fn lod_items_cover_their_buckets(
        steps in prop::collection::vec(-0.03f64..0.03, 3000..6000),
        log in any::<bool>(),
    ) {
        let bars = bars_from(&steps, 100.0);
        let p = provider(&bars);
        let s = compile(&view(log), &p, &CompileOptions::new(1200, 600)).unwrap();
        let lod = s.lod.expect("aggregated");
        prop_assert!(lod.bars_per_bucket >= 2);
        let n = check_round_trip(&s);
        prop_assert!(n > 0);
    }
}
