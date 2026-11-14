//! RD1/T6e: deterministic presenter vs LLM-written grammar.
//!
//! `bench`    — presenter latency (alone and end to end through the session) and determinism.
//! `schema`   — print the generated `View` JSON Schema.
//! `eval DIR` — evaluate model outputs (`*.txt`/`*.json` in DIR) against schema and semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

use qp_schema::protocol::SessionInit;
use qp_schema::{Document, Finding, Layer, Mark, Scale, ScaleKind, Series, SeriesSource, View};
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn init() -> SessionInit {
    qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap()
}

fn doc() -> Document {
    qp_session::read_json(&root().join("fixtures/protocol/docs/thyao.json")).unwrap()
}

pub(crate) fn session() -> Session {
    let mut s = Session::new(init(), &root()).unwrap();
    let hello = json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}});
    let out = s.handle_raw("a", &hello.to_string());
    assert_eq!(out.len(), 1);
    s
}

// ----- verbatim copy of qp_session::present::presenter_view (it is pub(crate)) -----

fn find_series<'a>(doc: &'a Document, id: &str) -> Option<&'a Series> {
    doc.series.iter().find(|s| s.id == id)
}

fn bar_series<'a>(doc: &'a Document, id: &str) -> Option<&'a Series> {
    let mut cur = find_series(doc, id)?;
    for _ in 0..64 {
        if cur.bars().is_some() {
            return Some(cur);
        }
        match &cur.source {
            SeriesSource::Study { inputs, .. } => cur = find_series(doc, inputs.first()?)?,
            _ => return None,
        }
    }
    None
}

fn presenter_view(doc: &Document, finding: &Finding, id: &str) -> Result<View, String> {
    let fact_id = finding.facts.first().ok_or("no facts")?;
    let fact = doc.facts.iter().find(|f| &f.id == fact_id).ok_or("no fact")?;
    let bs = bar_series(doc, &fact.series).ok_or("no bars")?;
    let bars = bs.bars().unwrap_or_default();
    let last = bars.last().ok_or("no bars")?;
    let from = usize::try_from(fact.bar.saturating_sub(30))
        .unwrap_or(0)
        .min(bars.len() - 1);
    let mut layers = vec![Layer {
        mark: Mark::Candle,
        series: Some(bs.id.clone()),
        panel: 0,
        encoding: BTreeMap::new(),
    }];
    let mut panels = 1;
    if fact.series != bs.id {
        panels = 2;
        layers.push(Layer {
            mark: Mark::Line,
            series: Some(fact.series.clone()),
            panel: 1,
            encoding: BTreeMap::new(),
        });
        if let Some(level) = fact.evidence.get("level") {
            layers.push(Layer {
                mark: Mark::Rule,
                series: None,
                panel: 1,
                encoding: [("y".to_owned(), format!("{level}"))].into_iter().collect(),
            });
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let domain = [bars[from].t as f64, last.t as f64];
    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        x: Scale { kind: ScaleKind::Time, domain: Some(domain) },
        y: (0..panels).map(|_| Scale { kind: ScaleKind::Linear, domain: None }).collect(),
        layers,
    })
}

// ----- stats -----

fn stats(name: &str, mut ns: Vec<u128>) -> Value {
    ns.sort_unstable();
    let n = ns.len();
    let q = |p: f64| ns[((n as f64 - 1.0) * p).round() as usize];
    let mean = ns.iter().sum::<u128>() as f64 / n as f64;
    let v = json!({"name":name,"n":n,"min_ns":ns[0],"median_ns":q(0.5),"p90_ns":q(0.9),
        "p99_ns":q(0.99),"max_ns":ns[n-1],"mean_ns":mean.round()});
    println!("{v}");
    v
}

fn hash(b: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    h.write(b);
    h.finish()
}

