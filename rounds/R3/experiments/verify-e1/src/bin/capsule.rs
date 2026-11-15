//! Denetçinin kapsül motoru kontrolleri (gerçek qp-facts dedektörleriyle):
//!  A) ileriye bakma — dört ayrı karıştırma (geleceği kes, geleceği uç barlarla değiştir,
//!     yalnız gelecekteki akışı değiştir, kesip sonuna uç bar ekle); iskelet yapılandırma ve
//!     denetçinin barlara bakan kendi sonuç kuralı + karneye bakan kendi önem kuralı;
//!  B) aday saflığı — her barda gürültülü açık barla `candidates` çağrılır; durum, olaylar,
//!     karne değişmez; tüm koşu `simulate` ile bayt eşit;
//!  C) geri çekme — karneye satır yazmaz; önceki doğumların karnesi değişmez.
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use qp_capsule::engine::{Engine, EngineConfig};
use qp_capsule::scaffold::{self, KindTheme};
use qp_capsule::{simulate, InstrumentInput, OpenStory, OutcomeRule, RankCtx, Ranker, SimResult, Verdict};
use qp_facts::{default_detectors, to_fact, Detector, FlowPoint};
use qp_schema::capsule::{Direction, MeaningKind, Stage, TimeKind};
use qp_schema::Bar;
use verify_e1::{market, Lcg};

const SERIES: &str = "1d";
static LEAKS: AtomicUsize = AtomicUsize::new(0);
static JUDGED: AtomicUsize = AtomicUsize::new(0);

/// Denetçinin sonuç kuralı: doğum kapanışına göre ±%3 → gerçekleşti / geçersiz; ufuk 10.
/// `bars.len() != now + 1` görülürse sızıntı sayılır.
struct MoveJudge;
impl OutcomeRule for MoveJudge {
    fn name(&self) -> &'static str {
        "auditor.move_judge"
    }
    fn horizon(&self, _k: MeaningKind, _t: TimeKind) -> u32 {
        10
    }
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        JUDGED.fetch_add(1, Ordering::Relaxed);
        if bars.len() as u64 != now + 1 {
            LEAKS.fetch_add(1, Ordering::Relaxed);
        }
        let b0 = bars[s.story.born_bar as usize].c;
        let m = bars[now as usize].c / b0 - 1.0;
        let m = match s.story.direction {
            Direction::Up => m,
            Direction::Down => -m,
            Direction::Neutral => m.abs(),
        };
        if m >= 0.03 {
            Some((Verdict::Realized, "auditor_up"))
        } else if m <= -0.03 {
            Some((Verdict::Invalidated, "auditor_down"))
        } else {
            None
        }
    }
}

/// Denetçinin önem kuralı: karneye ve sayaçlara duyarlı (karnede sızıntı olaya yansısın).
struct LedgerRank;
impl Ranker for LedgerRank {
    fn name(&self) -> &'static str {
        "auditor.ledger_rank"
    }
    fn rank(&self, _s: &OpenStory, c: &RankCtx) -> (f64, u8) {
        let p = f64::from(c.ledger.realized + 1) / f64::from(c.ledger.n + 2);
        let imp = p + f64::from(c.ledger_all.n) * 1e-6 + f64::from(c.same_kind_all) * 1e-9
            + f64::from(c.stories_all) * 1e-12;
        (imp, if p > 0.5 { 1 } else if p > 0.3 { 2 } else { 3 })
    }
}

fn cfg(kind: &str) -> EngineConfig {
    match kind {
        "scaffold" => scaffold::config(10),
        _ => EngineConfig {
            theme: Box::new(KindTheme),
            outcome: Box::new(MoveJudge),
            ranker: Box::new(LedgerRank),
        },
    }
}

