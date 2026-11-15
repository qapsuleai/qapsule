//! VERIFY 5 ön kontrolü: toplu = canlı düzeltmeli akış, bit düzeyinde (denetçinin kendi
//! verisi ve gürültüsü: rejimli sentetik piyasa, testlerdekinden farklı tohum ve düzeltme
//! biçimi — uç değerli ara sürümler, 0–6 düzeltme, son halin iki kez gönderilmesi).
use qp_facts::{default_detectors, run_batch, Detection, FlowPoint};
use qp_schema::Bar;
use verify_e1::{market, Lcg};

fn sig(v: &[(u64, Detection)]) -> Vec<String> {
    v.iter()
        .map(|(i, d)| {
            let ev: Vec<String> = d
                .evidence
                .iter()
                .map(|(k, x)| format!("{k}={:016x}", x.to_bits()))
                .collect();
            format!(
                "{i} {} {:?} {:?} {:?} {:?} {:016x} {} {:?} {:?}",
                d.code,
                d.kind,
                d.direction,
                d.time,
                d.edge,
                d.strength.to_bits(),
                ev.join(","),
                d.geometry,
                d.condition
            )
        })
        .collect()
}

fn noisy(b: &Bar, f: &FlowPoint, r: &mut Lcg) -> (Bar, FlowPoint) {
    let mut nb = b.clone();
    let k = match (r.f() * 4.0) as u32 {
        0 => 1.25,         // uç yukarı
        1 => 0.8,          // uç aşağı
        _ => 1.0 + (r.f() - 0.5) * 0.04,
    };
    nb.c = b.c * k;
    nb.o = b.o * (1.0 + (r.f() - 0.5) * 0.02);
    nb.h = nb.c.max(nb.o).max(b.h * (1.0 + r.f() * 0.05));
    nb.l = nb.c.min(nb.o).min(b.l * (1.0 - r.f() * 0.05));
    nb.v = b.v * r.f() * 8.0;
    let mut nf = f.clone();
    match (r.f() * 3.0) as u32 {
        0 => nf.values.clear(),
        1 => {
            for v in nf.values.values_mut() {
                *v *= 40.0 * (r.f() - 0.5);
            }
        }
        _ => {}
    }
    (nb, nf)
}

fn main() {
    let mut fails = 0;
    let mut total = 0usize;
    let mut cand_total = 0usize;
    for seed in [77u64, 2027] {
        for (sym, bars, flow) in market(10, 1500, seed) {
            for k in 0..default_detectors().len() {
                let mut det_b = default_detectors().remove(k);
                let mut det_s = default_detectors().remove(k);
                let id = det_b.id();
                let b = run_batch(det_b.as_mut(), &bars, Some(&flow));
                let mut r = Lcg(seed * 1_000_003 + k as u64 + sym.len() as u64);
                let mut out = Vec::new();
                for (i, (bar, f)) in bars.iter().zip(&flow).enumerate() {
                    let (nb, nf) = noisy(bar, f, &mut r);
                    cand_total += det_s.push(&nb, Some(&nf)).len();
                    for _ in 0..((r.f() * 7.0) as u32) {
                        let (nb, nf) = noisy(bar, f, &mut r);
                        cand_total += det_s.update_last(&nb, Some(&nf)).len();
                    }
                    // final, sometimes sent twice with noise in between
                    let mut last = det_s.update_last(bar, Some(f));
                    if r.f() < 0.2 {
                        let (nb, nf) = noisy(bar, f, &mut r);
                        det_s.update_last(&nb, Some(&nf));
                        last = det_s.update_last(bar, Some(f));
                    }
                    out.extend(last.into_iter().map(|d| (i as u64, d)));
                }
                total += b.len();
                if sig(&b) != sig(&out) {
                    fails += 1;
                    println!("MISMATCH seed {seed} {sym} {id}: batch {} vs revised {}", b.len(), out.len());
                }
            }
        }
    }
    println!("detections compared: {total}, provisional (candidate) detections seen: {cand_total}, mismatching (instrument,detector) pairs: {fails}");
}
