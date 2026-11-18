//! R5/A (denetçi): kendi girişimsizlik denemem.
//!
//! Var olan testlerin (`non_interference.rs`, `stories.rs`) kapsamadığı bileşimler:
//! - akış kuralı (K53, `CoinFlipRule`) AÇIKKEN (ürünün gösterim varsayılanı `rate55`, K42 muaf;
//!   muafiyetsiz; ve çok gizleyen bir sınama kuralı) yetkisiz okurun baytları lisanslı akışa
//!   (tohum 1, 2, 42, 7) göre değişiyor mu — `hidden`, `coin_flip`, `in_feed` dolaylı kanal mı?
//! - sesler (`marla`, `selvi`) + hikâye listesi + `capsules {all}` + `ledger` + `describe` aynı
//!   koşuda, duyarlılık `all` (gizlenenler de listede).
//! - yetkisiz (hiç veri yetkisi yok) okurun metin alanlarında rakam (simge çıkarıldıktan sonra).
//! Karşılaştırma: sunucu yorumları (mühür; Ö37'nin tek kanalı) çıkarılınca bayt eşitlik.
//! Olumlu denetim: tam yetkili okurun satırları tohumla değişmeli.

use std::path::PathBuf;

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::{CoinFlipRule, Session};
use serde_json::{json, Value};

const BARS_SEED: u64 = 7;
const K: usize = 300;
const N: usize = 420;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn init(seed: u64) -> SessionInit {
    let flow = if seed == 0 { json!([]) } else { json!([{"series": "px", "seed": seed}]) };
    serde_json::from_value(json!({
        "document": {
            "schema_version": "0", "id": "doc-audit", "title": "Denetim",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
            {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": BARS_SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"], "bora": ["bist/THYAO"], "can": []}
        },
        "capsules": {"series": ["px"], "flow": flow}
    }))
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn run(seed: u64, rule: Option<CoinFlipRule>, voice: &str) -> Vec<OutboundLine> {
    let mut s = Session::new(init(seed), &root()).unwrap();
    s.set_coin_flip_rule(rule);
    let mut out = Vec::new();
    for (c, who, lang) in [("t", "ana", "tr"), ("b", "bora", "tr"), ("c", "can", "en"), ("d", "can", "tr")] {
        out.extend(send(&mut s, c, &json!({"type":"hello","protocol":"qp/0","principal":who,
            "surface":{"kind":"phone","lang":lang}})));
        out.extend(send(&mut s, c, &json!({"type":"input","id":"v","gesture":{"kind":"style","voice":voice}})));
        out.extend(send(&mut s, c, &json!({"type":"command","id":"aud","ops":[{"op":"set_audience",
            "audience":{"group":"future","sensitivity":"all","watchlist":["THYAO"]}}]})));
        for (i, t) in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","all":true}),
            json!({"kind":"capsules","instrument":"THYAO"}),
            json!({"kind":"changes"}),
            json!({"kind":"claims"}),
            json!({"kind":"stories","instrument":"THYAO"}),
        ].iter().enumerate() {
            out.extend(send(&mut s, c, &json!({"type":"subscribe","id":format!("s{i}"),"topic":t})));
        }
    }
    let bars = qp_source::synthetic_bars(N, BARS_SEED);
    for (i, b) in bars.iter().enumerate().take(N).skip(K) {
        let ev: DataEvent = serde_json::from_value(json!({"feed": feed(), "seq": i - K + 1,
            "change": {"kind":"bar","bar": b}})).unwrap();
        out.extend(s.data_event(&ev).unwrap());
    }
    for c in ["t", "b", "c", "d"] {
        for q in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","all":true}),
            json!({"kind":"capsules","open":true}),
            json!({"kind":"capsules","audience":true}),
            json!({"kind":"ledger"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"describe"}),
            json!({"kind":"stories","instrument":"THYAO","limit":500}),
            json!({"kind":"stories","instrument":"THYAO","range":{"last":60},"limit":5}),
        ] {
            out.extend(send(&mut s, c, &json!({"type":"query","id":"q","query":q})));
        }
    }
    out
}

