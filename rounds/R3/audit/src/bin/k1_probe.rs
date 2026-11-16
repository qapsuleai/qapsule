//! K1 yeniden denetimi — girişimsizliği kendi yolumla kırma denemesi (`r3/int` @ 9ae6b50).
//!
//! Kurgu döküm 39'dan, ama **iki enstrüman** (THYAO `px`, ASELS `px2`; barlar denetçinin),
//! ikisinde de sentetik lisanslı akış. Kişiler: Ana `*`, Arkın (sahip) `*`, Bora `bist/*`
//! (akışsız), Can (hiçbir şey), **Dora `bist/*` + `flow/THYAO`** (kısmi akış yetkisi).
//!
//! Yapılandırmalar: akış tohumları (THYAO, ASELS) = (1,1001), (2,1002), (7,1001) [yalnız THYAO
//! akışı farklı], (1,1007) [yalnız ASELS akışı farklı] ve akışsız. Her koşuda:
//! - Bora/Can/Dora kitle yazar, `capsules`, `capsules {all}`, `changes`, `claims` konularına abone;
//! - barlar iki beslemeden sırayla; bar 120'de Arkın, **bütün koşuların otoritelerinde görülen
//!   her bulgu kimliğini** (birleşim; bazıları yalnız lisanslı doğumlu, bazıları çakışan) onaylar,
//!   bir kısmını reddeder ve birine `show_finding` yapar;
//! - sonda Bora, Can ve Dora: birleşimdeki her kimlik için `capsule`, `get`, `present`,
//!   `verify_evidence`, `mode: propose` ile `reject_finding`, `dispute` (uydurma `t` ve `digest`);
//!   `log` (`after_seq` 0/50/100), `ledger`, `ledger by_kind`, `capsules` biçimleri, `document`.
//! Mühür iletileri çıkarıldıktan sonra Bora, Can ve Dora'nın bütün satırları yapılandırmalar
//! arasında bayt eşit olmalı; Ana'nınki (pozitif kontrol) değişmeli.
//! Kullanım: k1_probe <repo kökü> <bars.json (en az 2 enstrüman)> <bar sayısı>
use std::collections::BTreeSet;
use std::path::Path;

use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

const K: usize = 60;

fn init(root: &Path, b1: &[Bar], b2: &[Bar], flow: Option<(u64, u64)>) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("fixtures/protocol/conformance/39-capsule-entitlement.json"))
            .unwrap(),
    )
    .unwrap();
    let mut v = t["init"].clone();
    let mut f2 = v["data"]["feeds"][0].clone();
    v["data"]["feeds"][0]["bars"] = json!(b1[..K]);
    f2["feed"]["symbol"] = json!("ASELS");
    f2["bars"] = json!(b2[..K]);
    v["data"]["feeds"].as_array_mut().unwrap().push(f2);
    let mut s2 = v["document"]["series"][0].clone();
    s2["id"] = json!("px2");
    s2["instrument"]["symbol"] = json!("ASELS");
    s2["source"]["symbol"] = json!("ASELS");
    v["document"]["series"].as_array_mut().unwrap().push(s2);
    v["capsules"]["series"] = json!(["px", "px2"]);
    v["capsules"]["flow"] = match flow {
        Some((a, b)) => json!([{"seed": a, "series": "px"}, {"seed": b, "series": "px2"}]),
        None => json!([]),
    };
    let e = &mut v["data"]["entitlements"];
    e["bora"] = json!(["bist/THYAO", "bist/ASELS"]);
    e["dora"] = json!(["bist/THYAO", "bist/ASELS", "flow/THYAO"]);
    let mut dora = v["principals"][3].clone();
    dora["id"] = json!("dora");
    dora["author"]["name"] = json!("Dora");
    v["principals"].as_array_mut().unwrap().push(dora.clone());
    v["data"]["entitlements"]["eda"] = json!(["bist/THYAO", "bist/ASELS"]);
    v["data"]["entitlements"]["fatih"] = json!(["bist/THYAO", "bist/ASELS"]);
    let mut eda = dora.clone();
    eda["id"] = json!("eda");
    eda["author"]["name"] = json!("Eda");
    eda["role"] = json!("owner");
    let mut fatih = dora;
    fatih["id"] = json!("fatih");
    fatih["author"]["name"] = json!("Fatih");
    fatih["role"] = json!("proposer");
    v["principals"].as_array_mut().unwrap().push(eda);
    v["principals"].as_array_mut().unwrap().push(fatih);
    serde_json::from_value(v).unwrap()
}

