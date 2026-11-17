//! RP3/T4 — memory of the web shell's core (`ShellCore`, full replica) over a long replay,
//! natively. The demo host (`qp demo --synthetic`: 30 instruments, 120 bars of history,
//! hourly steps) and one `ShellCore` as a page gets it; the host's frames are handed to
//! the core as text, as the browser does (`{"data":…,"replay":…}` and `{"msg":…}`).
//!
//! A tagging allocator attributes every allocation to the code that made it (the core or
//! the host / harness), wherever it is freed: each block carries a small header with its
//! tag. `live` = bytes held by the core now; `peak` = its high-water mark (WASM linear
//! memory never shrinks: it follows the peak plus fragmentation).
//!
//! ```text
//! cargo run --release -- STEPS EVERY [feed|chart|faces-all] [--dpr N] [--late N]
//!     [--start value|typed] [--burst N]
//! ```
//!
//! Prints one JSON line per checkpoint (every `EVERY` steps) with the core's live and peak
//! bytes and the sizes of its main structures (deep clones measured with the allocator).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};

use qp_schema::protocol::OutboundLine;
use qp_session::ReplayHost;
use qp_wasm::shell::{demo_session_init, DemoOptions, ShellCore};
use serde_json::json;

const HDR: usize = 16;
const TAGS: usize = 3;
/// 0: host / harness, 1: core, 2: measuring clones.
static LIVE: [AtomicI64; TAGS] = [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)];
static PEAK: [AtomicI64; TAGS] = [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)];

thread_local! {
    static TAG: Cell<u8> = const { Cell::new(0) };
}

struct Tagging;

unsafe impl GlobalAlloc for Tagging {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let align = l.align().max(HDR);
        let lay = Layout::from_size_align_unchecked(l.size() + align, align);
        let p = System.alloc(lay);
        if p.is_null() {
            return p;
        }
        let tag = TAG.try_with(Cell::get).unwrap_or(0);
        let user = p.add(align);
        *user.sub(1) = tag;
        let n = l.size() as i64;
        let now = LIVE[tag as usize].fetch_add(n, Ordering::Relaxed) + n;
        PEAK[tag as usize].fetch_max(now, Ordering::Relaxed);
        user
    }
    unsafe fn dealloc(&self, user: *mut u8, l: Layout) {
        let align = l.align().max(HDR);
        let tag = *user.sub(1);
        LIVE[tag as usize].fetch_sub(l.size() as i64, Ordering::Relaxed);
        let lay = Layout::from_size_align_unchecked(l.size() + align, align);
        System.dealloc(user.sub(align), lay);
    }
}

#[global_allocator]
static A: Tagging = Tagging;

fn with_tag<R>(t: u8, f: impl FnOnce() -> R) -> R {
    let old = TAG.with(|c| c.replace(t));
    let r = f();
    TAG.with(|c| c.set(old));
    r
}

/// Live bytes of a deep clone of `x` (tag 2), the clone dropped after.
fn size_of_clone<T: Clone>(x: &T) -> i64 {
    let before = LIVE[2].load(Ordering::Relaxed);
    let c = with_tag(2, || x.clone());
    let n = LIVE[2].load(Ordering::Relaxed) - before;
    drop(c);
    n
}

const CONN: &str = "w";

struct Run {
    host: ReplayHost,
    core: ShellCore,
    sent_events: usize,
}

impl Run {
    fn deliver(&mut self, lines: Vec<OutboundLine>) {
        for l in lines {
            if l.conn == CONN {
                let text = json!({"msg": l.msg}).to_string();
                let core = &mut self.core;
                with_tag(1, || {
                    let _ = core.on_frame(&text);
                });
            }
        }
    }

    /// Passes the core's queued messages to the host until it is idle.
    fn pump(&mut self) {
        for _ in 0..1000 {
            let core = &mut self.core;
            let out = with_tag(1, || core.take_outbox());
            if out.is_empty() {
                break;
            }
            for m in out {
                let lines = self.host.handle_raw(CONN, &m.to_string());
                self.deliver(lines);
            }
        }
    }

