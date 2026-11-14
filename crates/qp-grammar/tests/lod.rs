//! Viewport and level of detail (`docs/GRAMMAR.md` §3, §15).

mod common;

use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::scene::SceneLayer;
use qp_schema::Bar;

fn big(n: usize) -> Vec<Bar> {
    let mut c = 100.0_f64;
    (0..n)
        .map(|i| {
            let o = c;
            let s = (((i * 7919) % 101) as f64 - 50.0) / 2000.0;
            c *= 1.0 + s;
            Bar {
                t: 1_000_000_000_000 + i as i64 * 60_000,
                o,
                h: o.max(c) + 0.1,
                l: o.min(c) - 0.1,
                c,
                v: (i % 13) as f64,
            }
        })
        .collect()
}

fn provider(bars: &[Bar]) -> MemProvider {
    let hist: Vec<Option<f64>> = bars.iter().map(|b| Some(b.c - b.o)).collect();
    MemProvider::new()
        .with_bars("px", bars.to_vec(), Some("BIG"), Some("1m"))
        .with_values("d", "px", vec![("d", hist)], "d", &["d"])
}

const VIEW: &str = r#"{"id":"lod","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}},{"id":1,"y":{"kind":"linear"}}],
  "layers":[{"id":"c","mark":"candle","series":"px","panel":0},
            {"id":"l","mark":"line","series":"px","panel":0,"encoding":{"y":"c"}},
            {"id":"b","mark":"bar","series":"d","panel":1}]}"#;

fn layer<'a>(s: &'a qp_schema::scene::SceneDesc, id: &str) -> &'a SceneLayer {
    s.panels
        .iter()
        .flat_map(|p| &p.layers)
        .find(|l| l.id == id)
        .unwrap()
}

#[test]
fn buckets_aggregate_ohlc_lines_and_bars() {
    let bars = big(10_000);
    let p = provider(&bars);
    let s = compile(&common::view(VIEW), &p, &CompileOptions::new(1200, 600)).unwrap();
    let lod = s.lod.unwrap();
    assert_eq!(lod.bars_per_bucket, 9); // ceil(10000 / 1120)
    assert_eq!(lod.buckets, 1111); // remainder joins the last bucket
    let c = layer(&s, "c");
    assert_eq!(c.items.len(), 1111);
    assert_eq!(c.prims.len(), 2 * 1111);
    let first = &c.items[0];
    assert_eq!(first.bars, Some(9));
    assert_eq!(first.values["o"], bars[0].o);
    assert_eq!(first.values["c"], bars[8].c);
    assert_eq!(
        first.values["h"],
        bars[..9].iter().map(|b| b.h).fold(f64::MIN, f64::max)
    );
    assert_eq!(
        first.values["l"],
        bars[..9].iter().map(|b| b.l).fold(f64::MAX, f64::min)
    );
    let last = c.items.last().unwrap();
    assert_eq!(last.bar, Some(9990));
    assert_eq!(last.bars, Some(10));
    // Line: first/min/max/last per bucket, one vertical stroke each.
    let l = layer(&s, "l");
    assert_eq!(l.items.len(), 1111);
    assert_eq!(
        l.items[0].values["c.min"],
        bars[..9].iter().map(|b| b.c).fold(f64::MAX, f64::min)
    );
    assert_eq!(
        l.items[0].values["c.max"],
        bars[..9].iter().map(|b| b.c).fold(f64::MIN, f64::max)
    );
    assert_eq!(l.items[0].values["c"], bars[8].c);
    let qp_schema::scene::Prim::Polyline { points, .. } = &l.prims[0] else {
        panic!()
    };
    assert_eq!(points.len(), 4 * 1111);
    // Bars: largest absolute value of each bucket.
    let b = layer(&s, "b");
    let want =
        bars[..9]
            .iter()
            .map(|b| b.c - b.o)
            .fold(0.0_f64, |a, v| if v.abs() > a.abs() { v } else { a });
    assert_eq!(b.items[0].values["d"], want);
}

#[test]
fn lod_can_be_disabled_and_max_points_forces_it() {
    let bars = big(3000);
    let p = provider(&bars);
    let mut o = CompileOptions::new(1200, 600);
    o.lod = false;
    let s = compile(&common::view(VIEW), &p, &o).unwrap();
    assert_eq!(s.lod, None);
    assert_eq!(layer(&s, "c").items.len(), 3000);
    assert!(layer(&s, "c").items.iter().all(|i| i.bars.is_none()));
    let mut o = CompileOptions::new(1200, 600);
    o.range = Some((0, 199));
    o.max_points = Some(60);
    let s = compile(&common::view(VIEW), &p, &o).unwrap();
    assert_eq!(s.lod.unwrap().bars_per_bucket, 4);
    assert_eq!(layer(&s, "c").items.len(), 50);
}

#[test]
fn viewport_in_a_large_series() {
    let bars = big(100_000);
    let p = provider(&bars);
    let mut o = CompileOptions::new(1200, 600);
    o.range = Some((50_000, 50_199));
    let s = compile(&common::view(VIEW), &p, &o).unwrap();
    assert_eq!(s.lod, None);
    assert_eq!((s.x.from_bar, s.x.to_bar), (50_000, 50_199));
    assert_eq!(layer(&s, "c").items[0].bar, Some(50_000));
    let lo = bars[50_000..50_200]
        .iter()
        .map(|b| b.l)
        .fold(f64::MAX, f64::min);
    assert!(s.panels[0].y.min < lo && s.panels[0].y.min > lo - 10.0);
    // Clamped to the last slot.
    o.range = Some((99_990, 200_000));
    let s = compile(&common::view(VIEW), &p, &o).unwrap();
    assert_eq!((s.x.from_bar, s.x.to_bar), (99_990, 99_999));
}
