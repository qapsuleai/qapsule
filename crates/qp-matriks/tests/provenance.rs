//! The proto copy is byte-exact and matches its provenance record (K62).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn proto_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("proto")
}

fn provenance() -> serde_json::Value {
    let text = std::fs::read_to_string(proto_dir().join("provenance.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn recorded_files(p: &serde_json::Value) -> Vec<(String, u64, String)> {
    p["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["name"].as_str().unwrap().to_string(),
                f["bytes"].as_u64().unwrap(),
                f["sha256"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn record_metadata() {
    let p = provenance();
    assert_eq!(
        p["source_path"],
        "services/quantex-stream/matriks-adapter/src/proto"
    );
    assert_eq!(
        p["source_head_commit"],
        "f3d95aeaa5b8fc4858b3160acf882055fbf47f02"
    );
    assert_eq!(
        p["source_dir_last_commit"],
        "b4de978137caa53bd10306c8658f2034f7dbdf16"
    );
    assert_eq!(p["copied_on"], "2026-11-19");
    assert_eq!(
        p["file_count"].as_u64().unwrap() as usize,
        recorded_files(&p).len()
    );
    assert_eq!(recorded_files(&p).len(), 22);
}

#[test]
fn copy_matches_record() {
    let p = provenance();
    let recorded = recorded_files(&p);
    let on_disk: BTreeSet<String> = std::fs::read_dir(proto_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".proto"))
        .collect();
    let in_record: BTreeSet<String> = recorded.iter().map(|(n, _, _)| n.clone()).collect();
    assert_eq!(
        on_disk, in_record,
        "proto file set differs from provenance.json"
    );
    for (name, bytes, sha) in &recorded {
        let data = std::fs::read(proto_dir().join(name)).unwrap();
        assert_eq!(data.len() as u64, *bytes, "{name}: size");
        assert_eq!(&sha256_hex(&data), sha, "{name}: sha256");
    }
}

/// Compares against the source repo when it exists locally (read-only).
/// `QUANTEX_ROOT` or `~/LocalGit/quantex`; skipped (and says so) otherwise.
#[test]
fn copy_matches_local_source_if_present() {
    let p = provenance();
    let root = std::env::var_os("QUANTEX_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("LocalGit/quantex")));
    let Some(src) = root
        .map(|r| r.join(p["source_path"].as_str().unwrap()))
        .filter(|d| d.is_dir())
    else {
        eprintln!("SKIPPED copy_matches_local_source_if_present: quantex source not found (set QUANTEX_ROOT)");
        return;
    };
    let source_set: BTreeSet<String> = std::fs::read_dir(&src)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    let recorded = recorded_files(&p);
    let in_record: BTreeSet<String> = recorded.iter().map(|(n, _, _)| n.clone()).collect();
    assert_eq!(
        source_set, in_record,
        "source dir file set differs (source changed?)"
    );
    for (name, _, sha) in &recorded {
        let data = std::fs::read(src.join(name)).unwrap();
        assert_eq!(&sha256_hex(&data), sha, "{name}: source differs from copy");
    }
}
