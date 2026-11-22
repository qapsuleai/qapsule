//! End-to-end tests for the `qp` binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn qp() -> Command {
    Command::cargo_bin("qp").expect("qp binary is built")
}

fn schema_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".schema.json"))
        .collect();
    files.sort();
    files
}

#[test]
fn version_prints() {
    let out = qp().arg("--version").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.starts_with("qp "), "{stdout}");
}

#[test]
fn schema_export_is_complete_and_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    for dir in [&a, &b] {
        qp().args(["schema", "export", "--out"])
            .arg(dir)
            .assert()
            .success();
    }
    let fa = schema_files(&a);
    let fb = schema_files(&b);
    assert!(
        fa.len() >= 7,
        "expected >= 7 schema files, got {}",
        fa.len()
    );
    assert_eq!(fa.len(), fb.len());
    for (pa, pb) in fa.iter().zip(&fb) {
        assert_eq!(pa.file_name(), pb.file_name());
        assert_eq!(std::fs::read(pa).unwrap(), std::fs::read(pb).unwrap());
    }
}

#[test]
fn schema_validate_examples_builtin_and_exported() {
    let tmp = tempfile::tempdir().unwrap();
    qp().args(["schema", "export", "--out"])
        .arg(tmp.path())
        .assert()
        .success();
    for (file, ty) in [
        ("capsule.example.json", "Capsule"),
        ("document.example.json", "Document"),
    ] {
        let path = fixtures().join("schema").join(file);
        qp().args(["schema", "validate"])
            .arg(&path)
            .args(["--type", ty])
            .assert()
            .success()
            .stdout("valid\n");
        qp().args(["schema", "validate"])
            .arg(&path)
            .args(["--type", ty, "--schema-dir"])
            .arg(tmp.path())
            .assert()
            .success()
            .stdout("valid\n");
    }
}

