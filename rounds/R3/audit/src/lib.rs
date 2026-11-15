//! Ortak yardımcılar (R3 son denetim).
use std::collections::BTreeMap;

use qp_capsule::InstrumentInput;
use qp_facts::FlowPoint;
use qp_schema::Bar;
use serde_json::Value;

/// `gen_bars.py` çıktısını okur.
pub fn load(path: &str) -> Vec<InstrumentInput> {
    let inp: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    inp.as_array()
        .unwrap()
        .iter()
        .map(|inst| {
            let bars: Vec<Bar> = inst["bars"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| {
                    let f = |k: usize| b[k].as_f64().unwrap();
                    Bar {
                        t: b[0].as_i64().unwrap(),
                        o: f(1),
                        h: f(2),
                        l: f(3),
                        c: f(4),
                        v: f(5),
                    }
                })
                .collect();
            let flow: Vec<FlowPoint> = inst["flow"]
                .as_array()
                .unwrap()
                .iter()
                .zip(&bars)
                .map(|(x, b)| {
                    let mut values = BTreeMap::new();
                    if let Some(n) = x.as_f64() {
                        values.insert("net".to_owned(), n);
                    }
                    FlowPoint { t: b.t, values }
                })
                .collect();
            InstrumentInput {
                symbol: inst["symbol"].as_str().unwrap().to_owned(),
                series: format!("{}.1d", inst["symbol"].as_str().unwrap()),
                bars,
                flow: Some(flow),
            }
        })
        .collect()
}

/// Denetçinin kendi LCG'si.
pub struct Lcg(pub u64);
impl Lcg {
    pub fn u(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}
