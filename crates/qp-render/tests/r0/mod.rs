//! Test/example helper (not part of the library): the R0 view (candles of `px` plus the
//! SMA(20) line of `sma20`, one linear panel) compiled by `qp-grammar` from a provider
//! whose study values come from `qp-compute`. The renderer only rasterizes the result.
#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::View;
use qp_schema::scene::SceneDesc;
use qp_schema::Series;

/// The V0 view (`rounds/R2/tasks/VIEWS.md` V0), as in
/// `crates/qp-grammar/tests/r0_equivalence.rs`.
pub const R0_VIEW: &str = r#"{"id":"r0","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"sma","mark":"line","series":"sma20","panel":0}]}"#;

pub fn view() -> View {
    serde_json::from_str(R0_VIEW).expect("R0 view")
}

/// `px` = the series' bars, `sma20` = SMA(20) of close from the compute core.
pub fn provider(series: &Series) -> MemProvider {
    let bars = series.bars().expect("series has bars").to_vec();
    let params = BTreeMap::from([("period".to_owned(), 20.0)]);
    let sma = qp_compute::compute_bars("sma", &params, qp_compute::Field::Close, &bars)
        .expect("sma")
        .outputs
        .remove("sma")
        .expect("sma output");
    MemProvider::new()
        .with_bars("px", bars, series.symbol(), series.interval.as_deref())
        .with_values("sma20", "px", vec![("sma", sma)], "sma", &["sma"])
        .with_label("sma20", "SMA20")
}

/// The R0 scene for `series` at `options`.
pub fn scene_with(series: &Series, options: &CompileOptions) -> SceneDesc {
    compile(&view(), &provider(series), options).expect("compile R0 view")
}

/// The R0 scene for `series` at `width` x `height` (default options: LOD on).
pub fn scene(series: &Series, width: u32, height: u32) -> SceneDesc {
    scene_with(series, &qp_designs::options(width, height))
}

/// The R0 scene for `series` at `width` x `height` with level of detail off (every bar
/// drawn, as the R0 layout did).
pub fn scene_full(series: &Series, width: u32, height: u32) -> SceneDesc {
    let mut o = qp_designs::options(width, height);
    o.lod = false;
    scene_with(series, &o)
}