fn inputs(seed: u64, n: usize, days: usize) -> Vec<InstrumentInput> {
    market(n, days, seed)
        .into_iter()
        .map(|(symbol, bars, flow)| InstrumentInput {
            symbol,
            series: SERIES.to_owned(),
            bars,
            flow: Some(flow),
        })
        .collect()
}

fn j<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_string(x).unwrap()
}

fn dets() -> Vec<Box<dyn Detector>> {
    default_detectors()
}

fn past_view(r: &SimResult, cut: i64) -> Vec<String> {
    let mut v = Vec::new();
    v.push(j(&r.facts.iter().filter(|f| f.t < cut).collect::<Vec<_>>()));
    v.push(j(&r.events.iter().filter(|e| e.transition.t < cut).collect::<Vec<_>>()));
    v.push(j(&r
        .events
        .iter()
        .filter(|e| e.transition.stage == Stage::Born && e.transition.t < cut)
        .map(|e| (&e.capsule, e.story.ledger_at_birth))
        .collect::<Vec<_>>()));
    v.push(j(&r
        .records
        .iter()
        .filter(|x| x.entry.recorded_at < cut)
        .map(|x| &x.entry)
        .collect::<Vec<_>>()));
    let keys: BTreeSet<String> = r.events.iter().map(|e| e.story.ledger_key.clone()).collect();
    // anahtar kümesi geleceğe göre değişebilir (yalnız gelecekte görülen tür); n=0 olanlar atlanır
    for k in keys {
        let st = r.ledger_stat(&k, cut);
        if st.n > 0 {
            v.push(format!("{k}:{st:?}"));
        }
    }
    v
}

fn lookahead() {
    println!("== A) look-ahead ==");
    let base = inputs(31337, 5, 700);
    for kind in ["scaffold", "judge"] {
        let r0 = simulate(&base, &dets, cfg(kind));
        let stages: BTreeSet<String> = r0.events.iter().map(|e| format!("{:?}", e.transition.stage)).collect();
        let realized = r0.records.iter().filter(|x| x.entry.outcome == qp_schema::Outcome::Confirmed).count();
        println!("[{kind}] base: facts {} events {} ledger rows {} (realized {realized}) stages {:?}",
            r0.facts.len(), r0.events.len(), r0.records.len(), stages);
        for cut_bar in [150usize, 333, 520, 690] {
            let cut = base[0].bars[cut_bar].t;
            let mut results = Vec::new();
            // p1: kes
            let p1: Vec<_> = base.iter().map(|x| InstrumentInput {
                bars: x.bars.iter().filter(|b| b.t < cut).cloned().collect(),
                flow: x.flow.as_ref().map(|f| f.iter().filter(|p| p.t < cut).cloned().collect()),
                ..x.clone()
            }).collect();
            // p2: gelecekteki barları uç barlarla değiştir
            let p2: Vec<_> = base.iter().enumerate().map(|(k, x)| {
                let mut r = Lcg(99 + k as u64);
                let mut bars = x.bars.clone();
                let mut c = 1.0;
                for b in bars.iter_mut().filter(|b| b.t >= cut) {
                    c = if r.f() < 0.5 { c * 3.0 } else { c / 2.5 };
                    let p = b.c * c;
                    b.o = p; b.h = p * 1.5; b.l = p * 0.5; b.c = p * (0.6 + 0.8 * r.f()); b.v *= 1000.0;
                }
                InstrumentInput { bars, ..x.clone() }
            }).collect();
            // p3: yalnız gelecekteki akış
            let p3: Vec<_> = base.iter().enumerate().map(|(k, x)| {
                let mut r = Lcg(7 + k as u64);
                let flow = x.flow.as_ref().map(|f| f.iter().map(|p| {
                    let mut p: FlowPoint = p.clone();
                    if p.t >= cut {
                        for v in p.values.values_mut() { *v *= -100.0 * r.f(); }
                    }
                    p
                }).collect());
                InstrumentInput { flow, ..x.clone() }
            }).collect();
            // p4: kes ve sonuna 60 uç bar ekle
            let p4: Vec<_> = p1.iter().enumerate().map(|(k, x)| {
                let mut r = Lcg(1234 + k as u64);
                let mut bars = x.bars.clone();
                let mut flow = x.flow.clone().unwrap();
                let last = bars.last().unwrap().clone();
                for q in 0..60i64 {
                    let p = last.c * (if q % 2 == 0 { 4.0 } else { 0.3 }) * (0.9 + 0.2 * r.f());
                    let t = cut + q * 86_400_000;
                    bars.push(Bar { t, o: p, h: p * 1.2, l: p * 0.8, c: p, v: last.v * 500.0 });
                    let mut values = std::collections::BTreeMap::new();
                    values.insert("net".to_owned(), 1e12 * (r.f() - 0.5));
                    flow.push(FlowPoint { t, values });
                }
                InstrumentInput { bars, flow: Some(flow), ..x.clone() }
            }).collect();
            let pv0 = past_view(&r0, cut);
            for (name, p) in [("truncate", &p1), ("extreme_bars", &p2), ("future_flow", &p3), ("truncate+append", &p4)] {
                let r1 = simulate(p, &dets, cfg(kind));
                let effective = j(&r0.events) != j(&r1.events) || j(&r0.facts) != j(&r1.facts);
                let pv1 = past_view(&r1, cut);
                let same = pv1 == pv0;
                if !same && std::env::var("VERBOSE").is_ok() {
                    for (a, b) in pv0.iter().zip(&pv1) {
                        if a != b {
                            let ix = a.bytes().zip(b.bytes()).position(|(x, y)| x != y).unwrap_or(0);
                            let lo = ix.saturating_sub(300);
                            println!("    DIFF {name} at byte {ix}:\n     base: {}\n     pert: {}", &a[lo..(ix + 200).min(a.len())], &b[lo..(ix + 200).min(b.len())]);
                            break;
                        }
                    }
                }
                results.push(format!("{name}:{}{}", if same { "same" } else { "DIFF" }, if effective { "" } else { "(no-op!)" }));
            }
            println!("  cut bar {cut_bar}: {}", results.join(" "));
        }
    }
    println!("judge calls {} leaks (bars.len()!=now+1) {}", JUDGED.load(Ordering::Relaxed), LEAKS.load(Ordering::Relaxed));
}

