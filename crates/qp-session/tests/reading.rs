//! Semantic reading from the scene (R2/T6, criterion 7, VERIFY 10): `screen` R2 fields,
//! the point query `hit`, a property test over six views (values equal the data, empty
//! areas have no item) and the data → pixel → data round trip through the protocol.

mod support;

use std::collections::BTreeMap;

use qp_grammar::scale::{pixel_span, slot_x, value_y};
use qp_schema::grammar::Mark;
use qp_schema::scene::{DataSource, SceneDesc, ScenePanel};
use qp_session::CompileOptions;
use serde_json::{json, Value};
use support::H;

/// Deterministic generator for test points (fixed seed; xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, n)`.
    #[allow(clippy::cast_precision_loss)]
    fn below(&mut self, n: f64) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64 * n
    }
}

/// The thyao document plus two point series `q05`/`q95` reaching 20 bars past the last
/// bar (inline in the init: `put_series` accepts only bar and study series in v0).
fn with_points() -> H {
    let mut doc: Value = serde_json::from_str(
        &std::fs::read_to_string(support::root().join("fixtures/protocol/docs/thyao.json"))
            .unwrap(),
    )
    .unwrap();
    let mut q05 = Vec::new();
    let mut q95 = Vec::new();
    let t0 = 1_759_881_600_000_i64; // last bar of `px`
    for k in 0..=20_i64 {
        #[allow(clippy::cast_precision_loss)]
        let spread = k as f64 * 0.6;
        q05.push(json!({"t": t0 + k * 86_400_000, "v": 124.69 - spread}));
        q95.push(json!({"t": t0 + k * 86_400_000, "v": 124.69 + spread}));
    }
    let series = doc["series"].as_array_mut().unwrap();
    for (id, pts) in [("q05", q05), ("q95", q95)] {
        series.push(json!({"id":id,"source":{"kind":"inline"},
            "data":{"kind":"points","points":pts}}));
    }
    let mut init = support::init();
    init.document =
        qp_schema::protocol::DocumentRef::Inline(Box::new(serde_json::from_value(doc).unwrap()));
    H::with(init)
}