fn bench(out_dir: &Path) {
    let d = doc();
    let f = d.findings[0].clone();
    let mut results = Vec::new();

    // 1. Presenter function alone (warm-up then N runs).
    let n = 100_000;
    for _ in 0..1000 {
        std::hint::black_box(presenter_view(&d, &f, "present-x").unwrap());
    }
    let mut ns = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        let v = presenter_view(std::hint::black_box(&d), &f, "present-x").unwrap();
        ns.push(t.elapsed().as_nanos());
        std::hint::black_box(v);
    }
    results.push(stats("presenter_fn", ns));

    // 1b. Presenter + serialization to JSON bytes (what goes on the wire).
    let mut ns = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        let v = presenter_view(&d, &f, "present-x").unwrap();
        let b = serde_json::to_vec(&v).unwrap();
        ns.push(t.elapsed().as_nanos());
        std::hint::black_box(b);
    }
    results.push(stats("presenter_fn+serialize", ns));

    // 2. show_finding end to end through the session (fresh session per run, built outside timer).
    let cmd = json!({"type":"command","id":"c1","ops":[{"op":"show_finding","finding":"fnd-overbought"}]})
        .to_string();
    let m = 3000;
    let mut sessions: Vec<Session> = (0..m).map(|_| session()).collect();
    let mut ns = Vec::with_capacity(m);
    let mut ack = Vec::new();
    for s in &mut sessions {
        let t = Instant::now();
        let out = s.handle_raw("a", &cmd);
        let lines: Vec<String> = out.iter().map(|l| serde_json::to_string(l).unwrap()).collect();
        ns.push(t.elapsed().as_nanos());
        ack = lines;
    }
    results.push(stats("show_finding_e2e(parse+auth+expand+apply+log+serialize out)", ns));
    std::fs::write(out_dir.join("show_finding-out.jsonl"), ack.join("\n") + "\n").unwrap();

    // 3. `present` query at chart depth (read path; presenter runs because capsule has no view).
    let q = json!({"type":"query","id":"q1","query":{"kind":"present","finding":"fnd-overbought"}})
        .to_string();
    let mut s = session();
    for _ in 0..1000 {
        std::hint::black_box(s.handle_raw("a", &q));
    }
    let mut ns = Vec::with_capacity(n);
    let mut last = String::new();
    for _ in 0..20_000 {
        let t = Instant::now();
        let out = s.handle_raw("a", &q);
        let line = serde_json::to_string(&out[0]).unwrap();
        ns.push(t.elapsed().as_nanos());
        last = line;
    }
    results.push(stats("present_query_chart_desktop_e2e", ns));
    std::fs::write(out_dir.join("present-query-out.json"), &last).unwrap();

    // 3b. Same for phone (capsule depth: no view, spark of 60 points).
    let mut s = session();
    let hello = json!({"type":"hello","protocol":"qp/0","principal":"viewer","surface":{"kind":"phone"}});
    s.handle_raw("p", &hello.to_string());
    let mut ns = Vec::new();
    let mut plast = String::new();
    for _ in 0..20_000 {
        let t = Instant::now();
        let out = s.handle_raw("p", &q);
        let line = serde_json::to_string(&out[0]).unwrap();
        ns.push(t.elapsed().as_nanos());
        plast = line;
    }
    results.push(stats("present_query_capsule_phone_e2e", ns));
    std::fs::write(out_dir.join("present-query-phone-out.json"), &plast).unwrap();

    // 4. Determinism: 1000 fresh sessions → show_finding → bytes of outputs + document.
    let mut hashes_out = BTreeSet::new();
    let mut hashes_doc = BTreeSet::new();
    let mut hashes_view = BTreeSet::new();
    let mut view_bytes = Vec::new();
    for _ in 0..1000 {
        let mut s = session();
        let out = s.handle_raw("a", &cmd);
        let o: Vec<u8> = out.iter().flat_map(|l| serde_json::to_vec(l).unwrap()).collect();
        hashes_out.insert(hash(&o));
        hashes_doc.insert(hash(&serde_json::to_vec(s.document()).unwrap()));
        let v = s.document().views.iter().find(|v| v.id == "view-1-0").unwrap();
        view_bytes = serde_json::to_vec(v).unwrap();
        hashes_view.insert(hash(&view_bytes));
    }
    // presenter fn alone 1000 times
    let fn_hashes: BTreeSet<u64> = (0..1000)
        .map(|_| hash(&serde_json::to_vec(&presenter_view(&d, &f, "view-1-0").unwrap()).unwrap()))
        .collect();
    // copy == session output
    let copy_eq = serde_json::to_vec(&presenter_view(&d, &f, "view-1-0").unwrap()).unwrap() == view_bytes;
    let pretty = serde_json::to_string_pretty(&serde_json::from_slice::<Value>(&view_bytes).unwrap()).unwrap();
    std::fs::write(out_dir.join("presenter-view.json"), format!("{pretty}\n")).unwrap();
    let det = json!({
        "runs": 1000,
        "distinct_outputs": hashes_out.len(),
        "distinct_documents": hashes_doc.len(),
        "distinct_views": hashes_views_len(&hashes_view),
        "distinct_views_fn_alone": fn_hashes.len(),
        "view_hash": format!("{:016x}", hashes_view.iter().next().unwrap()),
        "copy_equals_session_view": copy_eq,
        "view_bytes_compact": view_bytes.len(),
        "view_bytes_pretty": pretty.len() + 1,
        "show_finding_out_bytes": ack.iter().map(|l| l.len() + 1).sum::<usize>(),
        "present_query_desktop_bytes": last.len(),
        "present_query_phone_bytes": plast.len(),
    });
    println!("{det}");
    let all = json!({"latency": results, "determinism": det});
    std::fs::write(out_dir.join("bench.json"), serde_json::to_string_pretty(&all).unwrap() + "\n").unwrap();
}

fn hashes_views_len(s: &BTreeSet<u64>) -> usize {
    s.len()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = root().join("rounds/RD1/experiments/raw/t6e");
    std::fs::create_dir_all(&out_dir).unwrap();
    match args.get(1).map(String::as_str) {
        Some("bench") => bench(&out_dir),
        Some("schema") => {
            println!("{}", serde_json::to_string_pretty(&qp_schema::json_schema("View").unwrap()).unwrap());
        }
        Some("eval") => eval::run(Path::new(&args[2]), &doc()),
        _ => eprintln!("usage: t6e-presenter bench|schema|eval DIR"),
    }
}

mod eval;