fn ev(sym: &str, seq: usize, b: &Bar) -> qp_schema::data::DataEvent {
    serde_json::from_value(json!({"feed":{"symbol":sym,"interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"bar","bar": b}}))
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

struct Out {
    lines: Vec<OutboundLine>,
    ids: BTreeSet<String>,
    /// Authority-only (licensed-born) capsules whose state the public-view writers changed.
    overwritten: usize,
    /// Licensed-born capsules in the authority.
    licensed_caps: usize,
}

fn lic_states(s: &Session) -> Vec<(String, String)> {
    let d = s.document();
    d.capsules
        .iter()
        .filter(|c| {
            c.story.as_ref().is_some_and(|st| {
                st.history[0].facts.iter().any(|f| f.contains("institutional"))
            })
        })
        .map(|c| (c.id.clone(), format!("{:?}|{:?}", c.state, c.view)))
        .collect()
}

fn run(root: &Path, b1: &[Bar], b2: &[Bar], n: usize, flow: Option<(u64, u64)>, universe: &BTreeSet<String>) -> Out {
    let mut s = Session::new(init(root, b1, b2, flow), root).unwrap();
    let mut out = Vec::new();
    for (c, who, lang) in [("a", "ana", "tr"), ("o", "arkin", "tr"), ("b", "bora", "tr"), ("c", "can", "en"), ("d", "dora", "tr"), ("e", "eda", "tr"), ("f", "fatih", "en")] {
        out.extend(send(&mut s, c, json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"headless","lang":lang}})));
    }
    for c in ["a", "b", "c", "d"] {
        out.extend(send(&mut s, c, json!({"type":"command","id":"aud","ops":[{"op":"set_audience",
            "audience":{"group":"future","sensitivity":"important","watchlist":["THYAO","ASELS"]}}]})));
        for (i, t) in [json!({"kind":"capsules"}), json!({"kind":"capsules","all":true}), json!({"kind":"changes"}), json!({"kind":"claims"})].iter().enumerate() {
            out.extend(send(&mut s, c, json!({"type":"subscribe","id":format!("s{i}"),"topic":t})));
        }
    }
    let fnds: Vec<&String> = universe.iter().filter(|x| x.starts_with("fnd.")).collect();
    for i in K..n {
        out.extend(s.data_event(&ev("THYAO", i - K + 1, &b1[i])).unwrap());
        out.extend(s.data_event(&ev("ASELS", i - K + 1, &b2[i])).unwrap());
        if i == 120 && std::env::var("K1_NO_ARKIN").is_err() {
            // Arkın's commands on every finding id any run's authority had (by bar 120 or later).
            for (j, f) in fnds.iter().enumerate() {
                let op = if j % 3 == 0 { "reject_finding" } else { "approve_finding" };
                out.extend(send(&mut s, "o", json!({"type":"command","id":format!("o{j}"),"ops":[{"op":op,"finding":f}]})));
            }
            if let Some(f) = fnds.iter().find(|f| f.contains("flow")) {
                out.extend(send(&mut s, "o", json!({"type":"command","id":"sf","ops":[{"op":"show_finding","finding":f,"view":"vv"}]})));
            }
        }
    }
    // Public-view writers: Eda (owner) writes directly on every id; Fatih proposes; Eda accepts.
    let before = lic_states(&s);
    for (k, id) in universe.iter().enumerate() {
        let op = if k % 2 == 0 { "approve_finding" } else { "reject_finding" };
        out.extend(send(&mut s, "e", json!({"type":"command","id":format!("e{k}"),"ops":[{"op":op,"finding":id}]})));
        out.extend(send(&mut s, "e", json!({"type":"command","id":format!("ev{k}"),"ops":[{"op":"show_finding","finding":id,"view":format!("ve{k}")}]})));
        out.extend(send(&mut s, "f", json!({"type":"command","id":format!("f{k}"),"ops":[{"op":"approve_finding","finding":id}]})));
    }
    let props: Vec<String> = s.document().proposals.iter().map(|p| p.id.clone()).collect();
    for (k, p) in props.iter().enumerate() {
        out.extend(send(&mut s, "e", json!({"type":"command","id":format!("a{k}"),"ops":[{"op":"accept_proposal","proposal":p}]})));
    }
    for k in 0..40 {
        out.extend(send(&mut s, "e", json!({"type":"command","id":format!("ap{k}"),"ops":[{"op":"accept_proposal","proposal":format!("p{k}")}]})));
    }
    let after = lic_states(&s);
    let overwritten = before.iter().zip(&after).filter(|(x, y)| x != y).count() + before.len().abs_diff(after.len());
    let licensed_caps = after.len();
    for c in ["b", "c", "d", "a", "e", "f"] {
        for (k, id) in universe.iter().enumerate() {
            for q in [
                json!({"kind":"capsule","capsule":id}),
                json!({"kind":"get","id":id}),
                json!({"kind":"present","finding":id}),
                json!({"kind":"verify_evidence","item":id}),
            ] {
                out.extend(send(&mut s, c, json!({"type":"query","id":"q","query":q})));
            }
            out.extend(send(&mut s, c, json!({"type":"command","id":format!("p{k}"),"mode":"propose","ops":[{"op":"reject_finding","finding":id}]})));
            out.extend(send(&mut s, c, json!({"type":"dispute","id":format!("d{k}"),"claim":id,"t":1_700_000_000_000i64 + k as i64,"digest":"0123456789abcdef"})));
        }
        for q in [
            json!({"kind":"log"}),
            json!({"kind":"log","after_seq":50}),
            json!({"kind":"log","after_seq":100}),
            json!({"kind":"ledger"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","audience":true}),
            json!({"kind":"capsules","meaning":"flow"}),
            json!({"kind":"capsules","instrument":"ASELS"}),
            json!({"kind":"document"}),
            json!({"kind":"describe"}),
        ] {
            out.extend(send(&mut s, c, json!({"type":"query","id":"z","query":q})));
        }
    }
    let d = s.document();
    let ids = d
        .facts
        .iter()
        .map(|f| f.id.clone())
        .chain(d.findings.iter().map(|f| f.id.clone()))
        .chain(d.capsules.iter().map(|c| c.id.clone()))
        .collect();
    Out { lines: out, ids, overwritten, licensed_caps }
}

fn strip(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("comments");
            if m.get("kind").and_then(Value::as_str) == Some("server_comment") {
                m.clear();
                m.insert("kind".into(), json!("SC"));
            }
            for x in m.values_mut() {
                strip(x);
            }
        }
        Value::Array(a) => {
            a.retain(|x| x.get("trust").and_then(Value::as_str) != Some("server"));
            for x in a {
                strip(x);
            }
        }
        _ => {}
    }
}

fn lines(o: &Out, c: &str) -> Vec<String> {
    o.lines
        .iter()
        .filter(|l| l.conn == c)
        .filter_map(|l| {
            let mut v = serde_json::to_value(&l.msg).unwrap();
            strip(&mut v);
            let t = v.to_string();
            (!t.contains("\"kind\":\"SC\"")).then_some(t)
        })
        .collect()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let root = Path::new(&a[1]).to_path_buf();
    let inp = r3_audit::load(&a[2]);
    let n: usize = a[3].parse().unwrap();
    let (b1, b2) = (inp[0].bars[..n].to_vec(), inp[1].bars[..n].to_vec());
    let configs: Vec<(&str, Option<(u64, u64)>)> = vec![
        ("(1,1001)", Some((1, 1001))),
        ("(2,1002)", Some((2, 1002))),
        ("(7,1001)", Some((7, 1001))),
        ("(1,1007)", Some((1, 1007))),
        ("akışsız", None),
    ];
    // Pass 1: the id universe (every authority's items, every config).
    let mut universe = BTreeSet::new();
    for (_, f) in &configs {
        universe.extend(run(&root, &b1, &b2, n, *f, &BTreeSet::new()).ids);
    }
    let lic: usize = universe.iter().filter(|x| x.contains("institutional")).count();
    println!("kimlik evreni: {} (lisanslı olgu kimliği {lic})", universe.len());
    // Pass 2: active probes with the universe.
    let runs: Vec<(&str, Out)> = configs.iter().map(|(name, f)| (*name, run(&root, &b1, &b2, n, *f, &universe))).collect();
    let base = &runs[0].1;
    for (name, o) in &runs[1..] {
        let mut row = format!("{:>10} ↔ (1,1001):", name);
        for c in ["b", "c", "d", "e", "f", "a"] {
            let (x, y) = (lines(base, c), lines(o, c));
            let diff = x.iter().zip(&y).filter(|(p, q)| p != q).count() + x.len().abs_diff(y.len());
            row += &format!("  {c} {}/{} satır, farklı {diff}", x.len(), y.len());
            if diff > 0 && std::env::var("K1_SHOW").map_or(false, |v| v == c) {
                if let Some((i, (p, q))) = x.iter().zip(&y).enumerate().find(|(_, (p, q))| p != q) {
                    let k = p.chars().zip(q.chars()).take_while(|(u, w)| u == w).count();
                    let lo = k.saturating_sub(300);
                    eprintln!("[{name}] {c} satır {i}:\n  A …{}\n  B …{}", p.chars().skip(lo).take(500).collect::<String>(), q.chars().skip(lo).take(500).collect::<String>());
                }
            }
        }
        println!("{row}");
    }
    for (name, o) in &runs {
        println!("{name:>10}: otoritede lisanslı doğumlu kapsül {}, açık görünümlü yazarların durumunu/görünümünü değiştirdiği {}", o.licensed_caps, o.overwritten);
    }
    // Dora (flow/THYAO entitled) never sees a THYAO licensed fact (public view)?
    let dl = lines(base, "d").join("\n");
    println!(
        "Dora'nın satırlarında `flow.institutional` geçiyor mu: {}",
        dl.contains("flow.institutional")
    );
}
