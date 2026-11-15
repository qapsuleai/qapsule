//! Denetçinin barlarını (gen_bars.py) qp-facts dedektörlerinden geçirir, çıktıyı JSON'a yazar.
//! Yeniden hesap (recompute.py) qp-facts'i çağırmaz; bu yalnız karşılaştırma tarafı.
//! Kullanım: facts_export <girdi.json> <çıktı.json>
use std::collections::BTreeMap;

use qp_facts::{default_detectors, run_batch, FlowPoint};
use qp_schema::Bar;
use serde_json::{json, Value};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let inp: Value = serde_json::from_str(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    let mut out = Vec::new();
    for inst in inp.as_array().unwrap() {
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
                } else if x.as_str() == Some("nonet") {
                    values.insert("buy".to_owned(), 1.0);
                }
                FlowPoint { t: b.t, values }
            })
            .collect();
        let mut dets = serde_json::Map::new();
        for mut d in default_detectors() {
            let id = d.id().to_owned();
            let res = run_batch(d.as_mut(), &bars, Some(&flow));
            let v: Vec<Value> = res
                .iter()
                .map(|(i, x)| {
                    json!({"bar": i, "code": x.code, "dir": format!("{:?}", x.direction),
                           "edge": format!("{:?}", x.edge), "strength": x.strength,
                           "evidence": x.evidence})
                })
                .collect();
            dets.insert(id, Value::Array(v));
        }
        out.push(json!({"symbol": inst["symbol"], "detections": dets}));
    }
    std::fs::write(&a[2], serde_json::to_string(&out).unwrap()).unwrap();
}
