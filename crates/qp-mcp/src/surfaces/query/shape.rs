//! Shaping an answer: field access, `select`, and the three formats (`card`, `json`,
//! `layered`).

use super::*;

pub(super) fn arr<'v>(v: &'v Value, key: &str) -> &'v [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

pub(super) fn s<'v>(v: &'v Value, key: &str) -> &'v str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// `v[key]` as compact JSON text (numbers exactly as the authority wrote them).
pub(super) fn t(v: &Value, key: &str) -> String {
    match v.get(key) {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::String(x)) => x.clone(),
        Some(x) => x.to_string(),
    }
}

/// The fields `keys` of `v` that are present (and not false/null).
pub(super) fn pick(v: &Value, keys: &[&str]) -> Value {
    let mut m = Map::new();
    for k in keys {
        if let Some(x) = v.get(*k) {
            if !x.is_null() && x != &Value::Bool(false) {
                m.insert((*k).to_owned(), x.clone());
            }
        }
    }
    Value::Object(m)
}

// ---------------------------------------------------------------------------------------
// Shaping.

pub(super) fn select_fields(v: &Value, select: &[String]) -> Value {
    if select.is_empty() {
        return v.clone();
    }
    let mut m = Map::new();
    for k in std::iter::once("id").chain(select.iter().map(String::as_str)) {
        if let Some(x) = v.get(k) {
            m.insert(k.to_owned(), x.clone());
        }
    }
    Value::Object(m)
}

pub(super) fn format_name(f: Format) -> &'static str {
    match f {
        Format::Card => "card",
        Format::Json => "json",
        Format::Layered => "layered",
    }
}
