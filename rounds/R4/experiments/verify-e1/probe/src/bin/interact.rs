//! T4 interaction probes on the init of transcript 52 (THYAO daily, capsule engine).
use std::path::{Path, PathBuf};

use qp_schema::protocol::SessionInit;
use qp_session::{document_digest, Session};
use serde_json::{json, Value};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..") }

struct S { s: Session, n: u64 }
impl S {
    fn raw(&mut self, c: &str, v: Value) -> Vec<Value> {
        self.n += 1;
        self.s.handle_raw(c, &v.to_string()).iter().map(|l| { let mut m = serde_json::to_value(&l.msg).unwrap(); m["_conn"] = json!(l.conn); m }).collect()
    }
    fn first(&mut self, c: &str, v: Value) -> Value { self.raw(c, v).into_iter().next().unwrap() }
    fn input(&mut self, g: Value) -> Value { let id = format!("i{}", self.n); self.first("p", json!({"type":"input","id":id,"gesture":g})) }
    fn hit(&mut self, x: f64) -> Option<i64> {
        let id = format!("h{}", self.n);
        let r = self.first("p", json!({"type":"query","id":id,"query":{"kind":"hit","view":"main","x":x,"y":150.0,"width":390,"height":300}}));
        r["data"]["hit"]["bar"].as_i64()
    }
    fn look(&mut self, a: i64, b: i64) -> Value { self.input(json!({"kind":"look","view":"main","from_bar":a,"to_bar":b})) }
}

fn vp(r: &Value) -> (i64, i64) {
    let v = if r["data"]["interaction"].is_object() { &r["data"]["interaction"]["viewport"] } else { &r["data"]["viewport"] };
    (v["from_bar"].as_i64().unwrap_or(-1), v["to_bar"].as_i64().unwrap_or(-1))
}
fn center(b: &Value) -> (f64, f64) {
    ((b["left"].as_f64().unwrap() + b["right"].as_f64().unwrap()) / 2.0, (b["top"].as_f64().unwrap() + b["bottom"].as_f64().unwrap()) / 2.0)
}