#[test]
fn schema_validate_rejects_invalid_instance() {
    let tmp = tempfile::tempdir().unwrap();
    let text = std::fs::read_to_string(fixtures().join("schema/capsule.example.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["state"] = serde_json::json!("not-a-state");
    let bad = tmp.path().join("bad.json");
    std::fs::write(&bad, serde_json::to_string(&value).unwrap()).unwrap();

    qp().args(["schema", "validate"])
        .arg(&bad)
        .args(["--type", "Capsule"])
        .assert()
        .code(1);

    let schemas = tmp.path().join("schemas");
    qp().args(["schema", "export", "--out"])
        .arg(&schemas)
        .assert()
        .success();
    qp().args(["schema", "validate"])
        .arg(&bad)
        .args(["--type", "Capsule", "--schema-dir"])
        .arg(&schemas)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_unknown_type_fails() {
    qp().args(["schema", "validate"])
        .arg(fixtures().join("schema/capsule.example.json"))
        .args(["--type", "Nope"])
        .assert()
        .failure();
}

#[test]
fn render_is_deterministic_and_matches_golden() {
    let tmp = tempfile::tempdir().unwrap();
    let input = fixtures().join("bars/sample.json");
    let a = tmp.path().join("a.png");
    let b = tmp.path().join("b.png");
    for out in [&a, &b] {
        qp().arg("render")
            .arg(&input)
            .arg("-o")
            .arg(out)
            .assert()
            .success();
    }
    let ba = std::fs::read(&a).unwrap();
    let bb = std::fs::read(&b).unwrap();
    assert_eq!(ba, bb, "two renders differ");
    let golden = std::fs::read(fixtures().join("golden/sample.png")).unwrap();
    assert!(
        ba == golden,
        "render does not match fixtures/golden/sample.png"
    );
}

/// RD2/G: `--scale 1` is the reference path byte for byte; `--scale 2` doubles both sides
/// (PNG IHDR width/height); 4 is refused (the gallery needs 1..=3).
#[test]
fn render_scale_keeps_scale_one_and_doubles_at_two() {
    let tmp = tempfile::tempdir().unwrap();
    let input = fixtures().join("bars/sample.json");
    let one = tmp.path().join("one.png");
    let two = tmp.path().join("two.png");
    for (out, s) in [(&one, "1"), (&two, "2")] {
        qp().arg("render")
            .arg(&input)
            .args(["--width", "300", "--height", "200", "--scale", s, "-o"])
            .arg(out)
            .assert()
            .success();
    }
    let plain = tmp.path().join("plain.png");
    qp().arg("render")
        .arg(&input)
        .args(["--width", "300", "--height", "200", "-o"])
        .arg(&plain)
        .assert()
        .success();
    assert_eq!(std::fs::read(&one).unwrap(), std::fs::read(&plain).unwrap());
    let png = std::fs::read(&two).unwrap();
    let be = |i: usize| u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
    assert_eq!((be(16), be(20)), (600, 400));
    qp().arg("render")
        .arg(&input)
        .args(["--scale", "4", "-o"])
        .arg(tmp.path().join("four.png"))
        .assert()
        .failure();
}

/// RD2/G: `--theme` reaches the compiler (a light render of a view differs from dark).
#[test]
fn render_theme_overrides_the_scheme() {
    let tmp = tempfile::tempdir().unwrap();
    let root = fixtures().join("..");
    let mut pngs = Vec::new();
    for t in ["dark", "light"] {
        let out = tmp.path().join(format!("{t}.png"));
        qp().current_dir(&root)
            .args([
                "render",
                "--transcript",
                "fixtures/views/v1-three-panels.json",
            ])
            .args([
                "--view", "three", "--width", "240", "--height", "160", "--theme", t, "-o",
            ])
            .arg(&out)
            .assert()
            .success();
        pngs.push(std::fs::read(&out).unwrap());
    }
    assert_ne!(pngs[0], pngs[1]);
}

#[test]
fn bad_input_fails_without_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();
    let out = tmp.path().join("out.png");

    for input in [bad.clone(), tmp.path().join("missing.json")] {
        let assert = qp()
            .arg("render")
            .arg(&input)
            .arg("-o")
            .arg(&out)
            .assert()
            .failure();
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(!stderr.contains("panicked"), "{stderr}");
        assert!(!stderr.is_empty());
    }
    assert!(!out.exists());

    let assert = qp()
        .args(["schema", "validate"])
        .arg(&bad)
        .args(["--type", "Capsule"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(!stderr.contains("panicked"), "{stderr}");
}

fn compute_json(args: &[&str]) -> serde_json::Value {
    let out = qp().arg("compute").args(args).assert().success();
    serde_json::from_slice(&out.get_output().stdout).unwrap()
}

fn nulls_then_numbers(col: &serde_json::Value) -> (usize, usize) {
    let a = col.as_array().unwrap();
    let warm = a.iter().take_while(|v| v.is_null()).count();
    assert!(
        a[warm..].iter().all(serde_json::Value::is_f64),
        "null after warmup"
    );
    (warm, a.len())
}

#[test]
fn compute_rsi_and_macd_from_a_series_file() {
    let sample = fixtures().join("bars/sample.json");
    let s = sample.to_str().unwrap();

    let rsi = compute_json(&[s, "--study", "rsi", "--param", "period=14"]);
    qp_schema::validate("StudyOutput", &rsi).unwrap();
    assert_eq!(rsi["t"].as_array().unwrap().len(), 200);
    assert_eq!(nulls_then_numbers(&rsi["outputs"]["rsi"]), (14, 200));
    assert_eq!(rsi["warmup"]["rsi"], 14);

    let macd = compute_json(&[s, "--study", "macd"]);
    qp_schema::validate("StudyOutput", &macd).unwrap();
    let outs = macd["outputs"].as_object().unwrap();
    assert_eq!(outs.len(), 3);
    assert_eq!(nulls_then_numbers(&outs["macd"]), (25, 200));
    assert_eq!(nulls_then_numbers(&outs["signal"]), (33, 200));
    assert_eq!(nulls_then_numbers(&outs["histogram"]), (33, 200));
    assert_eq!(macd["params"]["slow"], 26.0);

    // Same numbers as the library.
    let bars = qp_schema::parse_bar_series(&std::fs::read_to_string(&sample).unwrap()).unwrap();
    let lib = qp_compute::compute_bars(
        "rsi",
        &std::collections::BTreeMap::from([("period".to_owned(), 14.0)]),
        qp_compute::Field::Close,
        bars.bars().unwrap(),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&lib).unwrap(), rsi);
}

#[test]
fn compute_from_a_document_series() {
    let doc = fixtures().join("schema/document.example.json");
    let v = compute_json(&[
        doc.to_str().unwrap(),
        "--study",
        "sma",
        "--param",
        "period=2",
        "--series",
        "thyao-1d",
    ]);
    assert_eq!(
        v["outputs"]["sma"],
        serde_json::json!([null, 320.125, 326.5])
    );
    // A matriks binding without data is not available offline.
    qp().args([
        "compute",
        doc.to_str().unwrap(),
        "--study",
        "sma",
        "--series",
        "thyao-1d-rsi14",
    ])
    .assert()
    .code(2);
}

#[test]
fn compute_sweep_equals_single_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let sets = tmp.path().join("sets.json");
    std::fs::write(
        &sets,
        r#"[{"period":10,"k":2},{"period":20,"k":1.5},{"period":5}]"#,
    )
    .unwrap();
    let sample = fixtures().join("bars/sample.json");
    let s = sample.to_str().unwrap();
    let all = compute_json(&[s, "--study", "bollinger", "--sweep", sets.to_str().unwrap()]);
    let all = all.as_array().unwrap();
    assert_eq!(all.len(), 3);
    let single = compute_json(&[
        s,
        "--study",
        "bollinger",
        "--param",
        "period=20",
        "--param",
        "k=1.5",
    ]);
    assert_eq!(all[1], single);
    assert_eq!(all[2]["params"]["k"], 2.0); // default filled in
}

#[test]
fn compute_errors_are_reported() {
    let s = fixtures().join("bars/sample.json");
    let s = s.to_str().unwrap();
    qp().args(["compute", s, "--study", "nope"])
        .assert()
        .code(2);
    qp().args(["compute", s, "--study", "rsi", "--param", "period=0"])
        .assert()
        .code(2);
    qp().args(["compute", s, "--study", "rsi", "--param", "length=3"])
        .assert()
        .code(2);
    qp().args(["compute", s, "--study", "rsi", "--field", "vwap"])
        .assert()
        .code(2);
    qp().args(["compute", s, "--study", "rsi", "--series", "x"])
        .assert()
        .code(2);
    let list = compute_json(&["--list"]);
    assert_eq!(list.as_array().unwrap().len(), 5);
}
