//! End-to-end tests for the `qp` binary.

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
