//! Golden comparison for new R3 goldens (same rule as `tests/golden.rs`): a golden is
//! written only when it does not exist yet and `QP_UPDATE_GOLDEN=1`; an existing golden is
//! never updated. On mismatch the actual image goes to `target/golden-actual/`.

#![allow(dead_code)]

use std::path::PathBuf;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compares `actual` with the file at `rel` (repository-relative).
pub fn check_file(rel: &str, actual: &[u8]) {
    let path = root().join(rel);
    if !path.exists() && std::env::var_os("QP_UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::write(&path, actual).unwrap();
        eprintln!("wrote new golden {}", path.display());
        return;
    }
    let golden = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); a new golden can be written with QP_UPDATE_GOLDEN=1",
            path.display()
        )
    });
    if actual != golden.as_slice() {
        let dir = root().join("target/golden-actual");
        std::fs::create_dir_all(&dir).unwrap();
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let p = dir.join(name);
        std::fs::write(&p, actual).unwrap();
        panic!(
            "{rel} differs from the actual output (written to {}); existing goldens are not updated (CLAUDE.md)",
            p.display()
        );
    }
}