/// The six views of the property test, built with protocol messages only.
fn six_views(h: &mut H) -> Vec<&'static str> {
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    let r = h.cmd(
        "a",
        "c1",
        json!([
            {"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20}},
            {"op":"put_view","view":{"id":"boll","x":{"kind":"time"},
                "panels":[{"id":0,"y":{"kind":"linear"}}],
                "layers":[
                  {"id":"band","mark":"band","series":"bb20","panel":0,
                   "encoding":{"y":"upper","y2":"lower","color":"accent","opacity":0.18}},
                  {"id":"mid","mark":"line","series":"bb20","panel":0,"encoding":{"y":"middle"}},
                  {"id":"candles","mark":"candle","series":"px","panel":0}]}},
            {"op":"put_view","view":{"id":"logv","x":{"kind":"time"},
                "panels":[{"id":0,"y":{"kind":"log"}}],
                "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
                          {"id":"sma","mark":"line","series":"sma20","panel":0}]}},
            {"op":"put_view","view":{"id":"hist","x":{"kind":"time"},
                "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
                          {"id":1,"name":"macd","y":{"kind":"linear"}}],
                "layers":[
                  {"id":"close","mark":"area","series":"px","panel":0,"encoding":{"y":"c"}},
                  {"id":"hist","mark":"bar","series":"macd","panel":1,
                   "encoding":{"y":"histogram",
                     "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}},
                  {"id":"zero","mark":"rule","panel":1,"encoding":{"y":0}}]}},
            {"op":"put_view","view":{"id":"fan","x":{"kind":"time"},
                "panels":[{"id":0,"y":{"kind":"linear"}}],
                "layers":[
                  {"id":"candles","mark":"candle","series":"px","panel":0},
                  {"id":"b90","mark":"band","series":"q05","panel":0,
                   "encoding":{"y":"v","y2":{"series":"q95","field":"v"},"opacity":0.2}}]}},
            {"op":"show_finding","finding":"fnd-overbought","view":"pres"}
        ]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    vec!["main", "boll", "logv", "hist", "fan", "pres"]
}

/// All values of every series (bar columns and study outputs) through the `study`
/// query; points series by time through `get`.
struct Data {
    series: BTreeMap<String, BTreeMap<String, Vec<Option<f64>>>>,
    points: BTreeMap<String, BTreeMap<i64, f64>>,
    facts: BTreeMap<String, Value>,
}

fn data(h: &mut H) -> Data {
    let mut d = Data {
        series: BTreeMap::new(),
        points: BTreeMap::new(),
        facts: BTreeMap::new(),
    };
    for id in ["px", "rsi14", "macd", "bb20", "sma20"] {
        let r = h.query("s", json!({"kind":"study","series":id}));
        let outs: BTreeMap<String, Vec<Option<f64>>> =
            serde_json::from_value(r["data"]["values"]["outputs"].clone()).unwrap();
        d.series.insert(id.to_owned(), outs);
    }
    for id in ["q05", "q95"] {
        let r = h.query("s", json!({"kind":"get","id":id}));
        let pts = r["data"]["item"]["data"]["points"].as_array().unwrap();
        d.points.insert(
            id.to_owned(),
            pts.iter()
                .map(|p| (p["t"].as_i64().unwrap(), p["v"].as_f64().unwrap()))
                .collect(),
        );
    }
    let r = h.query("s", json!({"kind":"get","id":"fact-rsi-cross70"}));
    d.facts
        .insert("fact-rsi-cross70".to_owned(), r["data"]["item"].clone());
    d
}

impl Data {
    /// The data value of `field` of `series` at bar `bar` (time `t` for points).
    fn value(&self, series: &str, field: &str, bar: Option<u64>, t: Option<i64>) -> Option<f64> {
        if let Some(p) = self.points.get(series) {
            return p.get(&t?).copied();
        }
        let col = self.series.get(series)?.get(field)?;
        col.get(usize::try_from(bar?).ok()?).copied().flatten()
    }

    /// `(series, field)` of an item value key of a layer drawing `series`.
    fn resolve<'a>(&self, series: &'a str, key: &'a str) -> (&'a str, &'a str) {
        if let Some((s, f)) = key.split_once('.') {
            if self.series.contains_key(s) || self.points.contains_key(s) {
                return (s, f);
            }
        }
        (series, key)
    }
}

fn hit(h: &mut H, conn: &str, view: &str, x: f64, y: f64) -> Value {
    let r = h.query(conn, json!({"kind":"hit","view":view,"x":x,"y":y}));
    assert_eq!(r["type"], "result", "{r}");
    r["data"]["hit"].clone()
}

/// Checks that a hit item's values equal the data. Returns the number of values checked.
fn check_item(d: &Data, item: &Value) -> usize {
    let values: BTreeMap<String, f64> = serde_json::from_value(item["values"].clone()).unwrap();
    let bar = item["bar"].as_u64();
    let t = item["t"].as_i64();
    let mut n = 0;
    match item["data"]["kind"].as_str().unwrap() {
        "series" => {
            let series = item["data"]["series"].as_str().unwrap();
            for (k, v) in &values {
                if item["mark"] == "bar" && k == "base" {
                    // The default base of a bar without `y2` (`docs/GRAMMAR.md` §5).
                    assert_eq!(*v, 0.0, "{item}");
                    n += 1;
                    continue;
                }
                let (s, f) = d.resolve(series, k);
                let want = d
                    .value(s, f, bar, t)
                    .unwrap_or_else(|| panic!("no data for {s}.{f} at {bar:?}/{t:?}: {item}"));
                assert_eq!(*v, want, "{s}.{f} at bar {bar:?}: {item}");
                n += 1;
            }
        }
        "constant" => {
            let consts: Vec<f64> = item["constants"]
                .as_object()
                .unwrap()
                .values()
                .map(|c| c["value"].as_f64().unwrap())
                .collect();
            for v in values.values() {
                assert!(consts.contains(v), "{v} not a constant of {item}");
                n += 1;
            }
        }
        "fact" => {
            let f = &d.facts[item["data"]["fact"].as_str().unwrap()];
            assert_eq!(bar, f["bar"].as_u64());
            n += 1;
        }
        other => panic!("unexpected data kind {other}"),
    }
    n
}

/// Pixel rows (min, max) the data of the panel's layers covers at `slot`, from the data
/// (not from the scene's items): candle `l..h`, line/point values, band `y..y2`, bar
/// `0..y`, area `y..panel bottom`, horizontal rule its constant.
fn data_rows(d: &Data, scene: &SceneDesc, p: &ScenePanel, slot: u64) -> Vec<(f64, f64)> {
    let t = scene
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .flat_map(|l| &l.items)
        .find(|i| i.bar == Some(slot))
        .and_then(|i| i.t);
    let mut rows = Vec::new();
    for l in &p.layers {
        let mut vals: Vec<f64> = Vec::new();
        match &l.data {
            DataSource::Series { series } => {
                for f in &l.fields {
                    let (s, f) = d.resolve(series, f);
                    if let Some(v) = d.value(s, f, Some(slot), t) {
                        vals.push(v);
                    }
                }
                if vals.is_empty() {
                    continue;
                }
                match l.mark {
                    Mark::Bar => vals.push(0.0),
                    Mark::Area => vals.push(p.y.min),
                    _ => {}
                }
            }
            DataSource::Constant {} => vals.extend(l.constants.values().map(|c| c.value)),
            DataSource::Fact { .. } | DataSource::Finding { .. } => continue,
        }
        let ys: Vec<f64> = vals.iter().filter_map(|v| value_y(&p.y, *v)).collect();
        if let (Some(lo), Some(hi)) = (
            ys.iter().copied().reduce(f64::min),
            ys.iter().copied().reduce(f64::max),
        ) {
            rows.push((lo, hi));
        }
    }
    rows
}

#[test]
fn screen_reports_panels_scales_and_layers_from_the_scene() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    let r = h.query("s", json!({"kind":"screen","principal":"arkin"}));
    let sc = &r["data"]["screens"][0];
    // RD1 fields unchanged.
    assert_eq!(
        sc["viewport"],
        json!({"view":"main","from_bar":80,"to_bar":199})
    );
    assert_eq!(sc["panels"][0]["series"], json!(["px", "bb20"]));
    // R2 fields.
    let p0 = &sc["panels"][0];
    assert_eq!(p0["scale"]["id"], "y0");
    assert_eq!(p0["scale"]["kind"], "linear");
    assert_eq!(p0["layers"][0]["id"], "l0");
    assert_eq!(p0["layers"][0]["mark"], "candle");
    assert_eq!(p0["layers"][0]["items"], 120);
    assert_eq!(p0["layers"][0]["last"]["c"], 124.69);
    assert_eq!(
        p0["layers"][1]["fields"],
        json!(["upper", "middle", "lower"])
    );
    let p1 = &sc["panels"][1];
    assert_eq!(p1["scale"]["domain"], json!([0.0, 100.0]));
    let rule = &p1["layers"][1];
    assert_eq!(rule["mark"], "rule");
    assert_eq!(rule["data"], json!({"kind":"constant"}));
    assert_eq!(
        rule["constants"]["/encoding/y"],
        json!({"origin":{"kind":"threshold","name":"rsi.overbought"},"value":70.0})
    );
    // The values the screen reports are the data's.
    let rsi = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","range":{"last":1}}),
    );
    assert_eq!(
        p1["layers"][0]["last"]["rsi"],
        rsi["data"]["values"]["outputs"]["rsi"][0]
    );
    println!("{}", serde_json::to_string_pretty(sc).unwrap());
}