/// Global order like `simulate`.
fn order(inp: &[InstrumentInput]) -> Vec<(i64, String, usize, usize)> {
    let mut o = Vec::new();
    for (k, x) in inp.iter().enumerate() {
        for (i, b) in x.bars.iter().enumerate() {
            o.push((b.t, x.symbol.clone(), k, i));
        }
    }
    o.sort();
    o
}

fn candidates_pure() {
    println!("== B) candidates purity ==");
    let inp = inputs(4711, 4, 600);
    let reference = qp_capsule::pipeline::simulate_engine(&inp, &dets, cfg("judge")).0;
    let mut eng = Engine::new(cfg("judge"));
    let mut ds: Vec<Vec<Box<dyn Detector>>> = inp.iter().map(|_| dets()).collect();
    let mut r = Lcg(5);
    let (mut calls, mut cands, mut snap_checks, mut snap_diffs) = (0usize, 0usize, 0usize, 0usize);
    let mut stage_counts = std::collections::BTreeMap::new();
    for (n, (t, sym, k, i)) in order(&inp).into_iter().enumerate() {
        let x = &inp[k];
        let bar = &x.bars[i];
        let fl = x.flow.as_ref().map(|f| &f[i]);
        // 1–3 açık bar sürümü: aday hesapla
        for rev in 0..(1 + (r.f() * 3.0) as usize) {
            let mut nb = bar.clone();
            let m = if r.f() < 0.5 { 1.0 + 0.15 * r.f() } else { 1.0 - 0.15 * r.f() };
            nb.c *= m; nb.h = nb.h.max(nb.c); nb.l = nb.l.min(nb.c); nb.v *= 1.0 + 6.0 * r.f();
            let mut prov = Vec::new();
            for d in &mut ds[k] {
                let out = if rev == 0 { d.push(&nb, fl) } else { d.update_last(&nb, fl) };
                prov.extend(out.iter().map(|x| to_fact(d.as_ref(), x, &sym, SERIES, i as u64, t, "v")));
            }
            let mut bars: Vec<Bar> = x.bars[..i].to_vec();
            bars.push(nb);
            let before = if n % 20 == 0 { Some(j(&eng.snapshot())) } else { None };
            let c = eng.candidates(&sym, SERIES, i as u64, &bars, &prov);
            for cs in &c {
                *stage_counts.entry(format!("{:?}", cs.transition.stage)).or_insert(0usize) += 1;
                assert_eq!(cs.story.stage, Stage::Candidate);
            }
            cands += c.len();
            calls += 1;
            if let Some(b) = before {
                snap_checks += 1;
                if b != j(&eng.snapshot()) { snap_diffs += 1; }
            }
        }
        // kapanış: son hal
        let mut facts = Vec::new();
        for d in &mut ds[k] {
            for x in d.update_last(bar, fl) {
                facts.push(to_fact(d.as_ref(), &x, &sym, SERIES, i as u64, t, qp_capsule::pipeline::ENGINE_VERSION));
            }
        }
        eng.on_bar(&sym, SERIES, i as u64, &x.bars[..=i], &facts);
    }
    let same = j(&eng.snapshot()) == j(&reference.snapshot());
    println!("candidates() calls {calls}, candidate stories returned {cands} by would-be stage {stage_counts:?}");
    println!("snapshot before==after checks {snap_checks}, differing {snap_diffs}; full run with candidates == simulate_engine without: {same}");
    println!("ledger rows: with-candidates {} reference {}; events {} vs {}", eng.ledger_records().len(), reference.ledger_records().len(), eng.events().len(), reference.events().len());
}

