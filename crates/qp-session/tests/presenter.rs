//! The presenter on the grammar's depth rules (R2/T5b, criterion 5, VERIFY 11): three
//! surface profiles give `sentence`, `capsule` and `chart`; the output is deterministic
//! (1,000 presentations per depth, one serialized output each); both views compile.
//! `presenter_timing` (ignored) repeats the RD1/T6e latency measurement.

mod support;

use std::collections::BTreeSet;
use std::time::Instant;

use qp_grammar::{compile, depth, CompileOptions};
use qp_schema::grammar::View;
use qp_schema::protocol::SurfaceKind;
use qp_schema::Depth;
use serde_json::{json, Value};
use support::H;

fn present(h: &mut H, surface: &str) -> Value {
    let r = h.query(
        "s",
        json!({"kind":"present","finding":"fnd-overbought","surface":surface}),
    );
    r["data"]["presentation"].clone()
}

#[test]
fn three_surfaces_three_depths() {
    let mut h = H::new();
    h.hello("s", "scout", "headless");
    let w = present(&mut h, "watch");
    assert_eq!(w["depth"], "sentence");
    assert_eq!(w["text"], "THYAO aşırı alımda: RSI 70'in üstünde.");
    for k in ["view", "capsule_view", "spark", "confidence", "glyphs"] {
        assert!(w.get(k).is_none(), "watch has no {k}: {w}");
    }
    let p = present(&mut h, "phone");
    assert_eq!(p["depth"], "capsule");
    assert!(p.get("view").is_none(), "{p}");
    assert_eq!(p["confidence"], 0.65);
    assert_eq!(p["glyphs"].as_array().unwrap().len(), 3);
    let cv = &p["capsule_view"];
    assert_eq!(cv["id"], "capsule-fnd-overbought");
    assert_eq!(cv["title"], p["text"]);
    assert_eq!(cv["panels"].as_array().unwrap().len(), 1);
    assert_eq!(
        cv["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| (l["id"].as_str().unwrap(), l["mark"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![("line", "line"), ("level", "rule"), ("fact", "point")]
    );
    assert_eq!(
        cv["layers"][1]["origin"]["/encoding/y"],
        json!({"kind":"fact","fact":"fact-rsi-cross70","field":"level"})
    );
    let d = present(&mut h, "desktop");
    assert_eq!(d["depth"], "chart");
    assert!(d.get("capsule_view").is_none());
    assert_eq!(d["view"]["id"], "present-fnd-overbought");
    let ids: Vec<&str> = d["view"]["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["id"].as_str().unwrap())
        .collect();
    // K25: the grammar's own layer names, as in the capsule view.
    assert_eq!(ids, ["candles", "line", "level"]);
    assert_eq!(d["facts"][0]["id"], "fact-rsi-cross70");

    // Both views compile against the document (the capsule view at the phone frame).
    let cv: View = serde_json::from_value(p["capsule_view"].clone()).unwrap();
    let chart: View = serde_json::from_value(d["view"].clone()).unwrap();
    let prov = h.s.provider();
    let s = compile(&cv, &prov, &CompileOptions::new(390, 300)).unwrap();
    assert_eq!(s.x.to_bar - s.x.from_bar + 1, 60);
    assert_eq!(s.panels[0].layers[2].items[0].bar, Some(150));
    let s = compile(&chart, &prov, &CompileOptions::new(1200, 600)).unwrap();
    assert_eq!((s.x.from_bar, s.x.to_bar), (120, 199));
    assert_eq!(s.emphasis[0].from_bar, Some(150));
}

#[test]
fn presenter_is_deterministic_1000_times_per_depth() {
    let mut h = H::new();
    h.hello("s", "scout", "headless");
    for (surface, depth) in [
        ("watch", "sentence"),
        ("phone", "capsule"),
        ("desktop", "chart"),
    ] {
        let mut outs = BTreeSet::new();
        for _ in 0..1000 {
            let p = present(&mut h, surface);
            assert_eq!(p["depth"], depth);
            outs.insert(serde_json::to_string(&p).unwrap());
        }
        assert_eq!(outs.len(), 1, "{surface}: {} different outputs", outs.len());
    }
    // The grammar functions directly, through the authority's provider.
    let prov = h.s.provider();
    for d in [Depth::Sentence, Depth::Capsule, Depth::Chart] {
        let kind = match d {
            Depth::Sentence => SurfaceKind::Watch,
            Depth::Capsule => SurfaceKind::Phone,
            _ => SurfaceKind::Desktop,
        };
        let profile = qp_session::profile_for(kind, None);
        let mut outs = BTreeSet::new();
        for _ in 0..1000 {
            let p = depth::present("fnd-overbought", d, &profile, &prov).unwrap();
            assert_eq!(p.depth, d);
            outs.insert(format!("{p:?}"));
        }
        assert_eq!(outs.len(), 1, "{d:?}");
    }
}

#[test]
fn show_finding_stores_the_grammar_chart_view() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"show_finding","finding":"fnd-overbought","view":"vp"}]),
    );
    assert_eq!(r["status"], "applied");
    let stored =
        h.s.document()
            .views
            .iter()
            .find(|v| v.id == "vp")
            .unwrap()
            .clone();
    let prov = h.s.provider();
    // K25: stored as the grammar rule builds it, with the grammar's layer names.
    let rule = depth::chart_view("fnd-overbought", &prov, "vp").unwrap();
    assert_eq!(stored, rule);
}