#[test]
fn screen_follows_the_viewport_and_the_surface() {
    let mut h = H::new();
    h.hello("p", "arkin", "phone");
    h.hello("s", "scout", "headless");
    let r = h.query("s", json!({"kind":"screen","principal":"arkin"}));
    let sc = &r["data"]["screens"][0];
    assert_eq!(sc["viewport"]["from_bar"], 140);
    assert_eq!(sc["panels"][0]["layers"][0]["items"], 60);
    h.one(
        "p",
        json!({"type":"input","id":"i","gesture":{"kind":"look","view":"main","from_bar":190,"to_bar":199}}),
    );
    let r = h.query("s", json!({"kind":"screen","principal":"arkin"}));
    assert_eq!(
        r["data"]["screens"][0]["panels"][0]["layers"][0]["items"],
        10
    );
}

#[test]
fn hit_examples_item_and_empty() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    // The asker's viewport: bars 80..=199 (120 slots over the 1120-pixel plot).
    let mut opts = CompileOptions::new(1200, 600);
    opts.range = None;
    let scene = h.s.scene_for("a", Some("main"), &opts).unwrap();
    assert_eq!((scene.x.from_bar, scene.x.to_bar), (80, 199));
    let p0 = &scene.panels[0];
    let p1 = &scene.panels[1];
    let bar = 150_u64;
    let at = h.query("a", json!({"kind":"at","series":"px","bar":bar}));
    let rsi = at["data"]["at"]["studies"]["rsi14"]["rsi"]
        .as_f64()
        .unwrap();
    let x = slot_x(&scene.x, bar) + 0.5;
    let y = value_y(&p1.y, rsi).unwrap();
    let r = hit(&mut h, "a", "main", x, y);
    println!("item: {r}");
    assert_eq!(r["panel"], 1);
    assert_eq!(r["bar"], bar);
    assert_eq!(r["item"]["layer"], "l2");
    assert_eq!(r["item"]["mark"], "line");
    assert_eq!(r["item"]["values"]["rsi"], rsi);
    // The rule at 70 (named threshold) in the same panel, away from the RSI line.
    let r = hit(
        &mut h,
        "a",
        "main",
        slot_x(&scene.x, 100) + 0.5,
        value_y(&p1.y, 70.0).unwrap(),
    );
    println!("rule: {r}");
    assert_eq!(r["item"]["mark"], "rule");
    assert_eq!(
        r["item"]["constants"]["/encoding/y"]["origin"],
        json!({"kind":"threshold","name":"rsi.overbought"})
    );
    // A candle where the close is outside the Bollinger lines' box (bar 199).
    let c199 =
        h.query("a", json!({"kind":"at","series":"px","bar":199}))["data"]["at"]["ohlcv"].clone();
    let r = hit(
        &mut h,
        "a",
        "main",
        slot_x(&scene.x, 199) + 0.5,
        value_y(&p0.y, c199["c"].as_f64().unwrap()).unwrap(),
    );
    println!("candle or line at 199: {r}");
    assert_eq!(r["bar"], 199);
    // Above all data in the price panel: no item.
    let r = hit(&mut h, "a", "main", x, f64::from(p0.rect.top) + 1.0);
    println!("empty: {r}");
    assert_eq!(r["panel"], 0);
    assert!(r.get("item").is_none(), "{r}");
    // Outside every panel (title row).
    let r = hit(&mut h, "a", "main", x, 5.0);
    assert!(r.get("panel").is_none() && r.get("item").is_none(), "{r}");
    // The view defaults to the asker's current view; a bad view is not_found.
    let r = h.query("a", json!({"kind":"hit","x":x,"y":y}));
    assert_eq!(r["data"]["hit"]["view"], "main");
    let r = h.query("a", json!({"kind":"hit","view":"nope","x":1,"y":1}));
    assert_eq!(r["error"]["code"], "not_found");
    let r = h.query("a", json!({"kind":"hit","x":1,"y":1,"width":10}));
    assert_eq!(r["error"]["code"], "invalid");
}

