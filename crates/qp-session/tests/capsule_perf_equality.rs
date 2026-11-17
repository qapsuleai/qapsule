//! R3/perf: the capsule engine's performance fixes (S-D §6.3 proposals 1–5) change no
//! output. A fixed multi-instrument scenario (the `capsule_tick` pattern: bound and inline
//! series, licensed flow, readers with different entitlements and languages, audience
//! preferences, approvals, queries and late `hello`s) is run and every outbound line, the
//! log and the document are digested. The expected digests were recorded on the code
//! before the fixes (`r3/int` a81b5e5); the fixes must reproduce them byte for byte.
//! (R3/K1 changed the lines of the readers without every licensed flow on purpose: they
//! read the public world; see `bound_with_flow_and_readers_unchanged`.)

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fnv(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

const FNV0: u64 = 0xcbf2_9ce4_8422_2325;

/// Digest of a stream of outbound lines, the log and the document.
#[derive(Default)]
struct Dig {
    lines: u64,
    n: usize,
    bytes: usize,
    /// Per connection: (lines, bytes, digest) of its own lines.
    per: std::collections::BTreeMap<String, (usize, usize, u64)>,
    /// `QP_PERF_DUMP=<dir>`: every line also written there (for a byte diff).
    dump: Option<std::fs::File>,
}

impl Dig {
    fn new(name: &str) -> Self {
        let dump = std::env::var("QP_PERF_DUMP")
            .ok()
            .map(|d| std::fs::File::create(Path::new(&d).join(format!("{name}.lines"))).unwrap());
        Self {
            lines: FNV0,
            dump,
            ..Self::default()
        }
    }
    fn add(&mut self, ls: &[OutboundLine]) {
        for l in ls {
            let s = format!("{}|{}\n", l.conn, serde_json::to_string(&l.msg).unwrap());
            self.lines = fnv(s.as_bytes(), self.lines);
            let e = self.per.entry(l.conn.clone()).or_insert((0, 0, FNV0));
            e.0 += 1;
            e.1 += s.len();
            e.2 = fnv(s.as_bytes(), e.2);
            if let Some(f) = self.dump.as_mut() {
                std::io::Write::write_all(f, s.as_bytes()).unwrap();
            }
            self.n += 1;
            self.bytes += s.len();
        }
    }
    fn finish(&mut self, s: &Session) -> String {
        let log = serde_json::to_vec(s.log()).unwrap();
        let doc = serde_json::to_vec(s.document()).unwrap();
        if let Some(f) = self.dump.as_mut() {
            std::io::Write::write_all(f, &log).unwrap();
            std::io::Write::write_all(f, b"\n").unwrap();
            std::io::Write::write_all(f, &doc).unwrap();
        }
        let mut out = String::new();
        write!(
            out,
            "lines {} {} {:016x} log {} {:016x} doc {} {:016x}",
            self.n,
            self.bytes,
            self.lines,
            log.len(),
            fnv(&log, FNV0),
            doc.len(),
            fnv(&doc, FNV0)
        )
        .unwrap();
        out
    }

    /// Per connection: `conn lines bytes digest`, one per line.
    fn per_conn(&self) -> String {
        let mut out = String::new();
        for (c, (n, b, h)) in &self.per {
            writeln!(out, "{c} {n} {b} {h:016x}").unwrap();
        }
        out
    }
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

/// Revision `k` of `n` of bar `b` (the `capsule_tick` generator); `n` is `b` itself.
fn revision(b: &Bar, k: usize, n: usize) -> Bar {
    if k == n {
        return b.clone();
    }
    #[allow(clippy::cast_precision_loss)]
    let f = k as f64 / n as f64;
    let wiggle = if k.is_multiple_of(2) { 0.25 } else { -0.25 } * (b.h - b.l) * (1.0 - f);
    let c = (b.o + (b.c - b.o) * f + wiggle).clamp(b.l, b.h);
    Bar {
        t: b.t,
        o: b.o,
        h: b.o.max(c) + (b.h - b.o.max(b.c)).max(0.0) * f,
        l: b.o.min(c) - (b.o.min(b.c) - b.l).max(0.0) * f,
        c,
        v: b.v * f,
    }
}

const PER: usize = 3;

struct Scenario {
    n: usize,
    history: usize,
    total: usize,
    bound: bool,
    flow: usize,
}

fn init(sc: &Scenario, m: &[qp_source::SyntheticInstrument]) -> SessionInit {
    let mut series = Vec::new();
    for (i, inst) in m.iter().enumerate() {
        let id = format!("px{i:02}");
        let sym = &inst.symbol;
        if sc.bound {
            series.push(json!({"id":id,"instrument":{"symbol":sym},"interval":"1d",
                "source":{"kind":"bind","symbol":sym,"interval":"1d","window":{"last":10000},"feed":"bist"}}));
        } else {
            series.push(json!({"id":id,"instrument":{"symbol":sym},"interval":"1d",
                "source":{"kind":"inline"},"data":{"kind":"bars","bars":inst.bars[..sc.history]}}));
        }
    }
    let mut v = json!({
        "document": {"schema_version":"0","id":"doc-perf","title":"perf","series":series},
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"arkin","author":{"kind":"human","name":"Arkın"},"role":"owner"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
            {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"},
            {"id":"feed","author":{"kind":"human","name":"feed"},"role":"editor"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0
    });
    if sc.bound {
        let feeds: Vec<Value> = m
            .iter()
            .map(|i| json!({"feed":{"symbol":i.symbol,"interval":"1d","feed":"bist"},"bars":i.bars[..sc.history]}))
            .collect();
        // Bora: the bars of the even instruments only; Can: nothing; Ana: the bars and
        // the flow of the first instrument.
        let bora: Vec<String> = m
            .iter()
            .step_by(2)
            .map(|i| format!("bist/{}", i.symbol))
            .collect();
        let ana: Vec<String> = m
            .iter()
            .map(|i| format!("bist/{}", i.symbol))
            .chain(std::iter::once(format!("flow/{}", m[0].symbol)))
            .collect();
        v["data"] = json!({"feeds":feeds,"entitlements":{
            "host":["*"],"arkin":["*"],"feed":["*"],"ana":ana,"bora":bora,"can":[]}});
    }
    let ids: Vec<String> = (0..m.len()).map(|i| format!("px{i:02}")).collect();
    let flow: Vec<Value> = ids
        .iter()
        .take(sc.flow)
        .enumerate()
        .map(|(i, id)| json!({"series":id,"seed":i + 1}))
        .collect();
    v["capsules"] = json!({"series":ids,"flow":flow});
    serde_json::from_value(v).expect("init")
}

fn hello(s: &mut Session, conn: &str, who: &str, lang: Option<&str>, d: &mut Dig) {
    let mut surface = json!({"kind":"headless"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    d.add(&send(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    ));
}

fn queries(s: &mut Session, conn: &str, d: &mut Dig) {
    let doc = s.document().clone();
    for qv in [
        json!({"kind":"document"}),
        json!({"kind":"log"}),
        json!({"kind":"capsules"}),
        json!({"kind":"capsules","audience":true}),
        json!({"kind":"capsules","open":true,"limit":7}),
        json!({"kind":"ledger"}),
        json!({"kind":"ledger","by_kind":true}),
        json!({"kind":"describe"}),
        json!({"kind":"list","collection":"findings","full":true}),
        json!({"kind":"list","collection":"capsules","full":true}),
        json!({"kind":"present","finding":{"latest":true}}),
    ] {
        d.add(&send(s, conn, &json!({"type":"query","id":"q","query":qv})));
    }
    for c in doc.capsules.iter().rev().take(6) {
        for qv in [
            json!({"kind":"capsule","capsule":c.id}),
            json!({"kind":"present","finding":c.finding}),
            json!({"kind":"get","id":c.finding}),
        ] {
            d.add(&send(s, conn, &json!({"type":"query","id":"g","query":qv})));
        }
    }
}

/// Runs the scenario; returns its digest line and the per-connection digests.
#[allow(clippy::too_many_lines)]
fn run(name: &str, sc: &Scenario) -> (String, String) {
    let m = qp_source::synthetic_market(sc.n, sc.total, 1);
    let mut s = Session::new(init(sc, &m), &root()).unwrap();
    let mut d = Dig::new(name);
    hello(&mut s, "a", "arkin", None, &mut d);
    hello(&mut s, "b", "bora", None, &mut d);
    hello(&mut s, "c", "can", Some("en"), &mut d);
    hello(&mut s, "d", "ana", None, &mut d);
    hello(&mut s, "f", "feed", None, &mut d);
    let topics = [
        json!({"kind":"changes"}),
        json!({"kind":"capsules","all":true}),
        json!({"kind":"capsules"}),
        json!({"kind":"changes","ids":["px00","px01"]}),
    ];
    for c in ["a", "b", "c", "d"] {
        for (i, t) in topics.iter().enumerate() {
            d.add(&send(
                &mut s,
                c,
                &json!({"type":"subscribe","id":format!("s{i}"),"topic":t}),
            ));
        }
    }
    let mut prev: Vec<Bar> = m.iter().map(|i| i.bars[sc.history - 1].clone()).collect();
    let mut k = 0usize;
    for bar_i in sc.history..sc.total {
        for rev in 1..=PER {
            k += 1;
            for (i, inst) in m.iter().enumerate() {
                let b = revision(&inst.bars[bar_i], rev, PER);
                if sc.bound {
                    let change = if rev == 1 {
                        json!({"kind":"bar","bar":b})
                    } else {
                        let p = &prev[i];
                        let mut f = serde_json::Map::new();
                        for (key, a, z) in [
                            ("o", p.o, b.o),
                            ("h", p.h, b.h),
                            ("l", p.l, b.l),
                            ("c", p.c, b.c),
                            ("v", p.v, b.v),
                        ] {
                            #[allow(clippy::float_cmp)]
                            if a != z {
                                f.insert(key.to_owned(), json!(z));
                            }
                        }
                        json!({"kind":"patch","t":b.t,"fields":f})
                    };
                    let ev: DataEvent = serde_json::from_value(json!({
                        "feed":{"symbol":inst.symbol,"interval":"1d","feed":"bist"},
                        "seq":k,"change":change}))
                    .unwrap();
                    d.add(&s.data_event(&ev).unwrap());
                } else {
                    let op = if rev == 1 {
                        json!({"op":"append_bars","series":format!("px{i:02}"),"bars":[b]})
                    } else {
                        json!({"op":"update_last_bar","series":format!("px{i:02}"),"bar":b})
                    };
                    d.add(&send(
                        &mut s,
                        "f",
                        &json!({"type":"command","id":format!("t{k}.{i}"),"ops":[op]}),
                    ));
                }
                prev[i] = b;
            }
        }
        let step = bar_i - sc.history;
        if step == 5 {
            // A person's capsule preferences (other readers then get projected entries).
            d.add(&send(
                &mut s,
                "a",
                &json!({"type":"command","id":"aud","ops":[{"op":"set_audience","audience":{"group":"future","watchlist":[m[1].symbol],"positions":{(m[2].symbol.clone()):10.0}}}]}),
            ));
            d.add(&send(
                &mut s,
                "d",
                &json!({"type":"command","id":"sens","ops":[{"op":"set_sensitivity","sensitivity":"all"}]}),
            ));
        }
        if step == 9 {
            if let Some(c) = s.document().capsules.last().cloned() {
                d.add(&send(
                    &mut s,
                    "a",
                    &json!({"type":"command","id":"ap","ops":[{"op":"approve_finding","finding":c.finding}]}),
                ));
            }
            if let Some(c) = s.document().capsules.first().cloned() {
                d.add(&send(
                    &mut s,
                    "a",
                    &json!({"type":"command","id":"rj","ops":[{"op":"reject_finding","finding":c.finding}]}),
                ));
            }
        }
        if step == 12 {
            for c in ["a", "b", "c", "d"] {
                queries(&mut s, c, &mut d);
            }
            hello(&mut s, "e", "can", None, &mut d);
            hello(&mut s, "g", "bora", Some("en"), &mut d);
        }
    }
    for c in ["a", "b", "c", "d", "e", "g"] {
        queries(&mut s, c, &mut d);
    }
    hello(&mut s, "h", "ana", Some("en"), &mut d);
    hello(&mut s, "i", "arkin", None, &mut d);
    let all = d.finish(&s);
    (all, d.per_conn())
}

#[test]
fn bound_with_flow_and_readers_unchanged() {
    let (got, per) = run(
        "bound",
        &Scenario {
            n: 6,
            history: 100,
            total: 150,
            bound: true,
            flow: 3,
        },
    );
    println!("bound: {got}\n{per}");
    // R3/K1: the readers without every licensed flow (bora b/g, can c/e, ana d/h: ana
    // reads one of the three flows) read the public world, so their lines changed; the
    // fully entitled connections (arkin a/i, feed f), the log and the document are those
    // recorded before the perf fixes (a81b5e5) and before K1 (ee75ba3), byte for byte:
    // before K1 the whole line was
    // "lines 3023 21183036 923f2ecc80daa91b log 1177744 e38025b8f0d9a037 doc 661323 01db88a5bab1d57f".
    // (R4/T3, K42) Re-recorded on the K42 code (directional ledger key → ledger at birth,
    // importance and order; `validity` on items and glyphs; story condition/resolution;
    // negative-transition override). The cache paths are unchanged by K42; the R3
    // recordings above are kept for history. `rounds/R4/experiments/T3.md`.
    // (R4/FIX-E1 B2) Re-recorded: a closed story's sentence has no validity tail (T3
    // values: a "a 634 4965011 0bf79d3304bb3560", i "i 1 33382 3803d7e72efd9cce"; f
    // unchanged). `rounds/R4/experiments/FIX-E1.md`.
    // (RP3/M15) Re-recorded: the `describe` answer lists the new `viewer` topic (§22),
    // `,"viewer"` = 9 bytes in each of connection a's two `describe` answers (FIX-E1 value
    // "a 634 4940088 23725df69ce9e46b"); with `viewer` left out of `describe` every digest
    // was the FIX-E1 one (checked); log and document unchanged.
    for entitled in [
        "a 634 4940106 6e4d9bddee9ae5a9",
        "f 1 33828 64e687e9ac6e5803",
        "i 1 33374 3d07f785ac18cb32",
    ] {
        assert!(per.lines().any(|l| l == entitled), "{entitled}\n{per}");
    }
    // R3/K1b: the public world has its own sequence numbers (3017 → 3014 lines: an
    // entitled person's command the public world does not take leaves no entry there).
    assert_eq!(
        got,
        // R3/K1b: "lines 3014 21187649 d43ff5b9b8b70b19 log 1177744 e38025b8f0d9a037 doc 661323 01db88a5bab1d57f"
        // R4/T3 (K42): "lines 2996 21725039 712470eec960882f log 1209418 cc26cd5957963e33 doc 674891 d66cf7025043a379"
        // R4/FIX-E1 (B2) re-recorded:
        // "lines 2996 21664421 2ed2ac94d4a71aab log 1206570 e3754e6847abf125 doc 672043 92a079cb66258785"
        // RP3/M15: `describe` lists the `viewer` topic (10 answers x 9 bytes), see above.
        "lines 2996 21664511 64ee0975d39fcbb1 log 1206570 e3754e6847abf125 doc 672043 92a079cb66258785"
    );
}

#[test]
fn inline_unchanged() {
    let (got, per) = run(
        "inline",
        &Scenario {
            n: 4,
            history: 100,
            total: 130,
            bound: false,
            flow: 0,
        },
    );
    println!("inline: {got}\n{per}");
    assert_eq!(
        got,
        // R3 (a81b5e5): "lines 3747 12944829 9ae44424a6d56893 log 601548 3ef7a6effb39fab4 doc 282007 8a407794a0085f28"
        // R4/T3 (K42): "lines 3739 13164269 6ba3bbb9a84ac641 log 612798 3a99f85cceae652c doc 288051 fb03e8e6206af0bc"
        // R4/FIX-E1 (B2) re-recorded, see `bound_with_flow_and_readers_unchanged`:
        // "lines 3739 13109440 44c28111ab9e8d84 log 611848 8fa8658745f376d0 doc 287101 1c632884ddcf7d58"
        // RP3/M15: `describe` lists the `viewer` topic (10 answers x 9 bytes).
        "lines 3739 13109530 3ae3c1fc74746f8e log 611848 8fa8658745f376d0 doc 287101 1c632884ddcf7d58"
    );
}