fn strip_comments(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("comments");
            for x in m.values_mut() {
                strip_comments(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(strip_comments),
        _ => {}
    }
}

fn stripped(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .filter_map(|l| {
            let mut v = serde_json::to_value(&l.msg).unwrap();
            if v.pointer("/event/kind") == Some(&json!("server_comment")) {
                return None;
            }
            strip_comments(&mut v);
            Some(serde_json::to_string(&v).unwrap())
        })
        .collect()
}

fn raw(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter().filter(|l| l.conn == conn).map(|l| serde_json::to_string(&l.msg).unwrap()).collect()
}

/// Every string under a key that carries reader-facing words.
fn texts(v: &Value, key: &str, acc: &mut Vec<(String, String)>) {
    const TEXT_KEYS: &[&str] = &["headline", "detail", "ledger", "face", "text", "sentence", "tr", "en",
        "event", "direction_word", "when", "moved", "validity_phrase", "ledger_short", "title", "term"];
    match v {
        Value::Object(m) => m.iter().for_each(|(k, x)| texts(x, k, acc)),
        Value::Array(a) => a.iter().for_each(|x| texts(x, key, acc)),
        Value::String(s) if TEXT_KEYS.contains(&key) => acc.push((key.to_owned(), s.clone())),
        _ => {}
    }
}

fn main() {
    let product = qp_capsule::feedrule::HideRule::by_name("rate55").unwrap();
    let rules: Vec<(&str, Option<CoinFlipRule>)> = vec![
        ("none", None),
        ("rate55+exempt", Some(CoinFlipRule { hide: product, exempt_negative: true })),
        ("rate55", Some(CoinFlipRule { hide: product, exempt_negative: false })),
        ("test-n4-t0.6", Some(CoinFlipRule {
            hide: qp_capsule::feedrule::HideRule::Rate { min_n: 4, tau: 0.6 }, exempt_negative: false })),
    ];
    let mut fails = 0;
    if let Ok(dir) = std::env::var("NI_DUMP") {
        let ri: usize = std::env::var("NI_RULE").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let base = run(7, rules[ri].1, "marla");
        for c in ["b", "c", "d"] {
            std::fs::write(format!("{dir}/{c}.jsonl"), stripped(&base, c).join("\n")).unwrap();
        }
    }
    for voice in ["marla", "selvi"] {
        for (rn, rule) in &rules {
            let base = run(7, *rule, voice);
            let mut ana_differs = false;
            let mut hidden_seen = 0usize;
            for seed in [1_u64, 2, 42, 0] {
                let other = run(seed, *rule, voice);
                for c in ["b", "c", "d"] {
                    let (x, y) = (stripped(&base, c), stripped(&other, c));
                    if x != y {
                        fails += 1;
                        let i = x.iter().zip(&y).position(|(a, b)| a != b).unwrap_or(x.len().min(y.len()));
                        println!("FAIL {voice} {rn} seed7 vs {seed} conn {c}: len {} vs {}, first diff line {i}", x.len(), y.len());
                    }
                }
                ana_differs |= raw(&base, "t") != raw(&other, "t");
            }
            // digit scan on the reader with no data entitlement (c: en, d: tr)
            let mut digits = 0usize;
            let mut ntexts = 0usize;
            for c in ["c", "d"] {
                for l in stripped(&base, c) {
                    hidden_seen += l.matches("\"hidden\":true").count();
                    let v: Value = serde_json::from_str(&l).unwrap();
                    let mut acc = Vec::new();
                    texts(&v, "", &mut acc);
                    for (k, t) in acc {
                        ntexts += 1;
                        let t = t.replace("THYAO", "");
                        if t.chars().any(|ch| ch.is_ascii_digit()) {
                            digits += 1;
                            if digits <= 3 {
                                println!("  digit {voice} {rn} {c} {k}: {t}");
                            }
                        }
                    }
                }
            }
            if digits > 0 { fails += 1; }
            if !ana_differs { fails += 1; }
            println!("{voice:6} {rn:14} unentitled lines b/c/d = {}/{}/{}; hidden:true seen by c,d = {hidden_seen}; \
                      text fields (c,d) = {ntexts}, with digit = {digits}; entitled differs = {ana_differs}",
                stripped(&base, "b").len(), stripped(&base, "c").len(), stripped(&base, "d").len());
        }
    }
    println!("{}", if fails == 0 { "NI PROBE: PASS" } else { "NI PROBE: FAIL" });
    std::process::exit(i32::from(fails != 0));
}
