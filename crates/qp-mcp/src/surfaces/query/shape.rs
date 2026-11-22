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

pub(super) fn shape(
    find: &str,
    format: Format,
    select: &[String],
    out: Out,
    seq: Option<u64>,
) -> Value {
    let count = out.items.len();
    let mut ids: Vec<String> = Vec::new();
    for it in &out.items {
        for x in std::iter::once(&it.id).chain(it.refs.iter()) {
            if !x.is_empty() && !ids.contains(x) {
                ids.push(x.clone());
            }
        }
    }
    let mut env =
        json!({"find": find, "format": format_name(format), "count": count, "total": out.total});
    if let Some(s) = seq {
        env["seq"] = json!(s);
    }
    let truncated = out.total > count;
    let more = truncated.then(|| {
        format!(
            "{} more; raise `limit` (≤ 100) or narrow `where`",
            out.total - count
        )
    });
    let text = match format {
        Format::Card => {
            env["ids"] = json!(ids);
            let mut lines = vec![format!(
                "{find}: {count} of {}{}",
                out.total,
                seq.map(|s| format!(" · seq {s}")).unwrap_or_default()
            )];
            for it in &out.items {
                if select.is_empty() {
                    lines.extend(it.card.iter().cloned());
                } else {
                    let v = select_fields(&it.summary, select);
                    let kv = v
                        .as_object()
                        .map(|o| {
                            o.iter()
                                .filter(|(k, _)| k.as_str() != "id")
                                .map(|(k, x)| {
                                    format!(
                                        "{k}={}",
                                        x.as_str().map_or_else(|| x.to_string(), str::to_owned)
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(" · ")
                        })
                        .unwrap_or_default();
                    lines.push(format!("[{}] {kv}", it.id));
                }
                for (_, _, _, l) in &it.parts {
                    lines.extend(l.iter().cloned());
                }
            }
            for (_, _, _, l) in &out.extra {
                lines.extend(l.iter().cloned());
            }
            if let Some(m) = &more {
                lines.push(m.clone());
            }
            lines.join("\n")
        }
        Format::Json | Format::Layered => {
            let raw = format == Format::Json;
            let items: Vec<Value> = out
                .items
                .iter()
                .map(|it| {
                    let mut v = select_fields(if raw { &it.raw } else { &it.summary }, select);
                    if raw && !it.refs.is_empty() {
                        if let Some(o) = v.as_object_mut() {
                            o.entry("refs").or_insert_with(|| json!(it.refs));
                        }
                    }
                    for (name, r, sm, _) in &it.parts {
                        v[*name] = if raw { r.clone() } else { sm.clone() };
                    }
                    v
                })
                .collect();
            env["items"] = json!(items);
            for (name, r, sm, _) in &out.extra {
                env[*name] = if raw { r.clone() } else { sm.clone() };
            }
            if format == Format::Layered
                && find == "capsules"
                && out.items.iter().all(|i| i.parts.is_empty())
            {
                env["more"] = json!(
                    "expand: [\"story\",\"evidence\",\"ledger\",\"comments\"] adds those layers"
                );
            }
            if let Some(m) = &more {
                env["truncated"] = json!(m);
            }
            env["ids"] = json!(ids);
            env.to_string()
        }
    };

    let structured = if format == Format::Card {
        if let Some(m) = &more {
            env["truncated"] = json!(m);
        }
        env
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    };
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

pub(super) fn format_name(f: Format) -> &'static str {
    match f {
        Format::Card => "card",
        Format::Json => "json",
        Format::Layered => "layered",
    }
}
