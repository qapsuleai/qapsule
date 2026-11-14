//! Evaluates model-written `View`s: format, schema (qp-schema), typed parse, engine
//! acceptance (`put_view` through the session), semantics, distinctness.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qp_schema::{Document, Mark, View};
use serde_json::{json, Value};

const FACT_T: f64 = 1_753_920_000_000.0;
const FIRST_T: f64 = 1_735_776_000_000.0;
const LAST_T: f64 = 1_759_881_600_000.0;

fn extract(text: &str) -> (String, bool) {
    let t = text.trim();
    if t.starts_with('{') && t.ends_with('}') {
        return (t.to_owned(), true);
    }
    // Tolerant extraction: first '{' .. last '}' (format violation recorded).
    match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if b > a => (t[a..=b].to_owned(), false),
        _ => (t.to_owned(), false),
    }
}

fn panel_of<'a>(v: &'a View, pred: impl Fn(&qp_schema::Layer) -> bool) -> Vec<u32> {
    v.layers.iter().filter(|l| pred(l)).map(|l| l.panel).collect()
}

fn semantics(v: &View, doc: &Document) -> BTreeMap<&'static str, Value> {
    let ids: BTreeSet<&str> = doc.series.iter().map(|s| s.id.as_str()).collect();
    let mut m = BTreeMap::new();
    let unknown: Vec<&str> = v
        .layers
        .iter()
        .filter_map(|l| l.series.as_deref())
        .filter(|s| !ids.contains(s))
        .collect();
    m.insert("series_exist", json!(unknown.is_empty()));
    m.insert("unknown_series", json!(unknown));
    m.insert("id_ok", json!(v.id == "present-fnd-overbought"));
    m.insert(
        "panels_have_scales",
        json!(v.layers.iter().all(|l| (l.panel as usize) < v.y.len())),
    );
    m.insert(
        "candles_px_panel0",
        json!(v.layers.iter().any(|l| l.mark == Mark::Candle && l.series.as_deref() == Some("px") && l.panel == 0)),
    );
    let rsi_panels = panel_of(v, |l| l.series.as_deref() == Some("rsi14"));
    let price_panels: BTreeSet<u32> =
        panel_of(v, |l| matches!(l.series.as_deref(), Some("px" | "bb20"))).into_iter().collect();
    let rsi_own = !rsi_panels.is_empty()
        && rsi_panels.iter().all(|p| *p != 0 && !price_panels.contains(p));
    m.insert("rsi_in_own_panel", json!(rsi_own));
    let level_rule = v.layers.iter().any(|l| {
        l.series.is_none()
            && matches!(l.mark, Mark::Rule)
            && rsi_panels.contains(&l.panel)
            && l.encoding.get("y").and_then(|y| y.parse::<f64>().ok()) == Some(70.0)
    });
    m.insert("level70_rule_in_rsi_panel", json!(level_rule));
    // Fact bar marked: any constant layer whose x encodes the fact time or bar.
    let bar_mark = v.layers.iter().any(|l| {
        l.encoding.get("x").is_some_and(|x| {
            x.parse::<f64>().is_ok_and(|n| (n - FACT_T).abs() < 1.0 || (n - 150.0).abs() < 0.5)
        })
    });
    m.insert("fact_bar_marked", json!(bar_mark));
    let dom = v.x.domain;
    let dom_ok = match dom {
        None => true,
        Some([a, b]) => a < b && a <= FACT_T && b >= FACT_T && a >= FIRST_T - 4e10 && b <= LAST_T + 4e10,
    };
    m.insert("x_time", json!(v.x.kind == qp_schema::ScaleKind::Time));
    m.insert("x_domain_contains_fact_ms", json!(dom_ok));
    m.insert("x_domain", json!(dom));
    m.insert(
        "x_domain_reaches_last_bar",
        json!(dom.is_none_or(|[_, b]| b >= LAST_T)),
    );
    let extra: Vec<String> = v
        .layers
        .iter()
        .filter_map(|l| l.series.clone())
        .filter(|s| s != "px" && s != "rsi14")
        .collect();
    m.insert("extra_series", json!(extra));
    m.insert("panels", json!(v.y.len()));
    m.insert("layers", json!(v.layers.len()));
    m
}

fn structure(v: &View) -> String {
    let layers: Vec<String> = v
        .layers
        .iter()
        .map(|l| {
            let mut enc: Vec<&str> = l.encoding.keys().map(String::as_str).collect();
            enc.sort_unstable();
            format!("{:?}:{}:p{}:[{}]", l.mark, l.series.as_deref().unwrap_or("-"), l.panel, enc.join(","))
        })
        .collect();
    let ys: Vec<String> = v
        .y
        .iter()
        .map(|s| format!("{:?}{}", s.kind, if s.domain.is_some() { "*" } else { "" }))
        .collect();
    format!(
        "x={:?}{} y=[{}] title={} layers=[{}]",
        v.x.kind,
        if v.x.domain.is_some() { "*" } else { "" },
        ys.join(","),
        v.title.is_some(),
        layers.join(" ")
    )
}

fn put_view_accepted(v: &View) -> Value {
    let mut s = super::session();
    let cmd = json!({"type":"command","id":"c1","ops":[{"op":"put_view","view":v}]});
    let out = s.handle_raw("a", &cmd.to_string());
    let o = serde_json::to_value(&out[0]).unwrap();
    let msg = &o["msg"];
    if msg["type"] == "ack" {
        json!(msg["status"])
    } else {
        json!(format!("error: {}", msg["error"]["code"]))
    }
}

pub fn run(dir: &Path, doc: &Document) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt" || x == "json"))
        .collect();
    files.sort();
    let mut rows = Vec::new();
    let mut canon: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut structs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let group = name.split(['-', '_']).next().unwrap_or("").to_owned();
        let text = std::fs::read_to_string(f).unwrap();
        let (body, clean) = extract(&text);
        let mut row = json!({"file": name, "group": group, "format_clean": clean, "bytes": text.len()});
        match serde_json::from_str::<Value>(&body) {
            Err(e) => row["json"] = json!(format!("parse error: {e}")),
            Ok(val) => {
                row["json"] = json!("ok");
                let c = val.to_string(); // keys sorted (BTreeMap) → canonical
                canon.entry(group.clone()).or_default().insert(c);
                row["schema_valid"] = json!(match qp_schema::validate("View", &val) {
                    Ok(()) => Value::Bool(true),
                    Err(e) => json!(e),
                });
                match serde_json::from_value::<View>(val) {
                    Err(e) => row["typed"] = json!(format!("error: {e}")),
                    Ok(v) => {
                        row["typed"] = json!("ok");
                        row["put_view"] = put_view_accepted(&v);
                        row["semantics"] = json!(semantics(&v, doc));
                        let st = structure(&v);
                        structs.entry(group.clone()).or_default().insert(st.clone());
                        row["structure"] = json!(st);
                    }
                }
            }
        }
        println!("{row}");
        rows.push(row);
    }
    let summary: BTreeMap<String, Value> = canon
        .keys()
        .map(|g| {
            (
                g.clone(),
                json!({"distinct_canonical": canon[g].len(),
                       "distinct_structures": structs.get(g).map_or(0, BTreeSet::len),
                       "structures": structs.get(g)}),
            )
        })
        .collect();
    println!("{}", json!({"summary": summary}));
    let out = json!({"rows": rows, "summary": summary});
    std::fs::write(
        dir.join("eval.json"),
        serde_json::to_string_pretty(&out).unwrap() + "\n",
    )
    .unwrap();
}
