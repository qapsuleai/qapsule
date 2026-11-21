//! R4/T6 — the page's opening, natively (GOAL criterion 10): the demo host (`qp demo`'s
//! [`qp_wasm::shell::demo_session_init`]) and one [`ShellCore`] as a new page gets it,
//! every message timed on both sides (host `handle_raw`; core `on_frame` with the frame
//! text, as the browser hands it over), then the first face painted.
//!
//! ```text
//! cargo run --release -p qp-wasm --features replica --example startup -- \
//!     [synthetic | local DIR] [--from YYYY-MM-DD] [--history N] [--principal P]
//! ```
//!
//! Prints one line per message (`host ms`, `core ms`, bytes) and the totals; WASM in the
//! browser is slower by a roughly constant factor, the split is what this is for.

use std::path::{Path, PathBuf};
use std::time::Instant;

use qp_session::ReplayHost;
use qp_wasm::shell::{demo_session_init, DemoOptions, ShellCore};
use serde_json::json;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut local: Option<PathBuf> = None;
    let mut from = None;
    let mut history = 120;
    let mut principal = "arkin".to_owned();
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "synthetic" => {}
            "local" => {
                i += 1;
                local = Some(PathBuf::from(&a[i]));
            }
            "--from" => {
                i += 1;
                from = Some(qp_source::date_start(&a[i]).expect("--from"));
            }
            "--history" => {
                i += 1;
                history = a[i].parse().expect("--history");
            }
            "--principal" => {
                i += 1;
                principal.clone_from(&a[i]);
            }
            x => panic!("unknown argument {x}"),
        }
        i += 1;
    }
    let o = DemoOptions {
        local: local.is_some(),
        instruments: 30,
        history,
        days: 260,
        speed: 4.0,
        from_t: from,
        running: false,
        ..DemoOptions::default()
    };
    let t = Instant::now();
    let mut host = ReplayHost::with_window(
        demo_session_init(&o),
        Path::new("."),
        local.as_deref().unwrap_or(Path::new("")),
        Some(history as usize),
    )
    .expect("host");
    host.set_record_events(true);
    println!("host setup: {:.1} ms", ms(t));

    let t0 = Instant::now();
    let data = host.init().data.clone().expect("replay init has data");
    let start = json!({"data": data, "events": host.recorded_events()}).to_string();
    println!("start frame: {} B", start.len());
    let t = Instant::now();
    let mut data = data;
    data.entitlements.retain(|p, _| p == &principal);
    let mut core = ShellCore::new(&principal, "tr", &data, &host.recorded_events()).expect("core");
    println!("core new: {:.1} ms", ms(t));
    let (mut host_ms, mut core_ms, mut bytes) = (0.0, 0.0, 0usize);
    for round in 0..50 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            let t = Instant::now();
            let lines = host.handle_raw("w", &m.to_string());
            let h = ms(t);
            host_ms += h;
            for l in lines {
                if l.conn != "w" {
                    continue;
                }
                let text = json!({"msg": l.msg}).to_string();
                if std::env::var_os("QP_SPLIT").is_some() && text.len() > 1_000_000 {
                    // Where a large frame's time goes: parsing alone, typed, and a copy.
                    let t = Instant::now();
                    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
                    let pv = ms(t);
                    let inner = serde_json::to_string(&l.msg).expect("json");
                    let t = Instant::now();
                    let f: qp_schema::protocol::ServerMsg =
                        serde_json::from_str(&inner).expect("typed");
                    let pt = ms(t);
                    let t = Instant::now();
                    let c = f.clone();
                    let cl = ms(t);
                    drop((v, c));
                    // `QP_REPEAT=n`: the replica's load of this document n times (profile).
                    let n: usize = std::env::var("QP_REPEAT")
                        .ok()
                        .and_then(|x| x.parse().ok())
                        .unwrap_or(0);
                    for _ in 0..n {
                        let prov = qp_source::MemoryProvider::from_init(&data, Path::new("."))
                            .expect("provider");
                        let mut r = qp_session::Replica::with_memory(&principal, prov);
                        let t = Instant::now();
                        r.on_message(&f);
                        println!("  replica load {:.1} ms", ms(t));
                    }
                    println!(
                        "  split: parse Value {pv:.1} ms, parse typed {pt:.1} ms, clone {cl:.1} ms"
                    );
                }
                let t = Instant::now();
                let _ = core.on_frame(&text);
                let c = ms(t);
                core_ms += c;
                bytes += text.len();
                let kind = m["type"].as_str().unwrap_or("?");
                let q = m["query"]["kind"].as_str().unwrap_or("");
                println!(
                    "r{round} {kind} {q}: host {h:.1} ms, core {c:.1} ms, {} B",
                    text.len()
                );
            }
        }
    }
    if std::env::var_os("QP_SPLIT").is_some() {
        // The document's bytes by part (what the replica's `document` answer carries).
        let d = serde_json::to_value(host.session().document()).expect("doc");
        if let Some(o) = d.as_object() {
            let mut parts: Vec<(usize, &String, usize)> = o
                .iter()
                .map(|(k, v)| (v.to_string().len(), k, v.as_array().map_or(0, Vec::len)))
                .collect();
            parts.sort_unstable_by(|a, b| b.0.cmp(&a.0));
            for (n, k, c) in parts.iter().take(8) {
                println!("  doc.{k}: {n} B ({c} items)");
            }
        }
    }
    let feed = core.feed().to_vec();
    let t = Instant::now();
    let px = feed.first().map(|it| core.face_rgba(&it.id, true));
    let face = ms(t);
    println!(
        "feed {} items; first face {:.1} ms ({} px bytes)",
        feed.len(),
        face,
        px.map_or(0, |p| p.len())
    );
    println!(
        "total {:.1} ms (host {host_ms:.1}, core {core_ms:.1}, frames {bytes} B)",
        ms(t0)
    );
}
