//! Ortak yardımcılar (denetçi).
use qp_facts::FlowPoint;
use qp_schema::Bar;
use qp_source::{synthetic_flow, synthetic_market, FlowRecord};

/// Sentetik piyasa + akış; akışın bir kısmı denetçi tarafından düşürülür
/// (her 17. barda nokta boş, her 29. barda `net` yok) — boşluk mantığını sınamak için.
pub fn market(n_inst: usize, n_days: usize, seed: u64) -> Vec<(String, Vec<Bar>, Vec<FlowPoint>)> {
    synthetic_market(n_inst, n_days, seed)
        .into_iter()
        .enumerate()
        .map(|(k, s)| {
            let fl: Vec<FlowRecord> = synthetic_flow(&s.bars, seed + 1000 + k as u64);
            let flow = fl
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    let mut values = r.values;
                    if i % 17 == 16 {
                        values.clear();
                    } else if i % 29 == 28 {
                        values.remove("net");
                    }
                    FlowPoint { t: r.t, values }
                })
                .collect();
            (s.symbol, s.bars, flow)
        })
        .collect()
}

/// Küçük deterministik LCG (denetçinin kendi gürültüsü).
pub struct Lcg(pub u64);
impl Lcg {
    pub fn f(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}
