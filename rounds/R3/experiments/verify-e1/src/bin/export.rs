//! Sentetik barları/akışı ve qp-facts çıktısını JSON'a döker (Python karşılaştırması için).
//! Kullanım: export <çıktı.json> <tohum> <enstrüman> <gün>
use qp_facts::{default_detectors, run_batch};
use serde_json::{json, Value};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    let seed: u64 = a[2].parse().unwrap();
    let n: usize = a[3].parse().unwrap();
    let days: usize = a[4].parse().unwrap();
    let mut insts = Vec::new();
    for (sym, bars, flow) in verify_e1::market(n, days, seed) {
        let jb: Vec<Value> = bars
            .iter()
            .map(|b| json!([b.t, b.o, b.h, b.l, b.c, b.v]))
            .collect();
        let jf: Vec<Value> = flow.iter().map(|f| json!(f.values.get("net"))).collect();
        let mut dets = serde_json::Map::new();
        for mut d in default_detectors() {
            let id = d.id();
            let res = run_batch(d.as_mut(), &bars, Some(&flow));
            let v: Vec<Value> = res
                .iter()
                .map(|(i, x)| {
                    json!({"bar": i, "code": x.code, "dir": format!("{:?}", x.direction),
                           "edge": format!("{:?}", x.edge), "strength": x.strength,
                           "evidence": x.evidence})
                })
                .collect();
            dets.insert(id.to_owned(), Value::Array(v));
        }
        insts.push(json!({"symbol": sym, "bars": jb, "flow": jf, "detections": dets}));
    }
    std::fs::write(out, serde_json::to_string(&insts).unwrap()).unwrap();
    eprintln!("wrote {out}");
}
