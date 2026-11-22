//! Input schema shaping of the mixed surface: the `write` tool's schema narrowed to the
//! proposal ops, with unused `$defs` pruned.

use super::*;

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

pub(super) fn collect_refs(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if let Some(name) = o
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
            {
                out.insert(name.to_owned());
            }
            o.values().for_each(|x| collect_refs(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| collect_refs(x, out)),
        _ => {}
    }
}

/// Drops the `$defs` that the schema (outside `$defs`) no longer reaches.
pub(super) fn prune_defs(schema: &mut Value) {
    let Some(defs) = schema.get("$defs").and_then(Value::as_object).cloned() else {
        return;
    };
    let mut body = schema.clone();
    if let Some(o) = body.as_object_mut() {
        o.remove("$defs");
    }
    let mut seen = BTreeSet::new();
    let mut todo = BTreeSet::new();
    collect_refs(&body, &mut todo);
    while let Some(n) = todo.pop_first() {
        if seen.insert(n.clone()) {
            if let Some(d) = defs.get(&n) {
                let mut more = BTreeSet::new();
                collect_refs(d, &mut more);
                todo.extend(more.into_iter().filter(|m| !seen.contains(m)));
            }
        }
    }
    let kept: Map<String, Value> = defs.into_iter().filter(|(k, _)| seen.contains(k)).collect();
    if let Some(o) = schema.as_object_mut() {
        if kept.is_empty() {
            o.remove("$defs");
        } else {
            o.insert("$defs".into(), Value::Object(kept));
        }
    }
}

/// The `write` schema: `Op` narrowed to [`PROPOSAL_OPS`].
pub(super) fn write_schema() -> Value {
    let mut s = input_schema::<WriteArgs>();
    if let Some(variants) = s
        .pointer_mut("/$defs/Op/oneOf")
        .and_then(Value::as_array_mut)
    {
        variants.retain(|v| {
            v.pointer("/properties/op/const")
                .and_then(Value::as_str)
                .is_some_and(|n| PROPOSAL_OPS.contains(&n))
        });
    }
    prune_defs(&mut s);
    s
}
