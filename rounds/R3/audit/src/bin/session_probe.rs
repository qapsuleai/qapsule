//! VERIFY 7, 9, 10 — otoritenin (qp-session) ürün API'si üzerinden denetçi denemeleri.
//!
//! Veri: denetçinin kendi barları (`gen_bars.py`, ilk enstrüman), THYAO beslemesi olarak; init
//! döküm 39'unkiyle aynı kurgu (ana `*`, bora `bist/THYAO`, can yetkisiz; sentetik lisanslı akış
//! `init.capsules.flow`, tohum argümandan).
//!
//! - K26: ana'nın TR ve EN bağlantısında "artık" / "no longer" içeren cümleler.
//! - Sızıntı taraması: bora ve can'a giden **her** satırda, ham lisanslı akışın bütün değerleri
//!   (`net`, `buy`, `sell`, `brokers_net_top5`, her bar), lisanslı olguların kimlikleri ve kanıt
//!   değerleri; can için ayrıca bütün `evidence_ref` özetleri.
//! - Varlık kâhini: bora'nın lisanslı kapsül kimliği ile olmayan bir kimlik için aldığı hata.
//! - Girişimsizlik (non-interference): aynı açık barlar, **farklı lisanslı akış tohumları**; bora
//!   ve can'a giden akış, sunucu yorumları (mühür iletileri) çıkarıldıktan sonra bayt eşit olmalı.
//! - Aday: açık barda hacim sıçraması yamayla eklenip geri alınır; kapanıştan sonra belge, karne
//!   ve günlükteki motor kayıtları, sıçramayı hiç görmemiş koşuyla aynı mı.
//!
//! Kullanım: session_probe <repo kökü> <bars.json> <bar sayısı>
use std::collections::BTreeSet;
use std::path::Path;

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

fn init(root: &Path, bars: &[Bar], k: usize, seed: u64) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("fixtures/protocol/conformance/39-capsule-entitlement.json"))
            .unwrap(),
    )
    .unwrap();
    let mut v = t["init"].clone();
    v["data"]["feeds"][0]["bars"] = json!(bars[..k]);
    v["capsules"]["flow"][0]["seed"] = json!(seed);
    serde_json::from_value(v).unwrap()
}

fn ev(seq: usize, b: &Bar) -> DataEvent {
    serde_json::from_value(json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"bar","bar": b}}))
    .unwrap()
}

fn patch(seq: usize, t: i64, fields: Value) -> DataEvent {
    serde_json::from_value(json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"patch","t": t, "fields": fields}}))
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn hello(s: &mut Session, conn: &str, who: &str, lang: Option<&str>) -> Vec<OutboundLine> {
    let mut surface = json!({"kind":"headless"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    send(
        s,
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    )
}

fn lines_of(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_string(&l.msg).unwrap())
        .collect()
}

/// Removes server comments (the mühür messages, allowed to depend on licensed data).
fn strip_comments(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("comments");
            if m.get("kind").and_then(Value::as_str) == Some("server_comment") {
                m.clear();
                m.insert("kind".into(), json!("server_comment(stripped)"));
            }
            for x in m.values_mut() {
                strip_comments(x);
            }
        }
        Value::Array(a) => {
            a.retain(|x| x.get("trust").and_then(Value::as_str) != Some("server"));
            for x in a {
                strip_comments(x);
            }
        }
        _ => {}
    }
}

struct Run {
    out: Vec<OutboundLine>,
    session: Session,
}

