//! Harness self-tests: the same alternative and seed give byte-identical raw files; scoring
//! is a pure function of the raw files. Writes only under Cargo's test temp directory.

use std::fs;
use std::path::{Path, PathBuf};

use harness::{io, m1, m2};

fn root(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&p);
    p
}

fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((
                    p.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&p).unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn m1_sheets_are_byte_identical() {
    let (a, b) = (root("m1a"), root("m1b"));
    let e = m1::registry::by_name("control").unwrap();
    for r in [&a, &b] {
        for c in m1::sheets::CONDITIONS {
            m1::sheets::write_condition(&e, r, c, 7).unwrap();
        }
    }
    let (fa, fb) = (files(&a), files(&b));
    assert_eq!(fa.len(), 10); // 4 png + 3 prompts + 3 keys
    assert_eq!(fa, fb);
    // another seed changes the sheets
    let c = root("m1c");
    m1::sheets::write_condition(&e, &c, "nolegend", 8).unwrap();
    let png = |r: &Path, s: u64| {
        fs::read(m1::raw_dir(r, "control").join(format!("sheets/nolegend_s{s}/sheet.png"))).unwrap()
    };
    assert_ne!(png(&a, 7), png(&c, 8));
}

#[test]
fn m1_measure_is_deterministic_and_control_has_zero_channels() {
    let e = m1::registry::by_name("control").unwrap();
    let r1 = io::json(&m1::distinct::measure(&e).unwrap()).unwrap();
    let r2 = io::json(&m1::distinct::measure(&e).unwrap()).unwrap();
    assert_eq!(r1, r2);
    let r = m1::distinct::measure(&e).unwrap();
    assert_eq!(r.conditions.len(), 30);
    // the control encodes only the kind: every channel pair and same-kind K pair is 0
    assert!(r.main.pair.d.abs() < 1e-9);
    for w in r.channel_worst.values() {
        assert!(w.pair.d.abs() < 1e-9);
    }
    assert!(r.k_mean_worst > 0.0);
    assert_eq!(r.out_of_box_prims, 0);
}

fn fake_reader(raw: &Path, exp: &str, cond_key: &str, cond: &str, seed: u64, answer: &str) {
    let run = raw.join("readers").join(format!("{cond}_s{seed}_r1"));
    fs::create_dir_all(&run).unwrap();
    fs::write(
        run.join("meta.json"),
        format!(
            "{{\"experiment\": \"{exp}\", \"alt\": \"control\", \"{cond_key}\": \"{cond}\", \"seed\": {seed}, \"reader\": \"r1\", \"model\": \"test\"}}"
        ),
    )
    .unwrap();
    let wrapped = serde_json::json!({"type": "result", "result": answer}).to_string();
    fs::write(run.join("answer.json"), wrapped).unwrap();
}

#[test]
fn m1_scoring_is_pure_and_correct() {
    let r = root("m1score");
    let e = m1::registry::by_name("control").unwrap();
    m1::sheets::write_condition(&e, &r, "nolegend", 3).unwrap();
    let raw = m1::raw_dir(&r, "control");
    let key: m1::sheets::Key =
        serde_json::from_str(&fs::read_to_string(raw.join("keys/nolegend_s3.json")).unwrap())
            .unwrap();
    // a perfect answer, with prose around it
    let mut obj = serde_json::Map::new();
    for t in &key.items {
        obj.insert(
            t.label.clone(),
            serde_json::json!({"kind": t.kind, "direction": t.direction, "strength": t.strength, "phase": t.phase}),
        );
    }
    let ans = format!(
        "Here you go:\n```json\n{}\n```",
        serde_json::Value::Object(obj)
    );
    fake_reader(&raw, "m1", "condition", "nolegend", 3, &ans);
    let s = m1::score::run(&r, "control").unwrap();
    assert_eq!(s.conditions["nolegend"].accuracy["full"], 1.0);
    let first = fs::read(raw.join("scores.json")).unwrap();
    m1::score::run(&r, "control").unwrap();
    assert_eq!(first, fs::read(raw.join("scores.json")).unwrap());
}

#[test]
fn m2_measure_and_sheets_are_deterministic() {
    let r = m2::registry::by_name("control").unwrap();
    let a = io::json(&m2::metrics::measure(r.as_ref())).unwrap();
    let b = io::json(&m2::metrics::measure(r.as_ref())).unwrap();
    assert_eq!(a, b);
    let auto = m2::metrics::measure(r.as_ref());
    for l in ["tr", "en"] {
        let s = &auto.summary[l];
        assert_eq!(s.sentences, 73);
        assert_eq!(s.determinism, 1.0);
        // machine codes carry digits: every redacted situation leaks
        assert_eq!(s.leaks, s.redacted);
        assert_eq!(s.redacted, 18);
        // ... but every number is an engine slot (bars ago)
        assert_eq!(s.invented_numbers, 0);
        assert!(!s.k26_correct);
    }
    let (x, y) = (root("m2a"), root("m2b"));
    for d in [&x, &y] {
        m2::sheets::write_sheet(r.as_ref(), d, qp_capsule::Lang::Tr, 5).unwrap();
        m2::sheets::write_sheet(r.as_ref(), d, qp_capsule::Lang::En, 5).unwrap();
    }
    assert_eq!(files(&x), files(&y));
}

#[test]
fn m2_scoring_is_pure_and_correct() {
    let d = root("m2score");
    let r = m2::registry::by_name("control").unwrap();
    m2::sheets::write_sheet(r.as_ref(), &d, qp_capsule::Lang::En, 2).unwrap();
    let raw = m2::raw_dir(&d, "control");
    let key: m2::sheets::Key =
        serde_json::from_str(&fs::read_to_string(raw.join("keys/en_s2.json")).unwrap()).unwrap();
    let mut obj = serde_json::Map::new();
    for it in &key.items {
        let t = &it.truth;
        obj.insert(
            it.label.clone(),
            serde_json::json!({"kind": t.kind, "direction": t.direction, "when": t.when, "bars_ago": t.bars_ago, "still_valid": t.still_valid}),
        );
    }
    fake_reader(
        &raw,
        "m2",
        "lang",
        "en",
        2,
        &serde_json::Value::Object(obj).to_string(),
    );
    let s = m2::score::run(&d, "control").unwrap();
    assert_eq!(s.langs["en"].accuracy["full"], 1.0);
    assert_eq!(s.langs["en"].k26_full, "1/1");
    let first = fs::read(raw.join("scores.json")).unwrap();
    m2::score::run(&d, "control").unwrap();
    assert_eq!(first, fs::read(raw.join("scores.json")).unwrap());
}
