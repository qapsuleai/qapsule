//! File helpers of the host: paths confined to a root, JSON files.

use std::path::{Component, Path, PathBuf};

/// Resolves `rel` under `root`, refusing absolute paths and paths that leave `root`.
///
/// # Errors
/// The path escapes the root or does not exist.
pub fn confine(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute()
        || p.components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "path `{rel}` must be relative and stay inside the root"
        ));
    }
    let root_c = root
        .canonicalize()
        .map_err(|e| format!("root {}: {e}", root.display()))?;
    let full = root_c
        .join(p)
        .canonicalize()
        .map_err(|e| format!("{rel}: {e}"))?;
    if !full.starts_with(&root_c) {
        return Err(format!("path `{rel}` leaves the root"));
    }
    Ok(full)
}

/// Reads a JSON file into `T`.
///
/// # Errors
/// I/O or JSON errors.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}
