//! The `glossary` tool: the closed vocabulary from the schema's own descriptions.

use super::*;

// ---------------------------------------------------------------------------
// glossary
// ---------------------------------------------------------------------------

/// The variants of a unit enum with their descriptions, from its generated schema.
pub(super) fn variants<T: schemars::JsonSchema>() -> Vec<(String, String)> {
    let v = serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null);
    let mut out = Vec::new();
    for alt in v
        .get("oneOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let desc = s(alt, "description").replace('\n', " ");
        if let Some(c) = alt.get("const").and_then(Value::as_str) {
            out.push((c.to_owned(), desc));
        } else if let Some(es) = alt.get("enum").and_then(Value::as_array) {
            for e in es.iter().filter_map(Value::as_str) {
                out.push((e.to_owned(), desc.clone()));
            }
        }
    }
    if out.is_empty() {
        for e in v
            .get("enum")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            out.push((e.to_owned(), String::new()));
        }
    }
    out
}

pub(super) fn glossary(a: &GlossaryArgs) -> Value {
    let groups: [(&str, Vec<(String, String)>); 7] = [
        ("meaning", variants::<MeaningKind>()),
        ("direction", variants::<Direction>()),
        ("stage", variants::<Stage>()),
        ("validity", variants::<Validity>()),
        ("time", variants::<TimeKind>()),
        ("trust", variants::<Trust>()),
        ("evidence family", variants::<EvidenceFamily>()),
    ];
    let term = a.term.as_deref().map(str::to_lowercase);
    let mut lines = Vec::new();
    let mut entries = Vec::new();
    for (g, vs) in &groups {
        for (name, desc) in vs {
            let hit = term.as_ref().is_none_or(|t| {
                name.to_lowercase().contains(t.as_str())
                    || desc.to_lowercase().contains(t.as_str())
                    || g.contains(t.as_str())
            });
            if hit {
                lines.push(format!("{g} `{name}`: {desc}"));
                entries.push(json!({"group": g, "name": name, "text": desc}));
            }
        }
    }
    if lines.is_empty() {
        lines.push("No entry matches; call without `term` for the whole list.".to_owned());
    }
    ok_text(lines.join("\n"), json!({"entries": entries}))
}