fn retraction() {
    println!("== C) retraction ==");
    let inp = inputs(8086, 5, 600);
    let ord = order(&inp);
    let run = |retract_at: Option<usize>| -> (Engine, Vec<String>, Option<i64>) {
        let mut eng = Engine::new(cfg("judge"));
        let mut ds: Vec<Vec<Box<dyn Detector>>> = inp.iter().map(|_| dets()).collect();
        let mut retracted = Vec::new();
        let mut tr = None;
        for (n, (t, sym, k, i)) in ord.iter().cloned().enumerate() {
            let x = &inp[k];
            let fl = x.flow.as_ref().map(|f| &f[i]);
            let mut facts = Vec::new();
            for d in &mut ds[k] {
                for y in d.push(&x.bars[i], fl) {
                    facts.push(to_fact(d.as_ref(), &y, &sym, SERIES, i as u64, t, "v"));
                }
            }
            eng.on_bar(&sym, SERIES, i as u64, &x.bars[..=i], &facts);
            if Some(n) == retract_at {
                let rows_before = eng.ledger_records().len();
                // en eski iki kapanmış (biri gerçekleşmiş) ve iki açık hikâye
                let mut ids: Vec<String> = Vec::new();
                if let Some(s) = eng.closed_stories().iter().find(|s| s.story.stage == Stage::Realized) { ids.push(s.id.clone()); }
                if let Some(s) = eng.closed_stories().iter().find(|s| s.story.stage == Stage::Invalidated) { ids.push(s.id.clone()); }
                for s in eng.open_stories().iter().take(2) { ids.push(s.id.clone()); }
                for id in &ids {
                    let ev = eng.retract(id, i as u64, t, "auditor_data_corrected");
                    assert!(ev.is_some(), "retract {id}");
                }
                // aynı hikâyeyi ikinci kez geri çekmek reddedilmeli
                let again = eng.retract(&ids[0], i as u64, t, "again").is_none();
                // geçmişe dönük geri çekme reddedilmeli (t < son geçiş)
                println!("retracted {} stories at t={t} (n={n}); rows before {rows_before} after {}; second retract refused: {again}",
                    ids.len(), eng.ledger_records().len());
                retracted = ids;
                tr = Some(t);
            }
        }
        (eng, retracted, tr)
    };
    let (base, _, _) = run(None);
    let mid = ord.len() / 2;
    let (eng, ids, tr) = run(Some(mid));
    let tr = tr.unwrap();
    let idset: BTreeSet<&String> = ids.iter().collect();
    // 1) geri çekilen hikâye için karnede Retracted satırı yok; açıkken geri çekilenlerin satırı hiç yok
    let retracted_rows = eng.ledger_records().iter().filter(|r| r.entry.stage == Some(Stage::Retracted)).count();
    let rows_for_ids: Vec<_> = eng.ledger_records().iter().filter(|r| r.entry.capsule.as_ref().is_some_and(|c| idset.contains(c))).map(|r| (r.entry.capsule.clone(), r.retracted_at)).collect();
    let in_ledger_view = eng.ledger().iter().filter(|e| e.capsule.as_ref().is_some_and(|c| idset.contains(c))).count();
    println!("ledger rows with stage Retracted: {retracted_rows}; rows of retracted capsules (capsule, retracted_at): {rows_for_ids:?}; in ledger() view: {in_ledger_view}");
    // 2) toplam satır sayısı: geri çekmesiz koşuya göre fark yalnız açıkken geri çekilenlerin olmayan kapanışı kadar
    println!("total ledger rows: base {} vs with retraction {}", base.ledger_records().len(), eng.ledger_records().len());
    // 3) geri çekmeden önceki doğumların ledger_at_birth'i: temel koşuyla aynı ve son kayıtlardan yeniden hesaplanınca aynı
    let births = |e: &Engine| -> Vec<(String, u32, u32, i64, String)> {
        e.events().iter().filter(|x| x.transition.stage == Stage::Born && x.transition.t < tr)
            .map(|x| (x.capsule.clone(), x.story.ledger_at_birth.n, x.story.ledger_at_birth.realized, x.transition.t, x.story.ledger_key.clone())).collect()
    };
    let (b0, b1) = (births(&base), births(&eng));
    let mut recompute_bad = 0;
    for (_, n, rz, t, key) in &b1 {
        let st = eng.ledger_stat(key, *t);
        if st.n != *n || st.realized != *rz { recompute_bad += 1; }
    }
    println!("births before retraction: {} ; identical to no-retraction run: {} ; recomputed from final ledger records mismatches: {recompute_bad}", b1.len(), b0 == b1);
    // 4) sonraki doğumlar geri çekilen satırı görmez
    let after = |e: &Engine| -> Vec<(String, u32)> {
        e.events().iter().filter(|x| x.transition.stage == Stage::Born && x.transition.t > tr)
            .map(|x| (x.capsule.clone(), x.story.ledger_at_birth.n)).collect()
    };
    let (a0, a1) = (after(&base), after(&eng));
    let common: Vec<_> = a0.iter().zip(&a1).filter(|(x, y)| x.0 == y.0).collect();
    let lower = common.iter().filter(|(x, y)| y.1 < x.1).count();
    let higher = common.iter().filter(|(x, y)| y.1 > x.1).count();
    println!("births after retraction (aligned {}): ledger_at_birth.n lower than base {lower}, higher {higher}", common.len());
    let stage_retracted_events = eng.events().iter().filter(|e| e.transition.stage == Stage::Retracted).count();
    println!("Retracted events in history: {stage_retracted_events}");
}

fn main() {
    lookahead();
    candidates_pure();
    retraction();
}
