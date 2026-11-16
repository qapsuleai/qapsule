//! File helpers (copied from the R3 harness): the M11 root, deterministic JSON, sorted
//! directory walks and extraction of a reader's JSON answer.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Default root: `rounds/R4/experiments/M11/` (this crate is
/// `rounds/R4/experiments/harness/m11/`).
#[must_use]
pub fn default_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../M11")
}

/// Writes `bytes` to `path`, creating parent directories.
///
/// # Errors
/// I/O failure (as a message).
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Pretty JSON with a trailing newline (struct field order and `BTreeMap` order are fixed,
/// so equal values give equal bytes).
///
/// # Errors
/// Serialization failure.
pub fn json<T: Serialize>(v: &T) -> Result<Vec<u8>, String> {
    let mut s = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    s.push(b'\n');
    Ok(s)
}

/// Writes `v` as pretty JSON to `path`.
///
/// # Errors
/// I/O or serialization failure.
pub fn write_json<T: Serialize>(path: &Path, v: &T) -> Result<(), String> {
    write(path, &json(v)?)
}

/// Reads a file to a string.
///
/// # Errors
/// I/O failure.
pub fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Rounds to `d` decimals (half away from zero); used before writing numbers so the raw
/// records do not depend on the last bits of a transcendental function.
#[must_use]
pub fn round(x: f64, d: i32) -> f64 {
    let p = 10f64.powi(d);
    let r = (x * p).round() / p;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// Style names become directory names: `[a-z][a-z0-9_-]*`.
///
/// # Errors
/// Any other name.
pub fn check_name(name: &str) -> Result<(), String> {
    let b = name.as_bytes();
    let ok = !b.is_empty()
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!("style name `{name}` must match [a-z][a-z0-9_-]*"))
    }
}

/// Sub-directories of `dir`, sorted by name (empty if `dir` does not exist).
#[must_use]
pub fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// File name of a path as a string (empty if none).
#[must_use]
pub fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The reader's answer text from a raw `claude -p --output-format json` record: the
/// `result` field if the record is such an object, else the whole text.
#[must_use]
pub fn answer_text(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(s) = v.get("result").and_then(serde_json::Value::as_str) {
            return s.to_owned();
        }
    }
    raw.to_owned()
}

/// The first balanced top-level JSON object inside `text` (code fences and prose around it
/// are ignored). `None` if there is no parsable object.
#[must_use]
pub fn first_json_object(text: &str) -> Option<serde_json::Value> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(off) = text[start..].find('{') {
        let s = start + off;
        let (mut depth, mut in_str, mut esc) = (0i32, false, false);
        for (i, &b) in bytes.iter().enumerate().skip(s) {
            if in_str {
                if esc {
                    esc = false;
                } else if b == b'\\' {
                    esc = true;
                } else if b == b'"' {
                    in_str = false;
                }
                continue;
            }
            match b {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Ok(v) = serde_json::from_str(&text[s..=i]) {
                            return Some(v);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
        start = s + 1;
    }
    None
}

/// Lower-cased string field of a JSON object (`""` if absent).
#[must_use]
pub fn field(v: &serde_json::Value, k: &str) -> String {
    match v.get(k) {
        Some(serde_json::Value::String(s)) => s.trim().to_lowercase(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_prose() {
        let t = "Here:\n```json\n{\"A1\": {\"kind\": \"flow\", \"x\": \"}\"}}\n```\nDone {x";
        let v = first_json_object(t).unwrap();
        assert_eq!(field(&v["A1"], "kind"), "flow");
        assert!(first_json_object("no json").is_none());
        let raw = r#"{"type":"result","result":"{\"a\": 1}"}"#;
        assert_eq!(answer_text(raw), "{\"a\": 1}");
    }

    #[test]
    fn names_and_rounding() {
        assert!(check_name("r3").is_ok());
        assert!(check_name("3r").is_err());
        assert!(check_name("A b").is_err());
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(-0.00001, 2), 0.0);
    }
}
