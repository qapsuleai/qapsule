//! Yerel gerçek barları ve qp-facts çıktısını depo DIŞINDA bir geçici dosyaya döker
//! (yalnız recompute.py karşılaştırması için; sonra silinir). Akış yok (institutional 0).
//! Kullanım: export_local <depo-dışı çıktı.json> <periyot>
use qp_facts::{default_detectors, run_batch};
use qp_source::{load_local_bars, local_bars_dir};
use serde_json::{json, Value};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    assert!(!out.contains("/rounds/"), "never write real data into the repo");
    let mut insts = Vec::new();
    for (n, (_, bars)) in load_local_bars(&local_bars_dir(), &a[2]).into_iter().enumerate() {
        let jb: Vec<Value> = bars.iter().map(|b| json!([b.t, b.o, b.h, b.l, b.c, b.v])).collect();
        let jf: Vec<Value> = bars.iter().map(|_| Value::Null).collect();
        let mut dets = serde_json::Map::new();
        for mut d in default_detectors() {
            let id = d.id();
            let v: Vec<Value> = run_batch(d.as_mut(), &bars, None)
                .iter()
                .map(|(i, x)| json!({"bar": i, "code": x.code, "dir": format!("{:?}", x.direction),
                    "edge": format!("{:?}", x.edge), "strength": x.strength, "evidence": x.evidence}))
                .collect();
            dets.insert(id.to_owned(), Value::Array(v));
        }
        insts.push(json!({"symbol": format!("R{n}"), "bars": jb, "flow": jf, "detections": dets}));
    }
    std::fs::write(out, serde_json::to_string(&insts).unwrap()).unwrap();
}