    /// The new data events as one `data` frame (as `qp demo`'s broadcast).
    fn broadcast(&mut self) {
        let evs = self.host.recorded_events();
        if evs.len() > self.sent_events {
            let frame = json!({"data": &evs[self.sent_events..], "replay": self.host.state()})
                .to_string();
            self.sent_events = evs.len();
            let core = &mut self.core;
            with_tag(1, || {
                let _ = core.on_frame(&frame);
            });
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let steps: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or(5000);
    let every: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(250);
    let variant = a.get(2).cloned().unwrap_or_else(|| "feed".to_owned());
    let dpr: f64 = a
        .iter()
        .position(|x| x == "--dpr")
        .and_then(|i| a.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);
    // Faces painted per step: the rows the phone list shows at once (390×844, ~5 faces).
    let visible: usize = if variant == "faces-all" { 1000 } else { 5 };

    let o = DemoOptions {
        days: 1000,
        ..DemoOptions::default()
    };
    let none = std::env::temp_dir().join("qp-t4-no-local-data");
    let mut host = ReplayHost::with_window(
        demo_session_init(&o),
        std::path::Path::new("."),
        &none,
        Some(o.history as usize),
    )
    .expect("host");
    host.set_record_events(true);
    // `--late N`: the page opens after the host has run N steps (a late join, or the new
    // core after a reconnect at step N).
    let late: u64 = a
        .iter()
        .position(|x| x == "--late")
        .and_then(|i| a.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    for _ in 0..late {
        let _ = host.take_step();
    }
    let mut data = host.init().data.clone().expect("init data");
    data.entitlements.retain(|p, _| p == "arkin");
    let evs = host.recorded_events().to_vec();
    // `--start value|typed`: the core made from the start frame's text as the web binding
    // does (`qp_wasm::Shell::new`: a JSON tree, then copies of its parts) or parsed typed
    // in one pass; prints the frame size and the core's peak while it is made.
    let start = a
        .iter()
        .position(|x| x == "--start")
        .and_then(|i| a.get(i + 1))
        .cloned();
    let core = if let Some(how) = start {
        let text = json!({"conn": "c1", "token": "t", "principal": "arkin", "lang": "tr",
            "data": data, "events": evs, "info": {}})
        .to_string();
        PEAK[1].store(LIVE[1].load(Ordering::Relaxed), Ordering::Relaxed);
        let before = LIVE[1].load(Ordering::Relaxed);
        let c = with_tag(1, || {
            if how == "value" {
                let v: serde_json::Value = serde_json::from_str(&text).expect("json");
                let data = serde_json::from_value(v["data"].clone()).expect("data");
                let events: Vec<qp_schema::data::DataEvent> =
                    serde_json::from_value(v["events"].clone()).unwrap_or_default();
                ShellCore::new("arkin", "tr", &data, &events).expect("core")
            } else {
                #[derive(serde::Deserialize)]
                struct Start {
                    data: qp_schema::protocol::InitData,
                    #[serde(default)]
                    events: Vec<qp_schema::data::DataEvent>,
                }
                let st: Start = serde_json::from_str(&text).expect("start");
                ShellCore::new("arkin", "tr", &st.data, &st.events).expect("core")
            }
        });
        eprintln!(
            "{}",
            json!({"start": how, "late": late, "frame_bytes": text.len(),
                "events": evs.len(), "core_after_new": LIVE[1].load(Ordering::Relaxed) - before,
                "peak_during_new": PEAK[1].load(Ordering::Relaxed) - before})
        );
        c
    } else {
        with_tag(1, || ShellCore::new("arkin", "tr", &data, &evs).expect("core"))
    };
    let core = with_tag(1, || {
        let mut c = core;
        c.set_dpr(dpr);
        c
    });
    let sent_events = host.recorded_events().len();
    let mut r = Run {
        host,
        core,
        sent_events,
    };
    r.pump();
    // `--burst N`: the host runs N steps at once (a forward seek), then one data frame;
    // prints the frame size and the core's peak while it takes the frame.
    if let Some(n) = a
        .iter()
        .position(|x| x == "--burst")
        .and_then(|i| a.get(i + 1))
        .and_then(|s| s.parse::<u64>().ok())
    {
        for _ in 0..n {
            if let Some((_, lines)) = r.host.take_step() {
                r.deliver(lines);
            }
        }
        let evs = r.host.recorded_events();
        let frame = json!({"data": &evs[r.sent_events..], "replay": r.host.state()}).to_string();
        r.sent_events = evs.len();
        PEAK[1].store(LIVE[1].load(Ordering::Relaxed), Ordering::Relaxed);
        let before = LIVE[1].load(Ordering::Relaxed);
        let core = &mut r.core;
        with_tag(1, || {
            let _ = core.on_frame(&frame);
        });
        eprintln!(
            "{}",
            json!({"burst": n, "frame_bytes": frame.len(),
                "peak_during_frame": PEAK[1].load(Ordering::Relaxed) - before})
        );
        r.pump();
    }
    checkpoint(&mut r, late);
    let mut opened = false;
    let mut t = 0.0f64;
    for step in 1..=steps {
        match r.host.take_step() {
            Some((_, lines)) => {
                r.deliver(lines);
            }
            None => {
                eprintln!("replay ended at step {step}");
                break;
            }
        }
        r.broadcast();
        r.pump();
        // Paint what a page paints after a tick: the visible faces (changed only), the
        // open scene; the page's metrics calls (a frame interval, the tick span).
        let core = &mut r.core;
        with_tag(1, || {
            let feed = core.feed_json();
            let ids: Vec<String> = feed
                .as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(|x| x["id"].as_str().map(str::to_owned))
                        .take(visible)
                        .collect()
                })
                .unwrap_or_default();
            for id in &ids {
                let _ = core.face_rgba(id, false);
            }
            if opened {
                let _ = core.scene_rgba(false);
            }
            for _ in 0..8 {
                t += 8.3;
                core.metric_frame("idle", t);
            }
            core.metric_span("tick", t, t + 2.0);
            core.metric_span("face", t, t + 1.0);
        });
        if variant == "chart" && !opened && step >= 10 {
            let first = with_tag(1, || {
                r.core.feed().first().map(|i| i.id.clone())
            });
            if let Some(id) = first {
                with_tag(1, || {
                    r.core.open_capsule(&id);
                });
                r.pump();
                with_tag(1, || r.core.open_to("chart"));
                r.pump();
                opened = true;
            }
        }
        if step % every == 0 || step == steps {
            checkpoint(&mut r, late + step);
        }
    }
}

fn checkpoint(r: &mut Run, step: u64) {
    let live = LIVE[1].load(Ordering::Relaxed);
    let peak = PEAK[1].load(Ordering::Relaxed);
    let host_live = LIVE[0].load(Ordering::Relaxed);
    let core = &mut r.core;
    let sent = core.sent().to_vec();
    let sent_n = sent.len();
    let sent_b = size_of_clone(&sent);
    let sent_json: usize = sent.iter().map(|m| m.to_string().len()).sum();
    drop(sent);
    let feed = core.feed().to_vec();
    let feed_b = size_of_clone(&feed);
    let feed_n = feed.len();
    drop(feed);
    let rep = core.replica_mut();
    let display_n = rep.display().len();
    let display_b = size_of_clone(&rep.display().to_vec());
    let mut out = json!({
        "step": step, "core_live": live, "core_peak": peak, "host_live": host_live,
        "sent_n": sent_n, "sent_bytes": sent_b, "sent_json": sent_json,
        "feed_n": feed_n, "feed_bytes": feed_b,
        "display_n": display_n, "display_bytes": display_b,
    });
    let series_ids: Vec<String> = rep
        .document()
        .map(|d| d.series.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();
    let mut bars = 0usize;
    for s in &series_ids {
        bars += rep.bars(s).map_or(0, <[_]>::len);
    }
    out["rd_bars"] = json!(bars);
    if let Some(s) = rep.session_mut() {
        let d = s.document().clone();
        out["doc_bytes"] = json!(size_of_clone(&d));
        out["doc_json"] = json!(serde_json::to_vec(&d).map_or(0, |v| v.len()));
        out["doc_series"] = json!(d.series.len());
        out["doc_facts"] = json!(d.facts.len());
        out["doc_findings"] = json!(d.findings.len());
        out["doc_capsules"] = json!(d.capsules.len());
        out["doc_ledger"] = json!(d.ledger.entries.len());
        out["doc_views"] = json!(d.views.len());
        let fj = |v: &dyn erased::Ser| v.json_len();
        out["doc_part_json"] = json!({
            "series": fj(&d.series), "views": fj(&d.views), "facts": fj(&d.facts),
            "findings": fj(&d.findings), "capsules": fj(&d.capsules),
            "ledger": fj(&d.ledger), "narratives": fj(&d.narratives),
        });
        drop(d);
        out["initial_bytes"] = json!(size_of_clone(s.initial_document()));
        out["log_n"] = json!(s.log().len());
        out["log_bytes"] = json!(size_of_clone(&s.log().to_vec()));
        // The log's bytes by part (live bytes of clones of each part).
        let (mut ch, mut inv, mut req, mut ev) = (0i64, 0i64, 0i64, 0i64);
        for e in s.log() {
            ch += size_of_clone(&e.changes);
            inv += size_of_clone(&e.inverse);
            req += size_of_clone(&e.request);
            ev += size_of_clone(&e.evidence);
        }
        out["log_parts"] = json!({"changes": ch, "inverse": inv, "request": req, "evidence": ev});
    }
    println!("{out}");
}

mod erased {
    pub trait Ser {
        fn json_len(&self) -> usize;
    }
    impl<T: serde::Serialize> Ser for T {
        fn json_len(&self) -> usize {
            serde_json::to_vec(self).map_or(0, |v| v.len())
        }
    }
}
