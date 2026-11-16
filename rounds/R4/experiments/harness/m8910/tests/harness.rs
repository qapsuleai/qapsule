//! End-to-end test of the harness with the references only (no browser, no agent): S8 is
//! found, M8/M9/M10 pages, keys and prompts are built (screenshots skipped), fake readers
//! answer, the scores and the selection follow, and re-scoring is byte-identical.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use m8910::{io, m10, m8, m9, readers, s8, score};

fn tmp(name: &str) -> PathBuf {
    let d = io::scratch().join("test").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn answer(dir: &Path, v: &Value) {
    let raw = json!({"type": "result", "result": format!("İşte cevabım:\n```json\n{v}\n```")});
    io::write(&dir.join("answer.json"), raw.to_string().as_bytes()).unwrap();
}

fn reader(exp: &Path, e: &str, persona: &str, page: usize, k: u64) -> PathBuf {
    readers::setup(
        exp,
        &readers::Reader {
            experiment: e.into(),
            design: "r3".into(),
            persona: persona.into(),
            page,
            k,
            seed: k,
            model: "sonnet".into(),
        },
    )
    .unwrap()
}

#[test]
#[allow(clippy::too_many_lines)]
fn end_to_end_with_references() {
    let exp = tmp("e2e");
    let s = s8::find(u64::MAX).unwrap();
    assert!(
        s.states.iter().filter(|x| x.found).count() >= 10,
        "{:?}",
        s.states
            .iter()
            .map(|x| (x.state, x.found))
            .collect::<Vec<_>>()
    );
    for st in s.states.iter().filter(|x| x.found) {
        assert!(st.key.is_some());
        assert_eq!(st.key.as_ref().unwrap().s5, s8::S5[3]);
    }
    let s8p = exp.join("harness/s8.json");
    io::write_json(&s8p, &s).unwrap();

    // M8: pages twice → same computed measures, keys, prompts.
    let a = m8::sheets("r3", &s, &exp, 1, false).unwrap();
    let c1 = io::read(&exp.join("M8/raw/r3/computed.json")).unwrap();
    let b = m8::sheets("r3", &s, &exp, 1, false).unwrap();
    assert_eq!(a, b);
    assert_eq!(c1, io::read(&exp.join("M8/raw/r3/computed.json")).unwrap());
    let comp: m8::Computed = serde_json::from_str(&c1).unwrap();
    assert!(!comp.faces.is_empty());
    for (page, code) in &a.codes {
        let pk: m8::PageKey =
            serde_json::from_str(&io::read(&exp.join(format!("M8/raw/keys/{code}.json"))).unwrap())
                .unwrap();
        // Perfect reader (F), wrong reader (H: all "bilmiyorum").
        let d = reader(&exp, "m8", "F", *page, 1);
        let t = io::read(&d.join("prompt.template.txt")).unwrap();
        assert!(t.contains("@DIR@/") && !t.contains("r3"));
        let mut good = serde_json::Map::new();
        let mut bad = serde_json::Map::new();
        for (l, lk) in &pk.labels {
            let k = &lk.key;
            good.insert(l.clone(), json!({"S1": k.s1.to_uppercase(), "S1b": k.s1b, "S2": k.s2, "S3": k.s3, "S4": k.s4, "S5": k.s5}));
            bad.insert(l.clone(), json!({"S1": "anlamadım", "S1b": "anlamadım", "S2": "bilmiyorum", "S3": "bilmiyorum", "S4": "hiçbir bilgi göremedim", "S5": "al"}));
        }
        answer(&d, &Value::Object(good));
        answer(&reader(&exp, "m8", "H", *page, 1), &Value::Object(bad));
    }
    // A critic over the one design.
    let cdir = m8::critic(&exp, 1).unwrap();
    let ck: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&io::read(&cdir.join("key.json")).unwrap()).unwrap();
    let letter = ck.keys().next().unwrap().clone();
    io::write(
        &cdir.join("answer.json"),
        json!({"result": json!({"puanlar": {letter.clone(): {"1": 3, "2": 3, "3": 2, "4": 4, "5": 2, "6": 3, "7": 5, "8": 2}}, "sira": [letter]}).to_string()})
            .to_string()
            .as_bytes(),
    )
    .unwrap();
    let md1 = score::run(&exp, "m8").unwrap();
    let j1 = io::read(&exp.join("M8/raw/scores.json")).unwrap();
    let md2 = score::run(&exp, "m8").unwrap();
    assert_eq!(md1, md2);
    assert_eq!(j1, io::read(&exp.join("M8/raw/scores.json")).unwrap());
    let v: Value = serde_json::from_str(&j1).unwrap();
    assert_eq!(v["designs"]["r3"]["persona"]["F"]["full"], json!(1.0));
    assert_eq!(v["designs"]["r3"]["persona"]["H"]["full"], json!(0.0));
    assert_eq!(v["designs"]["r3"]["critic_mean_rank"], json!(1.0));
    // The reference is never selected.
    assert!(v["selection"]["selected"].is_null());

    // M9.
    let sc = m9::scenario().unwrap();
    let code = m9::screens("r3", &sc, &exp, 1, false).unwrap();
    let k: m9::Key =
        serde_json::from_str(&io::read(&exp.join(format!("M9/raw/keys/{code}.json"))).unwrap())
            .unwrap();
    assert_ne!(k.g2_which, "hiçbirinde");
    // Positions as the screenshot script writes them (every label on screen 1).
    let labs: serde_json::Map<String, Value> =
        k.labels.values().map(|l| (l.clone(), json!([1]))).collect();
    io::write_json(
        &exp.join(format!("M9/raw/keys/{code}.positions.json")),
        &json!({"screens": 1, "labels": labs}),
    )
    .unwrap();
    let d = reader(&exp, "m9", "F", 1, 1);
    answer(
        &d,
        &json!({"G1": {"cevap": k.g1, "ekran": [1], "acmam_gerekirdi": []},
        "G2": {"hangisi": k.g2_which, "ne": k.g2_what, "ekran": [1], "acmam_gerekirdi": []},
        "G3": {"cevap": k.g3.first().cloned().unwrap_or_default(), "ekran": [1], "acmam_gerekirdi": ["XX"]}}),
    );
    score::run(&exp, "m9").unwrap();
    let v: Value =
        serde_json::from_str(&io::read(&exp.join("M9/raw/scores.json")).unwrap()).unwrap();
    assert_eq!(v["designs"]["r3"]["persona"]["F"]["g1_f1"], json!(1.0));
    assert_eq!(v["designs"]["r3"]["persona"]["F"]["g2"], json!(1.0));

    // M10.
    let p = m10::screens("r3", &s, &exp, 1, false).unwrap();
    assert!(!p.states.is_empty());
    for st in &p.states {
        assert_eq!(st.regions.len(), 4);
        assert_eq!(st.regions.iter().filter(|r| r.correct).count(), 1);
        assert_eq!(st.taps, 2, "open + tap");
    }
    let d = reader(&exp, "m10", "H", 1, 1);
    let meta: Value = serde_json::from_str(&io::read(&d.join("meta.json")).unwrap()).unwrap();
    let key: m10::Key = serde_json::from_str(
        &io::read(&exp.join(format!("M10/raw/keys/{}.json", p.code))).unwrap(),
    )
    .unwrap();
    let mut ans = serde_json::Map::new();
    for (pos, idx) in meta["order"].as_array().unwrap().iter().enumerate() {
        let (_, g1, g2, g3) = &key.seqs[idx.as_str().unwrap()];
        ans.insert((pos + 1).to_string(), json!({"G1": g1, "G2": g2, "G3": g3}));
    }
    answer(&d, &Value::Object(ans));
    score::run(&exp, "m10").unwrap();
    let v: Value =
        serde_json::from_str(&io::read(&exp.join("M10/raw/scores.json")).unwrap()).unwrap();
    assert_eq!(v["designs"]["r3"]["persona"]["H"]["g1"], json!(1.0));
    assert_eq!(v["designs"]["r3"]["persona"]["H"]["g3"], json!(1.0));
}

