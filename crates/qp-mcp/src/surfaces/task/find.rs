//! The `find` tool: capsules by instrument, meaning, direction, stage, open/closed.

use super::*;

// ---------------------------------------------------------------------------
// find
// ---------------------------------------------------------------------------

pub(super) fn find(ctx: &mut Ctx<'_>, a: &FindArgs) -> Value {
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &a.instrument {
        q["instrument"] = json!(i);
    }
    if let Some(m) = a.meaning {
        q["meaning"] = json!(m);
    }
    if !a.stages.is_empty() {
        q["stages"] = json!(a.stages);
    }
    if let Some(o) = a.open {
        q["open"] = json!(o);
    }
    if let Some(t) = a.since_t {
        q["since_t"] = json!(t);
    }
    if let Some(t) = a.born_since_t {
        q["born_since_t"] = json!(t);
    }
    if a.feed {
        q["audience"] = json!(true);
    }
    // `latest` with a direction: the latest of that direction, so it is applied here.
    if a.latest && a.direction.is_none() {
        q["latest"] = json!(true);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("find", r);
    }
    let mut caps: Vec<Value> = r
        .pointer("/data/capsules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(d) = a.direction {
        let d = json!(d);
        caps.retain(|c| c.get("direction") == Some(&d));
        if a.latest {
            caps = latest_of(ctx, &caps);
        }
    }
    let (rows, next) = match page(&caps, a.after.as_deref(), limit_of(a.limit)) {
        Ok(p) => p,
        Err(e) => return fail_text(format!("find: {e}"), json!({"error": e})),
    };
    let mut out = Out::new();
    out.both(format!(
        "find · {} match(es){}",
        caps.len(),
        next.as_ref()
            .map_or(String::new(), |x| format!(" · next: {x}"))
    ));
    for c in rows {
        push_item(&mut out, c, a.lang, "- ");
    }
    if caps.is_empty() {
        out.both("No capsule matches.".to_owned());
    }
    server_comments(&mut out, r.pointer("/data/comments"), a.lang);
    let ids: Vec<String> = rows.iter().map(|c| s(c, "id").to_owned()).collect();
    let mut data = r.get("data").cloned().unwrap_or(Value::Null);
    data["capsules"] = json!(rows);
    data["total"] = json!(caps.len());
    if let Some(x) = &next {
        data["next"] = json!(x);
    }
    out.render(a.format, a.detail, ids, json!({"next": next}), data)
}

/// Of `caps`, the latest born by the protocol's `latest` rule (§20.2: greatest birth
/// time, then birth bar, then id). Birth times come from the authority's `stories` rows
/// of the candidates' instruments (only compared, never computed).
pub(super) fn latest_of(ctx: &mut Ctx<'_>, caps: &[Value]) -> Vec<Value> {
    let mut born_t: BTreeMap<String, i64> = BTreeMap::new();
    let mut instruments: Vec<&str> = caps.iter().map(|c| s(c, "instrument")).collect();
    instruments.sort_unstable();
    instruments.dedup();
    for inst in instruments {
        let mut after: Option<String> = None;
        loop {
            let mut q = json!({"kind": "stories", "instrument": inst, "limit": 500});
            if let Some(a) = &after {
                q["after"] = json!(a);
            }
            let r = ctx.query(q);
            for row in r
                .pointer("/data/stories")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(t) = row["born_t"].as_i64() {
                    born_t.insert(s(row, "id").to_owned(), t);
                }
            }
            match r.pointer("/data/next").and_then(Value::as_str) {
                Some(n) => after = Some(n.to_owned()),
                None => break,
            }
        }
    }
    caps.iter()
        .max_by(|a, b| {
            let key = |c: &Value| {
                (
                    born_t.get(s(c, "id")).copied().unwrap_or(i64::MIN),
                    c["born_bar"].as_u64().unwrap_or(0),
                    s(c, "id").to_owned(),
                )
            };
            key(a).cmp(&key(b))
        })
        .cloned()
        .into_iter()
        .collect()
}