fn median(mut v: Vec<u128>) -> (u128, u128) {
    v.sort_unstable();
    (v[v.len() / 2], v[v.len() * 99 / 100])
}

/// RD1/T6e's measurement on the grammar presenter (`cargo test --release -p qp-session
/// --test presenter -- --ignored --nocapture`).
#[test]
#[ignore = "timing; run in release with --ignored --nocapture"]
#[allow(clippy::cast_possible_truncation)]
fn presenter_timing() {
    let mut h = H::new();
    let prov = h.s.provider();
    let desktop = qp_session::profile_for(SurfaceKind::Desktop, None);
    let phone = qp_session::profile_for(SurfaceKind::Phone, None);
    let watch = qp_session::profile_for(SurfaceKind::Watch, None);
    let mut rows = Vec::new();
    let mut bench = |name: &str, n: usize, f: &mut dyn FnMut()| {
        for _ in 0..1000 {
            f();
        }
        let mut t = Vec::with_capacity(n);
        for _ in 0..n {
            let s = Instant::now();
            f();
            t.push(s.elapsed().as_nanos());
        }
        let (m, p99) = median(t);
        rows.push(format!("| {name} | {n} | {m} | {p99} |"));
    };
    bench("depth::chart_view (function)", 100_000, &mut || {
        std::hint::black_box(depth::chart_view("fnd-overbought", &prov, "v").unwrap());
    });
    bench("depth::present sentence (watch)", 100_000, &mut || {
        std::hint::black_box(
            depth::present("fnd-overbought", Depth::Chart, &watch, &prov).unwrap(),
        );
    });
    bench("depth::present capsule (phone)", 100_000, &mut || {
        std::hint::black_box(
            depth::present("fnd-overbought", Depth::Chart, &phone, &prov).unwrap(),
        );
    });
    bench("depth::present chart (desktop)", 100_000, &mut || {
        std::hint::black_box(
            depth::present("fnd-overbought", Depth::Chart, &desktop, &prov).unwrap(),
        );
    });
    // show_finding end to end: a fresh session per measurement (set-up outside the timer).
    let line = json!({"type":"command","id":"c1","ops":[{"op":"show_finding","finding":"fnd-overbought"}]})
        .to_string();
    let mut t = Vec::new();
    for i in 0..3000 + 100 {
        let mut s = H::new();
        s.hello("e", "editor", "headless");
        let st = Instant::now();
        std::hint::black_box(s.s.handle_raw("e", &line));
        if i >= 100 {
            t.push(st.elapsed().as_nanos());
        }
    }
    let (m, p99) = median(t);
    rows.push(format!(
        "| show_finding end to end (session) | 3000 | {m} | {p99} |"
    ));
    let mut q = H::new();
    q.hello("d", "arkin", "desktop");
    q.hello("p", "arkin", "phone");
    for (conn, name) in [("d", "desktop chart"), ("p", "phone capsule")] {
        let line =
            json!({"type":"query","id":"q","query":{"kind":"present","finding":"fnd-overbought"}})
                .to_string();
        let mut t = Vec::new();
        for i in 0..20_000 + 1000 {
            let st = Instant::now();
            std::hint::black_box(q.s.handle_raw(conn, &line));
            if i >= 1000 {
                t.push(st.elapsed().as_nanos());
            }
        }
        let (m, p99) = median(t);
        rows.push(format!("| present query, {name} | 20000 | {m} | {p99} |"));
    }
    println!("| measurement | n | median ns | p99 ns |\n|---|--:|--:|--:|");
    for r in rows {
        println!("{r}");
    }
}