fn main() {
    let d: Value = serde_json::from_str(&std::fs::read_to_string(root().join("fixtures/protocol/conformance/52-input-pan-pinch.json")).unwrap()).unwrap();
    let init: SessionInit = serde_json::from_value(d["init"].clone()).unwrap();
    let mut s = S { s: Session::new(init, &root()).unwrap(), n: 0 };
    s.first("p", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}));
    s.first("w", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}));
    s.first("w", json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}));
    s.first("c", json!({"type":"hello","protocol":"qp/0","principal":"can","surface":{"kind":"phone"}}));

    // A. pixel <-> bar
    println!("A. pinch: the bar under x stays under x");
    for x in [40.0, 120.0, 200.0, 280.0, 330.0] {
        s.look(70, 128);
        let b0 = s.hit(x);
        let r = s.input(json!({"kind":"pinch","target":{"view":"main"},"factor":2.0,"x":x,"width":390,"height":300}));
        let v = vp(&r);
        let b1 = s.hit(x);
        println!("  x {x:>5}: bar before {b0:?} after {b1:?} | viewport {v:?} width {} (from 59 bars /2 = {})", v.1 - v.0 + 1, (59.0f64 / 2.0).round());
    }
    // slot width from hits, on the range the drags start from
    s.look(40, 100);
    let mut edges = Vec::new();
    let mut prev = s.hit(0.0);
    for xi in 0..390 { let b = s.hit(xi as f64); if b != prev && b.is_some() && prev.is_some() { edges.push(xi) } prev = b; }
    let slot = if edges.len() > 2 { (edges[edges.len() - 1] - edges[0]) as f64 / (edges.len() - 1) as f64 } else { f64::NAN };
    println!("  slot from hit edges: {slot:.3} px ({} edges)", edges.len());
    println!("A. drag: the bar under the finger follows it (round to whole bars)");
    for dx in [23.0, -41.0, 7.0, -3.0 * slot, 2.0 * slot, 400.0, -2000.0] {
        s.look(40, 100);
        let x0 = 200.0;
        let b0 = s.hit(x0);
        let r = s.input(json!({"kind":"drag","target":{"view":"main"},"dx":dx,"width":390,"height":300}));
        let v = vp(&r);
        let b1 = s.hit(x0 + dx);
        let k = (-dx / slot).round() as i64;
        s.look(40, 100);
        let rp = s.input(json!({"kind":"pan","view":"main","bars":k}));
        println!("  dx {dx:>8.2}: bar under finger before {b0:?} after {b1:?} | viewport {v:?} | pan({k}) viewport {:?} equal={}", vp(&rp), vp(&rp) == v);
    }

    // B. view state not in the log, document unchanged
    let log0 = s.s.log().len();
    let dig0 = document_digest(s.s.document());
    let seq0 = s.first("p", json!({"type":"query","id":"lq","query":{"kind":"screen"}}));
    let _ = seq0;
    let caps: Vec<String> = s.s.document().capsules.iter().map(|c| c.id.clone()).take(6).collect();
    let mut errors = 0; let mut w_lines = 0; let mut sent = 0;
    for i in 0..1500u32 {
        let g = match i % 9 {
            0 => json!({"kind":"drag","target":{"view":"main"},"dx": ((i * 37) % 200) as f64 - 100.0, "width":390,"height":300}),
            1 => json!({"kind":"pinch","target":{"view":"main"},"factor": 0.5 + ((i % 7) as f64) * 0.3, "x": ((i * 13) % 390) as f64}),
            2 => json!({"kind":"tap_at","target":{"view":"main"},"x": ((i * 17) % 390) as f64, "y": ((i * 7) % 300) as f64}),
            3 => json!({"kind":"open","capsule": caps[(i as usize / 9) % caps.len()], "to":"chart"}),
            4 => json!({"kind":"drag","dx": ((i * 11) % 120) as f64 - 60.0}),
            5 => json!({"kind":"pinch","factor": 1.5}),
            6 => json!({"kind":"open","capsule": caps[(i as usize / 9) % caps.len()], "to":"evidence"}),
            7 => json!({"kind":"close","to":"sentence"}),
            _ => json!({"kind":"look","view":"main","from_bar": (i % 50) as i64, "to_bar": 100}),
        };
        let out = s.raw("p", json!({"type":"input","id":format!("b{i}"),"gesture":g}));
        sent += 1;
        for m in &out { if m["type"] == "error" { errors += 1 } if m["_conn"] == "w" { w_lines += 1 } }
    }
    let scr = s.first("p", json!({"type":"query","id":"sc","query":{"kind":"screen"}}));
    println!("B. {sent} inputs (errors {errors}): log {log0} -> {} ; doc digest equal {} ; lines to the `changes` subscriber {w_lines} ; screen has view_state {}",
        s.s.log().len(), dig0 == document_digest(s.s.document()), scr["data"].to_string().contains("view_state"));

    // C. shared boxes: tap the centre of each "after" box in the new scene, see what is there
    s.input(json!({"kind":"close","to":"sentence"}));
    let cap = "cap.THYAO.breakout.down.124";
    let o1 = s.input(json!({"kind":"open","capsule":cap,"to":"capsule","width":390,"height":300}));
    let sh1 = o1["data"]["interaction"]["opening"]["shared"].as_array().cloned().unwrap_or_default();
    println!("C. open sentence->capsule: shared {:?}", sh1.iter().map(|x| x["id"].as_str().unwrap().to_owned()).collect::<Vec<_>>());
    let check = |s: &mut S, depth: &str, sh: &[Value]| {
        for e in sh {
            let id = e["id"].as_str().unwrap();
            let Some(after) = e.get("after").filter(|a| a.is_object()) else { continue };
            let (x, y) = center(after);
            let r = s.input(json!({"kind":"tap_at","target":{"capsule":cap},"x":x,"y":y}));
            let h = &r["data"]["interaction"]["hit"];
            let item = &h["item"];
            let what = format!("bar {:?} mark {:?} data {} origins {}", h["bar"].as_i64(), item["mark"].as_str(), item["data"], item["values"].to_string().chars().take(0).collect::<String>() + &item.get("origins").map(|o| o.to_string()).unwrap_or_default());
            let ok = match id {
                "glyph" => item["mark"] == "glyph",
                "price" => item["data"]["series"] == "px" || h["bar"].is_i64(),
                x if x.starts_with("bar:") => h["bar"].as_i64() == x[4..].parse::<i64>().ok(),
                x if x.starts_with("fact:") => item.to_string().contains(&x[5..]),
                x if x.starts_with("evidence:") => item.to_string().contains(&x[9..]),
                "headline" | "ledger" => item["mark"] == "text",
                _ => false,
            };
            println!("   [{depth}] {id:<45} centre ({x:.0},{y:.0}) -> {} | {what}", if ok { "MATCH" } else { "NO MATCH" });
            // return to the depth we were checking
            let st = r["data"]["interaction"]["state"]["depth"].as_str().unwrap_or("").to_owned();
            if st != depth { s.input(json!({"kind":"close","to":depth})); }
        }
    };
    check(&mut s, "capsule", &sh1);
    let o2 = s.input(json!({"kind":"open","capsule":cap,"to":"chart"}));
    let sh2 = o2["data"]["interaction"]["opening"]["shared"].as_array().cloned().unwrap_or_default();
    // consistency: before boxes of capsule->chart == after boxes of sentence->capsule
    let mut cons = 0; let mut incons = 0;
    for e in &sh2 { if let Some(m) = sh1.iter().find(|x| x["id"] == e["id"]) { if m["after"] == e["before"] { cons += 1 } else { incons += 1; println!("   before/after mismatch {}", e["id"]) } } }
    println!("C. open capsule->chart: shared {:?}; before boxes equal to the capsule scene's boxes: {cons} equal, {incons} differ",
        sh2.iter().map(|x| x["id"].as_str().unwrap().to_owned()).collect::<Vec<_>>());
    check(&mut s, "chart", &sh2);
    let c1 = s.input(json!({"kind":"close","to":"capsule"}));
    let sh3 = c1["data"]["interaction"]["opening"]["shared"].as_array().cloned().unwrap_or_default();
    let same = sh3.iter().all(|e| sh1.iter().find(|x| x["id"] == e["id"]).map(|m| m["after"] == e["after"]).unwrap_or(false));
    println!("C. close chart->capsule: after boxes == first capsule scene: {same}");

    // D. evidence: authorised (arkin) vs not (can)
    let ea = s.input(json!({"kind":"open","capsule":cap,"to":"evidence"}));
    let ev = &ea["data"]["interaction"]["opening"]["evidence"];
    println!("D. arkin evidence: {}", ev.to_string().chars().take(400).collect::<String>());
    let ec = s.first("c", json!({"type":"input","id":"ce","gesture":{"kind":"open","capsule":cap,"to":"evidence"}}));
    println!("D. can (no entitlement): {}", ec.to_string().chars().take(700).collect::<String>());
    println!("D. can presentation: {}", serde_json::to_string(&ec["data"]["interaction"]["opening"]["presentation"]).unwrap());
    let qc = s.first("c", json!({"type":"query","id":"pc","query":{"kind":"present","finding":"fnd.THYAO.breakout.down.124","depth":"chart"}}));
    println!("D. can present query: {}", qc.to_string().chars().take(1500).collect::<String>());
    let qd = s.first("c", json!({"type":"query","id":"qd","query":{"kind":"capsule","capsule":cap}}));
    println!("D. can capsule query evidence equals opening evidence: {}", qd["data"]["detail"]["evidence"] == ec["data"]["interaction"]["opening"]["evidence"]);
    let qa = s.first("p", json!({"type":"query","id":"qa","query":{"kind":"capsule","capsule":cap}}));
    println!("D. arkin capsule query evidence equals opening evidence: {}", &qa["data"]["detail"]["evidence"] == ev);
    let it = &qa["data"]["detail"]["item"];
    println!("E. item stage {} validity {} glyph.validity {} | sentence tr: {}", it["stage"], it["validity"], qa["data"]["detail"]["glyph"]["validity"], it["sentence"]["tr"]);
    println!("E. opening(chart) presentation text (arkin): {}", ea["data"]["interaction"]["opening"]["presentation"]["text"]);
    let oc = s.input(json!({"kind":"close","to":"capsule"}));
    println!("E. opening(capsule) presentation: {}", oc["data"]["interaction"]["opening"]["presentation"].to_string().chars().take(900).collect::<String>());
}