#[test]
fn prompts_reject_design_names() {
    assert!(m8910::blind::check("x", "Bu ekranda r3 yazıyor").is_err());
    assert!(m8910::blind::check("x", "Ekrandaki kartlar").is_ok());
}

/// SPEC E1: the S4 "few cases" bucket is the product's threshold (`n <
/// GLYPH_MIN_LEDGER_N`, 20), not `n < 5`; the committed `s8.json` keys follow it.
#[test]
fn s4_few_cases_is_the_products_threshold() {
    let s8: Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../s8.json")).unwrap(),
    )
    .unwrap();
    let few = "az örnek var, bilinmiyor";
    let mut n_few = 0;
    for st in s8["states"].as_array().unwrap() {
        if st["found"] != json!(true) {
            continue;
        }
        let item: qp_schema::protocol::CapsuleItem =
            serde_json::from_value(st["item"].clone()).unwrap();
        let n = item.ledger_at_birth.n;
        assert_eq!(s8::trust_answer(&item), st["key"]["s4"].as_str().unwrap());
        assert_eq!(
            st["key"]["s4"] == json!(few),
            n < qp_grammar::depth::GLYPH_MIN_LEDGER_N,
            "state {} n {n}",
            st["state"]
        );
        n_few += usize::from(n < 20);
    }
    assert!(n_few > 1);
}

/// H-FP: the shell's replica gives a face design's headline template the same headline
/// pieces as the authority for the same capsule item (same story, bars, "now" and
/// calendar), in TR and EN; the S2-relevant phrase follows the key's day buckets.
#[test]
fn replica_headline_parts_equal_the_authoritys() {
    use m8910::world::{Core, Kind, World};
    use qp_schema::protocol::{CapsuleItem, Language};
    let s = m8::read_s8(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../s8.json")).unwrap();
    let mut sts: Vec<&s8::State> = s
        .states
        .iter()
        .filter(|x| x.found && Kind::from_name(&x.world) == Some(Kind::Demo))
        .collect();
    sts.sort_by_key(|x| (x.step, x.state));
    let mut w = World::new(Kind::Demo).unwrap();
    let mut c = Core::connect(&mut w, "hfp", m8910::READER, "tr").unwrap();
    let mut n = 0;
    for st in sts {
        if w.step() < st.step {
            c.seek(&mut w, st.step);
        }
        let item: CapsuleItem = serde_json::from_value(st.item.clone()).unwrap();
        for lang in [Language::Tr, Language::En] {
            let a = w.host.session().item_headline_parts(&item, lang);
            let r = c
                .core
                .replica_mut()
                .session_mut()
                .and_then(|s| s.item_headline_parts(&item, lang));
            assert!(a.is_some(), "state {}", st.state);
            assert_eq!(a, r, "state {} {lang:?}", st.state);
            n += 1;
        }
        let tr = w
            .host
            .session()
            .item_headline_parts(&item, Language::Tr)
            .unwrap();
        let s2 = st.key.as_ref().unwrap().s2.as_str();
        if s2 == "dün" {
            assert_eq!(tr.moved, "dün", "state {}", st.state);
        }
        if item.redacted {
            assert!(!tr.moved.chars().any(|c| c.is_ascii_digit()));
            assert_eq!(tr.ledger_n, None);
        }
    }
    assert!(n >= 10);
}