/// VERIFY 10: six views (multi-panel, band, log scale, bars + area + rule, points past
/// the last bar, the presenter's view), 250 random pixels each (fixed seed): every item
/// the point query returns carries the data's values; a pixel away from all data of its
/// slot (computed from the data, not from the scene) has no item.
#[test]
fn hit_property_values_equal_data_and_empty_areas_have_no_item() {
    let mut h = with_points();
    let views = six_views(&mut h);
    let d = data(&mut h);
    let mut rng = Rng(0x5EED_0F0A_7E57);
    let (mut items, mut empty, mut checked, mut far) = (0, 0, 0, 0);
    let mut by_mark: BTreeMap<String, usize> = BTreeMap::new();
    for conn in ["a", "s"] {
        for view in &views {
            let scene =
                h.s.scene_for(conn, Some(view), &CompileOptions::new(1200, 600))
                    .unwrap();
            for _ in 0..250 {
                let x = rng.below(1200.0).floor() + 0.5;
                let y = rng.below(600.0).floor() + 0.5;
                let r = hit(&mut h, conn, view, x, y);
                // Is the pixel far (> 6 px) from every data span of its slot?
                let is_far = match (r["panel"].as_u64(), r["bar"].as_u64()) {
                    (Some(pid), Some(slot)) => {
                        let p = scene
                            .panels
                            .iter()
                            .find(|p| u64::from(p.id) == pid)
                            .unwrap();
                        data_rows(&d, &scene, p, slot)
                            .iter()
                            .all(|(lo, hi)| y < lo - 6.0 || y > hi + 6.0)
                    }
                    _ => true,
                };
                if let Some(item) = r.get("item") {
                    assert!(!is_far, "{view}: item at ({x},{y}) far from all data: {r}");
                    items += 1;
                    checked += check_item(&d, item);
                    *by_mark
                        .entry(item["mark"].as_str().unwrap().to_owned())
                        .or_default() += 1;
                } else {
                    empty += 1;
                    if is_far {
                        far += 1;
                    }
                }
            }
        }
    }
    println!(
        "hit property: {} views x 2 askers x 250 pixels; {items} items ({checked} values equal \
         the data; by mark {by_mark:?}), {empty} empty ({far} far from all data)",
        views.len()
    );
    assert!(items >= 6 * 20, "too few items hit: {items}");
    assert!(far >= 6 * 20, "too few empty checks: {far}");
    for m in ["candle", "line", "band", "bar", "area", "rule"] {
        assert!(by_mark.contains_key(m), "no {m} item hit: {by_mark:?}");
    }
}

