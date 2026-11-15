//! VERIFY 8 — denetçinin kendi ileriye bakma testi (seçilen yapılandırma `EngineConfig::r3()`).
//!
//! Mevcut testlerden (`perturb_from`: kesimden sonra başka bir rastgele yürüyüş; `shuffle_after`:
//! kesimden sonraki barları karıştırma; verify-e1 karıştırmaları) farklı dört bozma:
//!
//! - `truncate`: kesimden sonrası hiç yok (motor geleceği hiç görmez).
//! - `crash_one`: yalnız **bir** enstrümanın geleceği çöker (×0,3 aşamalı, hacim ×20, akış ×1000):
//!   enstrümanlar arası sızıntı (sıralayıcının `ledger_all`, `same_kind_all`, `instruments`).
//! - `new_inst`: kesimden sonra ilk barı gelen **yeni** bir enstrüman eklenir (olgu üreten sert bir seri).
//! - `flow_flip`: yalnız gelecekteki akış işaret değiştirir ve 1e4 ile çarpılır; barlar aynı.
//!
//! Karşılaştırılan: `t < cut` olan bütün hikâye olayları (JSON bayt), `recorded_at < cut` olan
//! karne satırları, `t < cut` doğumların `ledger_at_birth`'ü, ve her anahtar için kesimden önceki
//! 6 zamanda `ledger_stat`. Ayrıca geleceğin gerçekten değiştiği.
//! Kullanım: lookahead <bars.json>
use std::collections::BTreeSet;

use qp_capsule::{simulate, EngineConfig, InstrumentInput, SimResult};
use qp_facts::default_detectors;
use qp_schema::capsule::Stage;
use qp_schema::Bar;
use r3_audit::{load, Lcg};

fn run(inp: &[InstrumentInput]) -> SimResult {
    simulate(inp, &default_detectors, EngineConfig::r3())
}

fn truncate(inp: &[InstrumentInput], cut: i64) -> Vec<InstrumentInput> {
    inp.iter()
        .map(|x| {
            let n = x.bars.iter().take_while(|b| b.t < cut).count();
            let mut y = x.clone();
            y.bars.truncate(n);
            if let Some(f) = y.flow.as_mut() {
                f.truncate(n);
            }
            y
        })
        .collect()
}

fn crash_one(inp: &[InstrumentInput], cut: i64) -> Vec<InstrumentInput> {
    let mut out = inp.to_vec();
    let x = &mut out[0];
    let mut k: f64 = 0.0;
    for (i, b) in x.bars.iter_mut().enumerate() {
        if b.t >= cut {
            k += 1.0;
            let m = (1.0 - 0.07 * k).max(0.3);
            b.o *= m;
            b.h *= m;
            b.l *= m * 0.98;
            b.c *= m * 0.97;
            b.v *= 20.0;
            if let Some(p) = x.flow.as_mut().and_then(|f| f.get_mut(i)) {
                for v in p.values.values_mut() {
                    *v *= 1000.0;
                }
            }
        }
    }
    out
}

fn new_inst(inp: &[InstrumentInput], cut: i64) -> Vec<InstrumentInput> {
    let mut out = inp.to_vec();
    let mut r = Lcg(99);
    let times: Vec<i64> = inp[0].bars.iter().map(|b| b.t).filter(|&t| t >= cut).collect();
    let mut c = 20.0;
    let mut bars = Vec::new();
    for (j, t) in times.iter().enumerate() {
        let o = c;
        c *= 1.0 + 0.08 * (r.u() - 0.45);
        let v = 1e5 * (1.0 + 9.0 * r.u()) * if j % 7 == 6 { 8.0 } else { 1.0 };
        bars.push(Bar {
            t: *t,
            o,
            h: o.max(c) * 1.01,
            l: o.min(c) * 0.99,
            c,
            v,
        });
    }
    let flow = bars
        .iter()
        .map(|b| qp_facts::FlowPoint {
            t: b.t,
            values: [("net".to_owned(), b.v * (r.u() - 0.5))].into_iter().collect(),
        })
        .collect();
    out.push(InstrumentInput {
        symbol: "ZZNEW".to_owned(),
        series: "ZZNEW.1d".to_owned(),
        bars,
        flow: Some(flow),
    });
    out
}

fn flow_flip(inp: &[InstrumentInput], cut: i64) -> Vec<InstrumentInput> {
    inp.iter()
        .map(|x| {
            let mut y = x.clone();
            if let Some(f) = y.flow.as_mut() {
                for p in f.iter_mut().filter(|p| p.t >= cut) {
                    for v in p.values.values_mut() {
                        *v *= -1e4;
                    }
                }
            }
            y
        })
        .collect()
}

fn js<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_string(x).unwrap()
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let base_in = load(&path);
    let base = run(&base_in);
    let n = base_in[0].bars.len();
    println!(
        "taban: {} enstrüman × {} bar; {} olay, {} karne satırı",
        base_in.len(),
        n,
        base.events.len(),
        base.records.len()
    );
    let mut fails = 0;
    for frac in [0.3, 0.55, 0.8] {
        let cut = base_in[0].bars[(n as f64 * frac) as usize].t;
        for (name, alt_in) in [
            ("truncate", truncate(&base_in, cut)),
            ("crash_one", crash_one(&base_in, cut)),
            ("new_inst", new_inst(&base_in, cut)),
            ("flow_flip", flow_flip(&base_in, cut)),
        ] {
            let alt = run(&alt_in);
            let past = |r: &SimResult| {
                js(&r
                    .events
                    .iter()
                    .filter(|e| e.transition.t < cut)
                    .collect::<Vec<_>>())
            };
            let rows = |r: &SimResult| {
                js(&r
                    .records
                    .iter()
                    .filter(|x| x.entry.recorded_at < cut)
                    .map(|x| &x.entry)
                    .collect::<Vec<_>>())
            };
            let births = |r: &SimResult| {
                r.events
                    .iter()
                    .filter(|e| e.transition.stage == Stage::Born && e.transition.t < cut)
                    .map(|e| (e.capsule.clone(), e.story.ledger_at_birth.n, e.story.ledger_at_birth.realized, e.story.tier))
                    .collect::<Vec<_>>()
            };
            let keys: BTreeSet<String> = base
                .events
                .iter()
                .chain(&alt.events)
                .map(|e| e.story.ledger_key.clone())
                .collect();
            let times: Vec<i64> = (1..=6)
                .map(|k| base_in[0].bars[((n as f64 * frac) as usize) * k / 6].t)
                .collect();
            let stats = |r: &SimResult| {
                let mut v = Vec::new();
                for &t in &times {
                    for k in &keys {
                        let s = r.ledger_stat(k, t);
                        v.push((s.n, s.realized));
                    }
                }
                v
            };
            let n_past = base.events.iter().filter(|e| e.transition.t < cut).count();
            let ok_ev = past(&base) == past(&alt);
            let ok_rows = rows(&base) == rows(&alt);
            let ok_b = births(&base) == births(&alt);
            let ok_s = stats(&base) == stats(&alt);
            let changed = js(&base.events) != js(&alt.events);
            let all = ok_ev && ok_rows && ok_b && ok_s && changed;
            if !all {
                fails += 1;
            }
            println!(
                "kesim {:.2} {:10} geçmiş olay {:5} eşit={} | satır eşit={} | doğum karnesi eşit={} | ledger_stat eşit={} | gelecek değişti={}",
                frac, name, n_past, ok_ev, ok_rows, ok_b, ok_s, changed
            );
        }
    }
    println!("SONUÇ: {}", if fails == 0 { "0 fark (PASS)" } else { "FARK VAR" });
}