/// One full run: hello, subscriptions, bars, a battery of queries.
fn full_run(root: &Path, bars: &[Bar], n: usize, seed: u64) -> Run {
    let k = 60;
    let mut s = Session::new(init(root, bars, k, seed), root).unwrap();
    let mut out = Vec::new();
    out.extend(hello(&mut s, "t", "ana", None));
    out.extend(hello(&mut s, "e", "ana", Some("en")));
    out.extend(hello(&mut s, "b", "bora", None));
    out.extend(hello(&mut s, "c", "can", Some("en")));
    for c in ["b", "c"] {
        out.extend(send(&mut s, c, json!({"type":"command","id":"aud","ops":[{"op":"set_audience",
            "audience":{"group":"future","sensitivity":"important","watchlist":["THYAO"]}}]})));
        for (i, t) in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","all":true}),
            json!({"kind":"capsules","instrument":"THYAO"}),
            json!({"kind":"changes"}),
            json!({"kind":"claims"}),
        ]
        .iter()
        .enumerate()
        {
            out.extend(send(&mut s, c, json!({"type":"subscribe","id":format!("s{i}"),"topic":t})));
        }
    }
    for (i, b) in bars.iter().enumerate().take(n).skip(k) {
        out.extend(s.data_event(&ev(i - k + 1, b)).unwrap());
    }
    let last = (n - 1) as u64;
    for c in ["b", "c", "t", "e"] {
        for q in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","audience":true}),
            json!({"kind":"capsules","meaning":"flow"}),
            json!({"kind":"capsules","open":true}),
            json!({"kind":"capsules","since_bar": last - 30}),
            json!({"kind":"ledger"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"describe"}),
            json!({"kind":"log"}),
            json!({"kind":"document"}),
            json!({"kind":"list","collection":"facts","full":true}),
            json!({"kind":"list","collection":"capsules","full":true}),
            json!({"kind":"list","collection":"findings","full":true}),
            json!({"kind":"present","finding":{"latest":true}}),
            json!({"kind":"present","finding":{"kind":"flow"}}),
            json!({"kind":"verify_evidence","item":{"latest":true}}),
        ] {
            out.extend(send(&mut s, c, json!({"type":"query","id":"q","query":q})));
        }
    }
    Run { out, session: s }
}