/// The empty-area half of the property, asserted: a pixel far (> 6 px) from every data
/// span of its slot returns no item.
#[test]
fn hit_far_from_data_is_empty() {
    let mut h = with_points();
    let views = six_views(&mut h);
    let d = data(&mut h);
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let mut asserted = 0;
    for view in &views {
        let scene =
            h.s.scene_for("s", Some(view), &CompileOptions::new(1200, 600))
                .unwrap();
        let mut per_view = 0;
        let mut tries = 0;
        while per_view < 20 && tries < 5000 {
            tries += 1;
            let p = &scene.panels[usize::try_from(rng.next() % scene.panels.len() as u64).unwrap()];
            let x = f64::from(p.rect.left) + rng.below(f64::from(p.rect.right - p.rect.left));
            let y = f64::from(p.rect.top) + rng.below(f64::from(p.rect.bottom - p.rect.top));
            let x = x.floor() + 0.5;
            let y = y.floor() + 0.5;
            let Some(slot) = qp_grammar::scale::slot_at(&scene.x, x) else {
                continue;
            };
            let rows = data_rows(&d, &scene, p, slot);
            if rows.iter().any(|(lo, hi)| y >= lo - 6.0 && y <= hi + 6.0) {
                continue;
            }
            let r = hit(&mut h, "s", view, x, y);
            assert_eq!(r["panel"].as_u64(), Some(u64::from(p.id)));
            assert!(
                r.get("item").is_none(),
                "{view}: pixel ({x},{y}) is far from data at slot {slot} but hit {r}"
            );
            per_view += 1;
        }
        assert_eq!(per_view, 20, "{view}: not enough empty pixels");
        asserted += per_view;
    }
    println!(
        "{asserted} empty pixels asserted over {} views",
        views.len()
    );
}

