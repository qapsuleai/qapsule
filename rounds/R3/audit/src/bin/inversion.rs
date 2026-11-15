//! VERIFY 11 — denetçinin kendi geri çıkarma saldırısı (mühür, `qp_capsule::trust::Muhur`).
//!
//! Düzeneğin kâhin saldırısından farklı: saldırgan **diğer barların değerlerini bilmez**; bütün
//! ileti dizisini (her barda sessiz / "net alış" / "net satış") birlikte kullanan **tam Bayesçi
//! çok-bar saldırgan**dır. Mühürün iç durumu (son 5 gözlemin kırpılmış ikili düzeyleri) gizli
//! Markov durumu olarak modellenir; her barın gizli değişkeni `(düzey = floor(log2|net|), işaret)`.
//! Önsel, saldırganın da gördüğü **açık veriye** koşullu: `d = düzey − floor(log2 hacim)` ve
//! getirinin işareti (ayrı eğitim tohumlarından öğrenilir). Kural herkese açık (Kerckhoffs).
//!
//! Hedef bar `t` için sonsal: `t−1`'e kadar ileri süzgeç, `t`'de dallanma, sonra `L` bar boyunca
//! etiketli ileri süzgeç (sonraki iletilerin `t` hakkında söyledikleri). Ölçüler (bit):
//! - `KL(sonsal || önsel)` düzey+işaret üzerinde (noktasal bilgi kazancı),
//! - `H(önsel) − H(sonsal)`,
//! - %90 güven kümesindeki düzey sayısı önsel/sonsal (ham |net| aralığının log2 genişliği).
//!
//! Ayrıca (kontrol) diğer barlar **bilinirken** hedef düzeyi taranır: farklı döküm sayısı ≤ 7 mi.
//! Kullanım: inversion [hedef sayısı] [L]
use std::collections::HashMap;

use qp_capsule::trust::{Cell, Muhur, Params, ZERO_LEVEL};
use qp_capsule::{FlowObservation, TrustPath};
use qp_schema::Bar;
use qp_source::{synthetic_flow, synthetic_market};

const DMIN: i32 = -26;
const DMAX: i32 = 3;
const ZERO_CAT: i32 = DMAX + 1; // net == 0

fn vb(b: &Bar) -> i32 {
    if b.v > 0.0 {
        b.v.log2().floor() as i32
    } else {
        0
    }
}

fn rsign(prev: Option<f64>, c: f64) -> usize {
    match prev {
        Some(p) if c > p => 2,
        Some(p) if c < p => 0,
        _ => 1,
    }
}

/// Prior table: [ret sign][d index][up] counts.
struct Prior {
    t: Vec<Vec<[f64; 2]>>,
}

fn dcat(level: i32, v: i32) -> i32 {
    if level == ZERO_LEVEL {
        ZERO_CAT
    } else {
        (level - v).clamp(DMIN, DMAX)
    }
}

impl Prior {
    fn train(seeds: &[u64]) -> Self {
        let nd = (ZERO_CAT - DMIN + 1) as usize;
        let mut t = vec![vec![[0.5f64; 2]; nd]; 3];
        for &s in seeds {
            for (k, inst) in synthetic_market(10, 400, s).into_iter().enumerate() {
                let fl = synthetic_flow(&inst.bars, s * 100 + k as u64);
                let mut prev = None;
                for (b, r) in inst.bars.iter().zip(&fl) {
                    let net = r.values["net"];
                    let c = Cell::of(net);
                    let d = dcat(c.level, vb(b));
                    t[rsign(prev, b.c)][(d - DMIN) as usize][usize::from(c.up)] += 1.0;
                    prev = Some(b.c);
                }
            }
        }
        for row in &mut t {
            let z: f64 = row.iter().map(|x| x[0] + x[1]).sum();
            for x in row.iter_mut() {
                x[0] /= z;
                x[1] /= z;
            }
        }
        Self { t }
    }
    /// Hidden values with prior mass for a bar: (absolute level, up, p).
    fn support(&self, b: &Bar, prev: Option<f64>) -> Vec<(i32, bool, f64)> {
        let v = vb(b);
        let row = &self.t[rsign(prev, b.c)];
        let mut out = Vec::new();
        for (i, x) in row.iter().enumerate() {
            let d = DMIN + i as i32;
            let lvl = if d == ZERO_CAT { ZERO_LEVEL } else { v + d };
            for (u, p) in [(false, x[0]), (true, x[1])] {
                if p > 1e-7 {
                    out.push((lvl, u, p));
                }
            }
        }
        out
    }
}

/// Mühür state model (from the public rule): stored clipped levels, oldest first.
type State = Vec<i32>;
const P: Params = Params {
    window: 5,
    min_obs: 5,
    low: 2,
    loud: 2,
    epoch: 1,
};

fn reference(s: &State) -> Option<i32> {
    if s.len() >= P.min_obs {
        let mut v = s.clone();
        v.sort_unstable();
        Some(v[(v.len() - 1) / 2])
    } else {
        None
    }
}

