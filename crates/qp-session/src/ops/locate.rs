//! Finds *where* a message violates the schema.
//!
//! serde reports *what* is wrong, but every protocol message is an internally tagged
//! enum, which serde buffers before deserializing the variant; `serde_path_to_error`
//! therefore loses the path at the first tagged enum (verified: an unknown field inside
//! an op reports path `.`). This module walks the message alongside the JSON Schema
//! generated from the same Rust types (`schemars`), picking `oneOf` branches by their
//! tag constant, and reports the first violation with a JSON pointer.

use serde_json::{Map, Value};

use crate::error::ptr;

/// A schema violation found by the walker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Violation {
    pub path: String,
    pub kind: ViolationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ViolationKind {
    /// A key the object does not accept.
    UnknownField {
        field: String,
        expected: Vec<String>,
    },
    /// A tag or enum value that is not one of the accepted names.
    UnknownValue {
        field: Option<String>,
        value: String,
        expected: Vec<String>,
    },
    /// A required key is missing.
    MissingField { field: String },
    /// Wrong JSON type (or out-of-range integer, wrong array length).
    WrongType { expected: String, got: String },
}

impl Violation {
    pub(crate) fn message(&self) -> String {
        match &self.kind {
            ViolationKind::UnknownField { field, .. } => format!("unknown field `{field}`"),
            ViolationKind::UnknownValue { field, value, .. } => match field {
                Some(f) => format!("unknown {f} `{value}`"),
                None => format!("unknown value `{value}`"),
            },
            ViolationKind::MissingField { field } => format!("missing field `{field}`"),
            ViolationKind::WrongType { expected, got } => {
                format!("invalid type: expected {expected}, got {got}")
            }
        }
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_matches(t: &str, v: &Value) -> bool {
    match t {
        "null" => v.is_null(),
        "boolean" => v.is_boolean(),
        "integer" => v.as_i64().is_some() || v.as_u64().is_some(),
        "number" => v.is_number(),
        "string" => v.is_string(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        _ => true,
    }
}

/// Walks `value` against `schema` (whose `$defs` live in `root`).
pub(crate) fn locate(value: &Value, schema: &Value, root: &Value) -> Option<Violation> {
    walk(value, schema, root, "", 0)
}

fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let mut s = schema;
    for _ in 0..32 {
        match s.get("$ref").and_then(Value::as_str) {
            Some(r) => {
                let name = r.rsplit('/').next().unwrap_or("");
                match root.get("$defs").and_then(|d| d.get(name)) {
                    Some(t) => s = t,
                    None => return s,
                }
            }
            None => return s,
        }
    }
    s
}

fn consts(branches: &[Value], root: &Value) -> Option<Vec<String>> {
    branches
        .iter()
        .map(|b| {
            resolve(b, root)
                .get("const")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

/// The common tag key of object branches (each branch has `properties.<key>.const`).
fn tag_key(branches: &[Value], root: &Value) -> Option<String> {
    let first = resolve(branches.first()?, root);
    let props = first.get("properties")?.as_object()?;
    for (k, p) in props {
        if p.get("const").is_some()
            && branches.iter().all(|b| {
                resolve(b, root)
                    .get("properties")
                    .and_then(|ps| ps.get(k))
                    .and_then(|p| p.get("const"))
                    .is_some()
            })
        {
            return Some(k.clone());
        }
    }
    None
}

fn wrong(path: &str, expected: impl Into<String>, v: &Value) -> Option<Violation> {
    Some(Violation {
        path: path.to_owned(),
        kind: ViolationKind::WrongType {
            expected: expected.into(),
            got: type_name(v).to_owned(),
        },
    })
}

#[allow(clippy::too_many_lines)]
fn walk(v: &Value, schema: &Value, root: &Value, path: &str, depth: u32) -> Option<Violation> {
    if depth > 200 {
        return None;
    }
    let s = resolve(schema, root);
    let branches = s
        .get("oneOf")
        .or_else(|| s.get("anyOf"))
        .and_then(Value::as_array);
    if let Some(branches) = branches {
        return walk_branches(v, branches, root, path, depth);
    }
    if let Some(c) = s.get("const") {
        if c != v {
            return Some(Violation {
                path: path.to_owned(),
                kind: ViolationKind::UnknownValue {
                    field: None,
                    value: v.as_str().map_or_else(|| v.to_string(), str::to_owned),
                    expected: vec![c.as_str().map_or_else(|| c.to_string(), str::to_owned)],
                },
            });
        }
        return None;
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        if !e.contains(v) {
            return Some(Violation {
                path: path.to_owned(),
                kind: ViolationKind::UnknownValue {
                    field: None,
                    value: v.as_str().map_or_else(|| v.to_string(), str::to_owned),
                    expected: e
                        .iter()
                        .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_owned))
                        .collect(),
                },
            });
        }
        return None;
    }
    // Type check.
    if let Some(t) = s.get("type") {
        let types: Vec<&str> = match t {
            Value::String(x) => vec![x.as_str()],
            Value::Array(xs) => xs.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !types.is_empty() && !types.iter().any(|t| type_matches(t, v)) {
            let exp: Vec<&str> = types.iter().copied().filter(|t| *t != "null").collect();
            return wrong(path, exp.join(" or "), v);
        }
    }
    if v.is_null() {
        return None;
    }
    if let Some(n) = v.as_number() {
        let fmt = s.get("format").and_then(Value::as_str).unwrap_or("");
        let unsigned = s.get("minimum").and_then(Value::as_f64) == Some(0.0);
        let ok = match fmt {
            "uint32" => n.as_u64().is_some_and(|x| u32::try_from(x).is_ok()),
            "uint64" | "uint" => n.as_u64().is_some(),
            "int64" => n.as_i64().is_some(),
            "uint8" => n.as_u64().is_some_and(|x| u8::try_from(x).is_ok()),
            _ => !unsigned || n.as_f64().is_some_and(|x| x >= 0.0),
        };
        if !ok {
            let what = match fmt {
                "uint32" => "unsigned 32-bit integer",
                "uint64" | "uint" => "unsigned integer",
                "int64" => "64-bit integer",
                _ => "non-negative number",
            };
            return wrong(path, what, v);
        }
        return None;
    }
    if let Some(arr) = v.as_array() {
        let min = s.get("minItems").and_then(Value::as_u64);
        let max = s.get("maxItems").and_then(Value::as_u64);
        let len = arr.len() as u64;
        if min.is_some_and(|m| len < m) || max.is_some_and(|m| len > m) {
            let exp = match (min, max) {
                (Some(a), Some(b)) if a == b => format!("array of {a} items"),
                _ => "array of a different length".to_owned(),
            };
            return wrong(path, exp, v);
        }
        if let Some(items) = s.get("items") {
            for (i, x) in arr.iter().enumerate() {
                if let Some(viol) = walk(x, items, root, &ptr(path, i), depth + 1) {
                    return Some(viol);
                }
            }
        }
        return None;
    }
    if let Some(obj) = v.as_object() {
        return walk_object(obj, s, root, path, depth);
    }
    None
}

fn walk_object(
    obj: &Map<String, Value>,
    s: &Value,
    root: &Value,
    path: &str,
    depth: u32,
) -> Option<Violation> {
    let props = s.get("properties").and_then(Value::as_object);
    let additional = s.get("additionalProperties");
    if let Some(props) = props {
        if additional == Some(&Value::Bool(false)) {
            for k in obj.keys() {
                if !props.contains_key(k) {
                    let expected = props
                        .iter()
                        .filter(|(_, p)| p.get("const").is_none())
                        .map(|(n, _)| n.clone())
                        .collect();
                    return Some(Violation {
                        path: ptr(path, k),
                        kind: ViolationKind::UnknownField {
                            field: k.clone(),
                            expected,
                        },
                    });
                }
            }
        }
    }
    if let Some(req) = s.get("required").and_then(Value::as_array) {
        for r in req.iter().filter_map(Value::as_str) {
            if !obj.contains_key(r) {
                return Some(Violation {
                    path: path.to_owned(),
                    kind: ViolationKind::MissingField {
                        field: r.to_owned(),
                    },
                });
            }
        }
    }
    for (k, x) in obj {
        let extra = match additional {
            Some(Value::Object(_)) => additional,
            _ => None,
        };
        let sub = props.and_then(|p| p.get(k)).or(extra);
        if let Some(sub) = sub {
            if let Some(viol) = walk(x, sub, root, &ptr(path, k), depth + 1) {
                return Some(viol);
            }
        }
    }
    None
}

#[allow(clippy::too_many_lines)]
fn walk_branches(
    v: &Value,
    branches: &[Value],
    root: &Value,
    path: &str,
    depth: u32,
) -> Option<Violation> {
    // Option<T>: `anyOf [T, {type: null}]`.
    let non_null: Vec<&Value> = branches
        .iter()
        .filter(|b| resolve(b, root).get("type").and_then(Value::as_str) != Some("null"))
        .collect();
    if non_null.len() < branches.len() {
        if v.is_null() {
            return None;
        }
        if non_null.len() == 1 {
            return walk(v, non_null[0], root, path, depth + 1);
        }
    }
    // String enum: every branch is a constant.
    if let Some(names) = consts(branches, root) {
        let ok = v.as_str().is_some_and(|x| names.iter().any(|n| n == x));
        if ok {
            return None;
        }
        if !v.is_string() {
            return wrong(path, "string", v);
        }
        return Some(Violation {
            path: path.to_owned(),
            kind: ViolationKind::UnknownValue {
                field: None,
                value: v.as_str().unwrap_or_default().to_owned(),
                expected: names,
            },
        });
    }
    // Internally tagged enum: pick the branch by its tag constant.
    if let Some(tag) = tag_key(branches, root) {
        let Some(obj) = v.as_object() else {
            return wrong(path, "object", v);
        };
        let names: Vec<String> = branches
            .iter()
            .filter_map(|b| {
                resolve(b, root)
                    .get("properties")
                    .and_then(|p| p.get(&tag))
                    .and_then(|p| p.get("const"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        let Some(tv) = obj.get(&tag) else {
            return Some(Violation {
                path: path.to_owned(),
                kind: ViolationKind::MissingField { field: tag },
            });
        };
        let Some(tname) = tv.as_str() else {
            return wrong(&ptr(path, &tag), "string", tv);
        };
        let Some(i) = names.iter().position(|n| n == tname) else {
            return Some(Violation {
                path: ptr(path, &tag),
                kind: ViolationKind::UnknownValue {
                    field: Some(tag),
                    value: tname.to_owned(),
                    expected: names,
                },
            });
        };
        return walk(v, &branches[i], root, path, depth + 1);
    }
    // Untagged: valid if any branch accepts it; otherwise report the error of the first
    // branch that accepts the value's JSON type (an object given where "id or selector"
    // is expected gets the selector's error, e.g. its unknown key), else the first error.
    let mut first = None;
    let mut typed = None;
    for b in branches {
        match walk(v, b, root, path, depth + 1) {
            None => return None,
            Some(viol) => {
                if typed.is_none() && accepts_type(b, v, root, 0) {
                    typed = Some(viol.clone());
                }
                if first.is_none() {
                    first = Some(viol);
                }
            }
        }
    }
    typed.or(first)
}

/// Whether a schema branch can accept a value of `v`'s JSON type at all.
fn accepts_type(schema: &Value, v: &Value, root: &Value, depth: u32) -> bool {
    let s = resolve(schema, root);
    if let Some(t) = s.get("type") {
        return match t {
            Value::String(x) => type_matches(x, v),
            Value::Array(xs) => xs
                .iter()
                .filter_map(Value::as_str)
                .any(|x| type_matches(x, v)),
            _ => true,
        };
    }
    if let Some(bs) = s
        .get("oneOf")
        .or_else(|| s.get("anyOf"))
        .and_then(Value::as_array)
    {
        return depth > 4 || bs.iter().any(|b| accepts_type(b, v, root, depth + 1));
    }
    if s.get("properties").is_some() {
        return v.is_object();
    }
    if let Some(c) = s.get("const") {
        return type_name(c) == type_name(v);
    }
    true
}