/// Data → pixel → data through the protocol: for random visible bars and every series
/// layer, the pixel computed from the scene's mappings (`XMap`, `YMap`) is queried with
/// `hit`; the answer has the same bar and the value at the pixel is the data value within
/// one pixel. When the topmost item is that layer's, its values contain the data value.
#[test]
fn round_trip_data_pixel_data_through_hit() {
    let mut h = with_points();
    let views = six_views(&mut h);
    let d = data(&mut h);
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let (mut trips, mut same_layer) = (0, 0);
    for view in &views {
        let scene =
            h.s.scene_for("s", Some(view), &CompileOptions::new(1200, 600))
                .unwrap();
        let span = scene.x.to_bar.min(scene.bars - 1) - scene.x.from_bar + 1;
        for _ in 0..25 {
            let bar = scene.x.from_bar + rng.next() % span;
            for p in &scene.panels {
                for l in &p.layers {
                    let DataSource::Series { series } = &l.data else {
                        continue;
                    };
                    let field = if l.mark == Mark::Candle {
                        "c"
                    } else {
                        l.fields[0].as_str()
                    };
                    let (s, f) = d.resolve(series, field);
                    let Some(v) = d.value(s, f, Some(bar), None) else {
                        continue;
                    };
                    let Some(py) = value_y(&p.y, v) else {
                        continue;
                    };
                    let px = slot_x(&scene.x, bar) + 0.5;
                    let r = hit(&mut h, "s", view, px, py);
                    assert_eq!(r["bar"].as_u64(), Some(bar), "{view}/{}: {r}", l.id);
                    assert_eq!(r["panel"].as_u64(), Some(u64::from(p.id)));
                    let got = r["value"].as_f64().unwrap();
                    let tol = pixel_span(&p.y, v);
                    assert!(
                        (got - v).abs() <= tol,
                        "{view}/{}: value {got} vs data {v} (tolerance {tol})",
                        l.id
                    );
                    if r["item"]["layer"] == l.id.as_str() {
                        same_layer += 1;
                        let vals: BTreeMap<String, f64> =
                            serde_json::from_value(r["item"]["values"].clone()).unwrap();
                        // One item per (slot, output) (GRAMMAR §16): the topmost item
                        // is this output's, or another output of the same layer drawn
                        // later whose ±4 px box also covers the pixel.
                        if let Some(x) = vals.get(field) {
                            assert!(*x == v, "{view}/{}: {v} vs {x}", l.id);
                        } else {
                            assert_eq!(vals.len(), 1, "{view}/{}: {vals:?}", l.id);
                            let other = vals.values().next().copied().unwrap();
                            let oy = value_y(&p.y, other).unwrap();
                            assert!(
                                (oy - py).abs() <= 5.0,
                                "{view}/{}: {v} hit {vals:?} {} px away",
                                l.id,
                                (oy - py).abs()
                            );
                        }
                    }
                    trips += 1;
                }
            }
        }
    }
    println!("round trip: {trips} data -> pixel -> data trips, {same_layer} on the layer itself");
    assert!(trips >= 6 * 25);
}