/// (emitted symbol 0 silent / 1 buy / 2 sell, next state)
fn step(s: &State, level: i32, up: bool) -> (u8, State) {
    let r = reference(s);
    let emit = match r {
        Some(r) if level >= r + P.loud => {
            if up {
                1
            } else {
                2
            }
        }
        _ => 0,
    };
    let keep = r.map_or(level, |r| level.clamp(r - P.low, r + P.loud));
    let mut n = s.clone();
    n.push(keep);
    while n.len() > P.window {
        n.remove(0);
    }
    (emit, n)
}

fn prune<K: std::hash::Hash + Eq + Clone>(m: HashMap<K, f64>, cap: usize) -> HashMap<K, f64> {
    let z: f64 = m.values().sum();
    let mut v: Vec<(K, f64)> = m.into_iter().map(|(k, w)| (k, w / z)).filter(|x| x.1 > 1e-13).collect();
    if v.len() > cap {
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(cap);
    }
    v.into_iter().collect()
}

fn entropy(p: &[f64]) -> f64 {
    p.iter().filter(|x| **x > 0.0).map(|x| -x * x.log2()).sum()
}

fn cred90(p: &[f64]) -> usize {
    let mut v: Vec<f64> = p.to_vec();
    v.sort_by(|a, b| b.total_cmp(a));
    let mut acc = 0.0;
    for (i, x) in v.iter().enumerate() {
        acc += x;
        if acc >= 0.9 {
            return i + 1;
        }
    }
    v.len()
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() {
        return f64::NAN;
    }
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n_targets: usize = a.get(1).map_or(400, |x| x.parse().unwrap());
    let lag: usize = a.get(2).map_or(10, |x| x.parse().unwrap());
    let cap = 60_000;
    let prior = Prior::train(&[101, 102, 103, 104, 105, 106]);

    let mut kl_msg = Vec::new();
    let mut kl_sil = Vec::new();
    let mut dh = Vec::new();
    let mut narrow = Vec::new();
    let mut lvl_hit = 0usize;
    let mut prior_hit = 0usize;
    let mut targets = 0usize;
    let mut model_ok = true;
    let mut max_distinct = 0usize;

    'outer: for (k, inst) in synthetic_market(6, 300, 1).into_iter().enumerate() {
        let bars = &inst.bars;
        let fl = synthetic_flow(bars, 1000 + k as u64);
        let nets: Vec<f64> = fl.iter().map(|r| r.values["net"]).collect();
        // The real path's transcript.
        let mut path = Muhur::demo(Params::default());
        let mut obs = Vec::new();
        for (i, (b, net)) in bars.iter().zip(&nets).enumerate() {
            let o = FlowObservation {
                t: b.t,
                bar: i as u64,
                instrument: inst.symbol.clone(),
                values: [("net".to_owned(), *net)].into_iter().collect(),
            };
            let m = path.observe(&o, &[]);
            obs.push(match m.first() {
                None => 0u8,
                Some(m) if m.codes.iter().any(|c| c == "net_buy") => 1,
                Some(_) => 2,
            });
        }
        // Model check: my state model reproduces the transcript from the true levels.
        let mut s: State = Vec::new();
        for (i, net) in nets.iter().enumerate() {
            let c = Cell::of(*net);
            let (e, n) = step(&s, c.level, c.up);
            if e != obs[i] {
                model_ok = false;
            }
            s = n;
        }
        let prevs: Vec<Option<f64>> = (0..bars.len()).map(|i| (i > 0).then(|| bars[i - 1].c)).collect();
        let supp: Vec<Vec<(i32, bool, f64)>> = (0..bars.len()).map(|i| prior.support(&bars[i], prevs[i])).collect();

        // Forward filter, keeping alpha_{t-1} for every t.
        let mut alphas: Vec<HashMap<State, f64>> = Vec::with_capacity(bars.len() + 1);
        let mut alpha: HashMap<State, f64> = HashMap::from([(Vec::new(), 1.0)]);
        alphas.push(alpha.clone());
        for (i, sp) in supp.iter().enumerate() {
            let mut next: HashMap<State, f64> = HashMap::new();
            for (st, w) in &alpha {
                for &(l, u, p) in sp {
                    let (e, n) = step(st, l, u);
                    if e == obs[i] {
                        *next.entry(n).or_default() += w * p;
                    }
                }
            }
            alpha = prune(next, cap);
            alphas.push(alpha.clone());
        }

        // Targets: every 3rd bar from 20 to len - lag.
        let mut t = 20;
        while t + lag < bars.len() {
            let sp = &supp[t];
            // Branch on x_t, tagged by its index in sp.
            let mut tagged: HashMap<(State, usize), f64> = HashMap::new();
            for (st, w) in &alphas[t] {
                for (j, &(l, u, p)) in sp.iter().enumerate() {
                    let (e, n) = step(st, l, u);
                    if e == obs[t] {
                        *tagged.entry((n, j)).or_default() += w * p;
                    }
                }
            }
            tagged = prune(tagged, cap);
            for i in t + 1..=t + lag {
                let mut next: HashMap<(State, usize), f64> = HashMap::new();
                for ((st, j), w) in &tagged {
                    for &(l, u, p) in &supp[i] {
                        let (e, n) = step(st, l, u);
                        if e == obs[i] {
                            *next.entry((n, *j)).or_default() += w * p;
                        }
                    }
                }
                tagged = prune(next, cap);
            }
            let mut post = vec![0.0; sp.len()];
            for ((_, j), w) in &tagged {
                post[*j] += w;
            }
            let z: f64 = post.iter().sum();
            if z <= 0.0 {
                t += 3;
                continue;
            }
            for x in &mut post {
                *x /= z;
            }
            let pr: Vec<f64> = sp.iter().map(|x| x.2).collect();
            let zp: f64 = pr.iter().sum();
            let pr: Vec<f64> = pr.iter().map(|x| x / zp).collect();
            let kl: f64 = post
                .iter()
                .zip(&pr)
                .filter(|(q, _)| **q > 0.0)
                .map(|(q, p)| q * (q / p).log2())
                .sum();
            // Level-only marginals for the range narrowing.
            let mut lv_post: HashMap<i32, f64> = HashMap::new();
            let mut lv_pr: HashMap<i32, f64> = HashMap::new();
            for (j, x) in sp.iter().enumerate() {
                *lv_post.entry(x.0).or_default() += post[j];
                *lv_pr.entry(x.0).or_default() += pr[j];
            }
            let lp: Vec<f64> = lv_post.values().copied().collect();
            let lq: Vec<f64> = lv_pr.values().copied().collect();
            let c_post = cred90(&lp);
            let c_pr = cred90(&lq);
            narrow.push((c_pr as f64).log2() - (c_post as f64).log2());
            dh.push(entropy(&pr) - entropy(&post));
            let truth = Cell::of(nets[t]).level;
            let map = lv_post.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|x| *x.0).unwrap();
            if map == truth {
                lvl_hit += 1;
            }
            let pmap = lv_pr.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|x| *x.0).unwrap();
            if pmap == truth {
                prior_hit += 1;
            }
            if obs[t] == 0 {
                kl_sil.push(kl);
            } else {
                kl_msg.push(kl);
            }
            targets += 1;

            // Control: other bars known, scan the target's level.
            let mut seen = std::collections::BTreeSet::new();
            for lvl in -60..80 {
                for up in [false, true] {
                    let mut s: State = Vec::new();
                    let mut tr = Vec::new();
                    for (i, net) in nets.iter().enumerate() {
                        let (l, u) = if i == t { (lvl, up) } else { let c = Cell::of(*net); (c.level, c.up) };
                        let (e, n) = step(&s, l, u);
                        tr.push(e);
                        s = n;
                    }
                    seen.insert(tr);
                }
            }
            max_distinct = max_distinct.max(seen.len());

            t += 3;
            if targets >= n_targets {
                break 'outer;
            }
        }
    }
    let mut all: Vec<f64> = kl_msg.iter().chain(&kl_sil).copied().collect();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    println!("durum modeli gerçek dökümü yeniden üretiyor: {model_ok}");
    println!("hedef bar: {targets} (iletili {}, sessiz {}); L = {lag}", kl_msg.len(), kl_sil.len());
    println!(
        "KL(sonsal||önsel) bit — hepsi: ort {:.3}, medyan {:.3}, p90 {:.3}, p99 {:.3}, en çok {:.3}",
        mean(&all),
        pct(&mut all.clone(), 0.5),
        pct(&mut all.clone(), 0.9),
        pct(&mut all.clone(), 0.99),
        pct(&mut all, 1.0)
    );
    println!(
        "  iletili barlar: ort {:.3}, p90 {:.3}, en çok {:.3} | sessiz barlar: ort {:.3}, p90 {:.3}, en çok {:.3}",
        mean(&kl_msg),
        pct(&mut kl_msg.clone(), 0.9),
        pct(&mut kl_msg, 1.0),
        mean(&kl_sil),
        pct(&mut kl_sil.clone(), 0.9),
        pct(&mut kl_sil, 1.0)
    );
    println!(
        "H(önsel) − H(sonsal) bit: ort {:.3}, p90 {:.3}, en çok {:.3}",
        mean(&dh),
        pct(&mut dh.clone(), 0.9),
        pct(&mut dh, 1.0)
    );
    println!(
        "%90 güven kümesi daralması (log2 düzey sayısı oranı = |net| aralığının bit cinsinden daralması): ort {:.3}, p90 {:.3}, en çok {:.3}",
        mean(&narrow),
        pct(&mut narrow.clone(), 0.9),
        pct(&mut narrow, 1.0)
    );
    println!("en olası düzey = gerçek düzey: sonsal {lvl_hit}/{targets}, yalnız önsel (açık veri) {prior_hit}/{targets}");
    println!("kontrol (diğer barlar bilinirken hedef taraması): en çok farklı döküm {max_distinct} (sınır 7 → log2 7 = 2,81 bit)");
}