fn num_strings(x: f64) -> Option<String> {
    if !x.is_finite() || x.fract() == 0.0 {
        return None;
    }
    let s = serde_json::to_string(&x).unwrap();
    (s.len() >= 5).then_some(s)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let root = Path::new(&a[1]).to_path_buf();
    let inputs = r3_audit::load(&a[2]);
    let n: usize = a[3].parse().unwrap();
    let bars: Vec<Bar> = inputs[0].bars[..n].to_vec();

    // ---------------------------------------------------------------- run A (seed 1)
    let ra = full_run(&root, &bars, n, 1);
    let doc = ra.session.document().clone();

    // Replay from the log (criterion 4) on the auditor's data, and a second identical run.
    {
        let again = qp_session::replay(ra.session.initial_document(), ra.session.log()).unwrap();
        let rb = full_run(&root, &bars, n, 1);
        println!(
            "== Yeniden oynatma: günlükten belge = canlı belge {} ; ikinci koşu günlük eşit {} ; giden satırlar eşit {} ; kapsül {} olay-günlük kaydı {}",
            serde_json::to_vec(&again).unwrap() == serde_json::to_vec(ra.session.document()).unwrap(),
            serde_json::to_string(rb.session.log()).unwrap() == serde_json::to_string(ra.session.log()).unwrap(),
            lines_of(&rb.out, "b") == lines_of(&ra.out, "b") && lines_of(&rb.out, "t") == lines_of(&ra.out, "t"),
            doc.capsules.len(),
            ra.session.log().len()
        );
    }

    // K26
    println!("== K26 (ana, TR / EN; son bar {})", n - 1);
    let caps_t: Vec<Value> = lines_of(&ra.out, "t")
        .iter()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter_map(|v| v["data"]["capsules"].as_array().cloned())
        .next_back()
        .unwrap_or_default();
    let mut shown = 0;
    for c in &caps_t {
        let tr = c["sentence"]["tr"].as_str().unwrap_or("");
        let en = c["sentence"]["en"].as_str().unwrap_or("");
        if (tr.contains("artık") && en.contains("no longer")) && c["time"] == "state" && shown < 4 {
            shown += 1;
            println!("  {} stage={} time={}\n    TR: {tr}\n    EN: {en}", c["id"], c["stage"], c["time"]);
        }
    }
    let states_open_but_no_longer = caps_t
        .iter()
        .filter(|c| {
            !matches!(
                c["stage"].as_str(),
                Some("realized" | "invalidated" | "expired" | "retracted")
            ) && c["sentence"]["tr"].as_str().unwrap_or("").contains("Artık geçerli değil")
        })
        .count();
    println!(
        "  kapsül {}; 'artık'+'no longer' olan durum kapsülü gösterildi {shown}; açık olup 'Artık geçerli değil' diyen {states_open_but_no_longer}",
        caps_t.len()
    );

    // Forbidden strings.
    let licensed: BTreeSet<String> = doc
        .facts
        .iter()
        .filter(|f| f.meaning.as_ref().is_some_and(|m| m.server))
        .map(|f| f.id.clone())
        .collect();
    let lic_caps: Vec<String> = doc
        .capsules
        .iter()
        .filter(|c| {
            c.story
                .as_ref()
                .is_some_and(|st| st.history[0].facts.iter().any(|f| licensed.contains(f)))
        })
        .flat_map(|c| [c.id.clone(), c.finding.clone()])
        .collect();
    let mut raw: BTreeSet<String> = BTreeSet::new();
    for r in qp_source::synthetic_flow(&bars, 1) {
        for v in r.values.values() {
            if let Some(s) = num_strings(*v) {
                raw.insert(s);
            }
        }
    }
    for f in doc.facts.iter().filter(|f| licensed.contains(&f.id)) {
        for v in f.evidence.values() {
            if let Some(s) = num_strings(*v) {
                raw.insert(s);
            }
        }
    }
    // A raw value may coincide with a public number; drop those seen by the entitled
    // reader's public-only data (bars) to avoid false alarms.
    let public: BTreeSet<String> = bars
        .iter()
        .flat_map(|b| [b.o, b.h, b.l, b.c, b.v])
        .filter_map(num_strings)
        .collect();
    raw.retain(|x| !public.contains(x));
    let digests: BTreeSet<String> = doc
        .facts
        .iter()
        .filter_map(|f| f.evidence_ref.as_ref().map(|r| r.digest.clone()))
        .collect();
    println!(
        "== Sızıntı taraması: lisanslı olgu {}, lisanslı hikâye {}, yasak ham değer {}, özet {}",
        licensed.len(),
        lic_caps.len() / 2,
        raw.len(),
        digests.len()
    );
    for c in ["b", "c"] {
        let ls = lines_of(&ra.out, c);
        let mut hits = 0;
        let mut first = String::new();
        for l in &ls {
            let mut bad: Vec<&str> = raw.iter().filter(|v| l.contains(v.as_str())).map(String::as_str).collect();
            bad.extend(licensed.iter().chain(&lic_caps).filter(|id| l.contains(&format!("\"{id}\""))).map(String::as_str));
            if c == "c" {
                bad.extend(digests.iter().filter(|d| l.contains(d.as_str())).map(String::as_str));
            }
            if !bad.is_empty() {
                hits += 1;
                if first.is_empty() {
                    first = format!("{bad:?} in {:.200}", l);
                }
            }
        }
        let comments = ls.iter().filter(|l| l.contains("\"trust\":\"server\"")).count();
        println!("  {c}: {} satır, sızıntılı satır {hits} {first}; mühür iletisi içeren satır {comments}", ls.len());
    }
    // Ana must see them (positive control).
    let ana = lines_of(&ra.out, "t");
    let seen = raw.iter().filter(|v| ana.iter().any(|l| l.contains(v.as_str()))).count();
    println!("  pozitif kontrol: ana'nın satırlarında görülen yasak değer {seen}");

    // Existence oracle.
    let Run { out: ra_out, session: mut s } = ra;
    if let Some(id) = lic_caps.first() {
        let fake = format!("cap.THYAO.flow.up.{}", 100_000);
        let r1 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"capsule","capsule":id}}));
        let r2 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"capsule","capsule":fake}}));
        let r3 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"get","id":id}}));
        let r4 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"get","id":fake}}));
        let norm = |r: &[OutboundLine], id: &str| serde_json::to_string(&r[0].msg).unwrap().replace(id, "<ID>");
        println!(
            "== Varlık kâhini (bora): capsule gerçek=sahte {} ; get gerçek=sahte {}\n    {}\n    {}",
            norm(&r1, id) == norm(&r2, &fake),
            norm(&r3, id) == norm(&r4, &fake),
            norm(&r1, id),
            norm(&r3, id)
        );
        let lf = licensed.iter().next().unwrap().clone();
        let r5 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"verify_evidence","item":lf}}));
        let r6 = send(&mut s, "b", json!({"type":"query","id":"x","query":{"kind":"verify_evidence","item":"fx.THYAO.flow.institutional.net_buy.99999"}}));
        println!(
            "    verify_evidence gerçek=sahte {}",
            norm(&r5, &lf) == norm(&r6, "fx.THYAO.flow.institutional.net_buy.99999")
        );
        let r7 = send(&mut s, "b", json!({"type":"command","id":"x","ops":[{"op":"approve_finding","finding": lic_caps[1]}]}));
        let r8 = send(&mut s, "b", json!({"type":"command","id":"x","ops":[{"op":"approve_finding","finding": "fnd.THYAO.flow.up.100000"}]}));
        println!(
            "    approve_finding gerçek=sahte {} ({})",
            norm(&r7, &lic_caps[1]) == norm(&r8, "fnd.THYAO.flow.up.100000"),
            norm(&r7, &lic_caps[1]).chars().take(160).collect::<String>()
        );
    }

    // Non-interference: other licensed seeds.
    println!("== Girişimsizlik (aynı açık barlar, farklı lisanslı akış; mühür iletileri çıkarıldı)");
    let strip_all = |o: &[OutboundLine], c: &str| -> Vec<Value> {
        lines_of(o, c)
            .iter()
            .filter_map(|l| {
                let mut v: Value = serde_json::from_str(l).unwrap();
                strip_comments(&mut v);
                let t = serde_json::to_string(&v).unwrap();
                (!t.contains("server_comment(stripped)")).then_some(v)
            })
            .collect()
    };
    for seed in [2u64, 7, 42] {
        let rb = full_run(&root, &bars, n, seed);
        for c in ["b", "c"] {
            let x = strip_all(&ra_out, c);
            let y = strip_all(&rb.out, c);
            let mut keys: std::collections::BTreeMap<String, usize> = Default::default();
            let mut nd = 0;
            for (p, q) in x.iter().zip(&y) {
                if p != q {
                    nd += 1;
                    json_diff(p, q, "", &mut keys);
                }
            }
            println!(
                "  tohum 1 vs {seed}, {c}: satır {} / {}, farklı {nd}; farklı alanlar {:?}",
                x.len(),
                y.len(),
                keys.iter().take(12).collect::<Vec<_>>()
            );
        }
        // The final query answers (same order in both runs: 16 queries per connection).
        for c in ["b", "c"] {
            let x = strip_all(&ra_out, c);
            let y = strip_all(&rb.out, c);
            let names = ["capsules", "capsules+audience", "capsules meaning=flow", "capsules open", "capsules since", "ledger", "ledger by_kind", "describe", "log", "document", "list facts", "list capsules", "list findings", "present latest", "present flow", "verify latest"];
            let (xa, ya) = (&x[x.len() - 16..], &y[y.len() - 16..]);
            let mut diffs = Vec::new();
            for (k, (p, q)) in xa.iter().zip(ya).enumerate() {
                if p != q {
                    let mut keys = Default::default();
                    json_diff(p, q, "", &mut keys);
                    diffs.push(format!("{}: {:?}", names[k], keys.keys().take(8).collect::<Vec<_>>()));
                }
            }
            println!("    son sorgu cevapları, {c}: {} / 16 farklı", diffs.len());
            for d in diffs {
                println!("      {d}");
            }
            // The finding confidence as delivered to this reader.
            let conf = |v: &Value| -> Vec<(String, String)> {
                v["data"]["items"].as_array().map(|a| a.iter().filter(|f| f["id"].as_str().is_some_and(|i| i.starts_with("fnd.THYAO.breakout"))).map(|f| (f["id"].as_str().unwrap().to_owned(), f["confidence"].to_string())).collect()).unwrap_or_default()
            };
            let (ca, cb) = (conf(&xa[12]), conf(&ya[12]));
            let dif: Vec<_> = ca.iter().filter(|p| cb.iter().any(|q| q.0 == p.0 && q.1 != p.1)).map(|p| (p.clone(), cb.iter().find(|q| q.0 == p.0).unwrap().1.clone())).collect();
            if let Some((id, _)) = ca.iter().find(|p| cb.iter().any(|q| q.0 == p.0 && q.1 != p.1)) {
                let pick = |v: &Value| v["data"]["items"].as_array().and_then(|a| a.iter().find(|f| f["id"] == json!(id)).cloned()).unwrap_or_default();
                let (fa, fb) = (pick(&xa[12]), pick(&ya[12]));
                println!("      örnek {id}:\n        A claim={} headline={}\n        B claim={} headline={}", fa["claim"], fa["headline"], fb["claim"], fb["headline"]);
            }
            println!("      {c}'ye giden kırılım bulgularında confidence farkı: {} {:?}", dif.len(), dif.iter().take(3).collect::<Vec<_>>());
        }
        if seed == 2 {
            // Diagnose: a public finding whose confidence differs.
            let da = s.document();
            let db = rb.session.document();
            let mut shown = 0;
            for fa in &da.findings {
                let Some(fb) = db.findings.iter().find(|f| f.id == fa.id) else { continue };
                if fa.confidence != fb.confidence && shown < 3 {
                    shown += 1;
                    let cap = da.capsules.iter().find(|c| c.finding == fa.id);
                    let capb = db.capsules.iter().find(|c| c.finding == fa.id);
                    let facts = |d: &qp_schema::Document, c: Option<&qp_schema::Capsule>| -> Vec<String> {
                        c.and_then(|c| c.story.as_ref()).map(|st| st.history.iter().flat_map(|h| h.facts.iter().map(move |f| {
                            let fx = d.facts.iter().find(|x| &x.id == f);
                            format!("{}@{:?}:{} s={:.4}", f, h.stage, h.bar, fx.and_then(|x| x.meaning.as_ref()).map_or(f64::NAN, |m| m.strength))
                        })).collect()).unwrap_or_default()
                    };
                    println!("    {}: confidence {} vs {}", fa.id, fa.confidence, fb.confidence);
                    println!("      tohum 1 hikâye olguları {:?}", facts(da, cap));
                    println!("      tohum 2 hikâye olguları {:?}", facts(db, capb));
                }
            }
        }
    }

    // Candidate: a volume spike patched in and out of the open bar.
    println!("== Aday (açık barda hacim sıçraması yamayla eklenip geri alınır)");
    let k = 60;
    let j = n - 2; // the open bar
    let mk = |spike: bool| {
        let mut s = Session::new(init(&root, &bars, k, 1), &root).unwrap();
        hello(&mut s, "t", "ana", None);
        for (i, b) in bars.iter().enumerate().take(j + 1).skip(k) {
            s.data_event(&ev(i - k + 1, b)).unwrap();
        }
        let mut seq = j - k + 2;
        let mut cand = Vec::new();
        if spike {
            s.data_event(&patch(seq, bars[j].t, json!({"v": bars[j].v * 50.0, "c": bars[j].c * 1.2, "h": bars[j].h * 1.2}))).unwrap();
            seq += 1;
            let mut live = bars[..=j].to_vec();
            live[j].v *= 50.0;
            live[j].c *= 1.2;
            live[j].h *= 1.2;
            cand = qp_session::capsule::candidates("THYAO", "px", &live)
                .into_iter()
                .map(|c| c.capsule)
                .collect();
            s.data_event(&patch(seq, bars[j].t, json!({"v": bars[j].v, "c": bars[j].c, "h": bars[j].h}))).unwrap();
            seq += 1;
        }
        s.data_event(&ev(seq, &bars[j + 1])).unwrap();
        (s, cand)
    };
    let (s1, cand) = mk(true);
    let (s0, _) = mk(false);
    let engine_entries = |s: &Session| -> Vec<String> {
        s.log()
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .filter(|v| !v.to_string().contains("\"patch\""))
            .map(|v| {
                let mut v = v;
                if let Some(m) = v.as_object_mut() {
                    m.remove("seq");
                }
                v.to_string()
            })
            .filter(|l| l.contains("cap.") || l.contains("ledger"))
            .collect()
    };
    let d1 = s1.document();
    let d0 = s0.document();
    let new_cands: Vec<&String> = cand
        .iter()
        .filter(|c| !d0.capsules.iter().any(|x| &&x.id == c))
        .collect();
    let log1 = serde_json::to_string(s1.log()).unwrap();
    println!(
        "  yamalı açık barda aday {} (kapanışta var olmayan {}: {:?})",
        cand.len(),
        new_cands.len(),
        new_cands
    );
    println!(
        "  kapsüller eşit {} | karne eşit {} | olgular eşit {} | günlükteki motor kayıtları eşit {} | günlükte yeni aday kimliği {} | günlükte \"candidate\" {}",
        serde_json::to_string(&d1.capsules).unwrap() == serde_json::to_string(&d0.capsules).unwrap(),
        serde_json::to_string(&d1.ledger).unwrap() == serde_json::to_string(&d0.ledger).unwrap(),
        serde_json::to_string(&d1.facts).unwrap() == serde_json::to_string(&d0.facts).unwrap(),
        engine_entries(&s1) == engine_entries(&s0),
        new_cands.iter().any(|c| log1.contains(c.as_str())),
        log1.contains("\"candidate\"")
    );
}

fn json_diff(a: &Value, b: &Value, path: &str, out: &mut std::collections::BTreeMap<String, usize>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()).collect::<BTreeSet<_>>() {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => json_diff(p, q, k, out),
                    _ => *out.entry(format!("{k}(var/yok)")).or_default() += 1,
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (p, q) in x.iter().zip(y) {
                json_diff(p, q, path, out);
            }
        }
        _ if a != b => *out.entry(path.to_owned()).or_default() += 1,
        _ => {}
    }
}
