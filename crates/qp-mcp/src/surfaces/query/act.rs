//! The `act` tool: comment, watch, unwatch, propose, preview (no decisions).

use super::*;

/// A capsule reference resolved to its capsule id (selectors and finding ids too).
pub(super) fn capsule_id(io: &mut Io<'_, '_>, r: &CapsuleRef) -> Result<String, Fail> {
    let d = io.capsule_detail(r)?;
    Ok(d.pointer("/item/id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned())
}

#[allow(clippy::too_many_lines)]
pub(super) fn act(ctx: &mut Ctx<'_>, args: &Value) -> Result<Value, Fail> {
    let Some(o) = args.as_object() else {
        return Err(Fail::schema(
            "",
            "act arguments must be an object".to_owned(),
            &["do"],
            vec![],
        ));
    };
    let Some(d) = o.get("do").and_then(Value::as_str) else {
        return Err(Fail::schema(
            "/do",
            "`do` is required".to_owned(),
            &ACT_DOS,
            vec![],
        ));
    };
    if HUMAN_ONLY.iter().any(|h| h.starts_with(d) || *h == d)
        || matches!(d, "accept" | "reject" | "approve" | "decide")
    {
        return Err(Fail(
            json!({"error": {"code": "forbidden", "reason": "human_only", "path": "/do",
            "message": format!("`{d}` is a person's decision; agents cannot decide proposals or findings"),
            "hint": "Propose instead (do: propose); the person accepts or rejects it on their own connection."}}),
        ));
    }
    if !ACT_DOS.contains(&d) {
        return Err(Fail::schema(
            "/do",
            format!("unknown action `{d}`"),
            &ACT_DOS,
            did_you_mean(d, ACT_DOS),
        ));
    }
    let dv: Do = serde_json::from_value(json!(d)).map_err(|e| serde_fail(&e, "do"))?;
    check_keys(args, "", act_keys(dv), &format!("act (do {d})"))?;
    let a: ActArgs = serde_json::from_value(args.clone()).map_err(|e| serde_fail(&e, "act"))?;
    let mut io = Io { ctx, seq: None };
    let reply = match a.do_ {
        Do::Comment => {
            let about = match (&a.capsule, &a.instrument) {
                (Some(r), None) => json!({"capsule": capsule_id(&mut io, r)?}),
                (None, Some(i)) => json!({"instrument": io.instrument(i, "/instrument")?}),
                _ => {
                    return Err(Fail::schema(
                        "/capsule",
                        "a comment is about exactly one of `capsule` or `instrument`".to_owned(),
                        &["capsule", "instrument"],
                        vec![],
                    ))
                }
            };
            let Some(text) = &a.text else {
                return Err(Fail::schema(
                    "/text",
                    "`text` is required".to_owned(),
                    &["text"],
                    vec![],
                ));
            };
            let mut cites = Vec::new();
            for c in &a.cites {
                cites.push(match c {
                    CapsuleRef::Id(x) => x.clone(),
                    CapsuleRef::Select(_) => capsule_id(&mut io, c)?,
                });
            }
            let mut op = json!({"op": "add_comment", "about": about,
                "lang": ser(&a.lang.unwrap_or_default()), "text": text, "cites": cites});
            if !a.external.is_empty() {
                op["external"] = ser(&a.external);
            }
            io.command(vec![op], None, None)?
        }
        Do::Watch => {
            let Some(text) = &a.text else {
                return Err(Fail::schema(
                    "/text",
                    "`text` (the request in words) is required".to_owned(),
                    &["text", "match"],
                    vec![],
                ));
            };
            let mut op = json!({"op": "add_watch", "text": text,
                "match": ser(&a.match_.clone().unwrap_or_default())});
            if let Some(f) = &a.for_ {
                op["for"] = json!(f);
            }
            if let Some(x) = a.expires_t {
                op["expires_t"] = json!(x);
            }
            if let Some(x) = a.max_matches {
                op["max_matches"] = json!(x);
            }
            io.command(vec![op], None, None)?
        }
        Do::Unwatch => {
            let Some(wid) = &a.watch else {
                return Err(Fail::schema(
                    "/watch",
                    "`watch` (the watch id) is required".to_owned(),
                    &["watch"],
                    vec![],
                ));
            };
            io.command(
                vec![json!({"op": "remove_watch", "intent": wid})],
                None,
                None,
            )?
        }
        Do::Propose => {
            if a.ops.is_empty() {
                return Err(Fail::schema(
                    "/ops",
                    "`ops` needs at least one operation".to_owned(),
                    &["ops"],
                    vec![],
                ));
            }
            for (i, op) in a.ops.iter().enumerate() {
                let name = op.get("op").and_then(Value::as_str);
                match name {
                    None => {
                        return Err(Fail::schema(
                            &format!("/ops/{i}/op"),
                            "each operation is an object with `op`".to_owned(),
                            &["op"],
                            vec![],
                        ));
                    }
                    Some(n) if HUMAN_ONLY.contains(&n) => {
                        return Err(Fail(
                            json!({"error": {"code": "forbidden", "reason": "human_only",
                            "path": format!("/ops/{i}"),
                            "message": format!("`{n}` is a person's decision; it cannot be proposed by an agent"),
                            "hint": "A person decides proposals on their own connection."}}),
                        ));
                    }
                    Some(_) => {}
                }
            }
            io.command(a.ops.clone(), Some("propose"), a.reason.as_deref())?
        }
        Do::Preview => {
            let Some(p) = &a.proposal else {
                return Err(Fail::schema(
                    "/proposal",
                    "`proposal` (id or selector) is required".to_owned(),
                    &["proposal"],
                    vec![],
                ));
            };
            let pid = io.proposal_id(p)?;
            let mut q = json!({"kind": "preview", "proposal": pid});
            if let Some(x) = a.width {
                q["width"] = json!(x);
            }
            if let Some(x) = a.height {
                q["height"] = json!(x);
            }
            let data = io.query(q)?;
            let ops: Vec<Value> = arr(&data, "changes")
                .iter()
                .filter_map(|c| c.get("op").cloned())
                .collect();
            let v = json!({"done": "preview", "proposal": data.get("proposal").cloned().unwrap_or(Value::Null),
                "changes": ops, "views": data.get("views").cloned().unwrap_or(json!([])),
                "seq": io.seq, "note": "Nothing was written. `render` with `proposal` draws it."});
            return Ok(ok_result(v));
        }
    };
    let mut v = json!({"done": d});
    if let Some(m) = reply.as_object() {
        for (k, x) in m {
            if k != "type" && k != "id" {
                v[k] = x.clone();
            }
        }
    }
    if a.do_ == Do::Propose {
        v["note"] = json!("Pending until a person accepts or rejects it; `act` do preview shows what it would change.");
    }
    Ok(ok_result(v))
}

pub(super) fn ok_result(v: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": v.to_string()}],
        "structuredContent": v,
        "isError": false,
    })
}
