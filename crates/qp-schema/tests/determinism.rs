//! Schema generation and export are deterministic.

use std::path::PathBuf;

fn render(schemas: &[(&'static str, serde_json::Value)]) -> Vec<(&'static str, String)> {
    schemas
        .iter()
        .map(|(n, v)| (*n, serde_json::to_string_pretty(v).unwrap()))
        .collect()
}

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qp-schema-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn json_schemas_is_deterministic() {
    let a = qp_schema::json_schemas();
    let b = qp_schema::json_schemas();
    assert_eq!(a, b);
    assert_eq!(render(&a), render(&b));
}

#[test]
fn json_schemas_has_required_roots_in_stable_order() {
    let names: Vec<&str> = qp_schema::json_schemas().iter().map(|(n, _)| *n).collect();
    for required in [
        "Capsule", "Document", "Series", "Fact", "Finding", "View", "Command",
    ] {
        assert!(names.contains(&required), "missing root {required}");
    }
    let mut dedup = names.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(dedup.len(), names.len(), "duplicate root names");
    for (name, schema) in qp_schema::json_schemas() {
        assert_eq!(schema["title"], name, "title of {name}");
        assert!(schema["$schema"].is_string(), "$schema of {name}");
    }
}

#[test]
fn export_schemas_is_byte_identical_across_runs() {
    let a = scratch_dir("a");
    let b = scratch_dir("b");
    let pa = qp_schema::export_schemas(&a).unwrap();
    let pb = qp_schema::export_schemas(&b).unwrap();
    assert_eq!(pa.len(), qp_schema::json_schemas().len());
    assert_eq!(pa.len(), pb.len());
    for (fa, fb) in pa.iter().zip(&pb) {
        assert_eq!(fa.file_name(), fb.file_name());
        let ta = std::fs::read(fa).unwrap();
        let tb = std::fs::read(fb).unwrap();
        assert_eq!(ta, tb, "{} differs", fa.display());
        assert!(
            ta.ends_with(b"}\n"),
            "{} lacks trailing newline",
            fa.display()
        );
        assert!(fa.to_string_lossy().ends_with(".schema.json"));
    }
    // Re-export into an existing directory overwrites with the same bytes.
    let again = qp_schema::export_schemas(&a).unwrap();
    assert_eq!(pa, again);
    for (f, g) in again.iter().zip(&pb) {
        assert_eq!(std::fs::read(f).unwrap(), std::fs::read(g).unwrap());
    }
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
