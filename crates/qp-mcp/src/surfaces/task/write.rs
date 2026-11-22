//! The writing tools: `comment` (a validated comment) and `propose` (a view or drawing
//! proposal with its preview).

use super::*;

// ---------------------------------------------------------------------------
// comment
// ---------------------------------------------------------------------------

pub(super) fn comment(ctx: &mut Ctx<'_>, a: CommentArgs) -> Value {
    let about = match (&a.capsule, &a.instrument) {
        (Some(CapsuleRef::Id(id)), None) => json!({"capsule": id}),
        (Some(sel @ CapsuleRef::Select(_)), None) => {
            // Resolve the selector as the reader sees it (the authority's rule).
            let r = ctx.query(json!({"kind": "capsule", "capsule": sel}));
            if !is_result(&r) {
                return protocol_error("comment", r);
            }
            let id = r
                .pointer("/data/detail/item/id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            json!({"capsule": id})
        }
        (None, Some(i)) => json!({"instrument": i}),
        _ => {
            return fail_text(
                "comment: give exactly one of `capsule` and `instrument`".to_owned(),
                json!({"error": {"code": "invalid", "path": "/capsule"}}),
            )
        }
    };
    let mut op = json!({"op": "add_comment", "about": about, "lang": a.lang,
        "text": a.text, "cites": a.cites});
    if !a.external.is_empty() {
        op["external"] = json!(a.external);
    }
    let r = ctx.command(vec![op]);
    if r.get("type").and_then(Value::as_str) != Some("ack") {
        return protocol_error("comment", r);
    }
    let id = r
        .pointer("/created/0")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_owned();
    ok_text(
        format!(
            "comment {id} accepted and stored (seq {}), about {about}",
            n(&r, "seq")
        ),
        r,
    )
}

// ---------------------------------------------------------------------------
// propose
// ---------------------------------------------------------------------------

pub(super) fn propose(ctx: &mut Ctx<'_>, a: ProposeArgs) -> Value {
    if a.ops.is_empty() {
        return fail_text(
            "propose: `ops` is empty".to_owned(),
            json!({"error": {"code": "invalid", "path": "/ops"}}),
        );
    }
    let id = ctx.request_id();
    let mut cmd = json!({"type": "command", "id": id, "mode": "propose", "ops": a.ops});
    if let Some(r) = a.reason {
        cmd["reason"] = json!(r);
    }
    let r = ctx.send(&cmd);
    if r.get("type").and_then(Value::as_str) != Some("ack") {
        return protocol_error("propose", r);
    }
    let Some(pid) = r.get("proposal").and_then(Value::as_str).map(str::to_owned) else {
        return ok_text(
            format!(
                "propose: applied without a proposal (status {})",
                n(&r, "status")
            ),
            r,
        );
    };
    let mut lines = vec![format!(
        "proposal {pid} is {} (seq {}); a person accepts or rejects it — you cannot",
        n(&r, "status"),
        n(&r, "seq")
    )];
    let p = ctx.query(json!({"kind": "preview", "proposal": pid}));
    let preview = if is_result(&p) {
        let d = p.get("data").cloned().unwrap_or(Value::Null);
        let ops: Vec<&str> = d
            .get("changes")
            .and_then(Value::as_array)
            .map(|c| c.iter().map(|x| s(x, "op")).collect())
            .unwrap_or_default();
        lines.push(format!("preview: changes [{}]", ops.join(", ")));
        for v in d
            .get("views")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lines.push(format!(
                "- view {}{}: {}",
                s(v, "view"),
                if v["new"].as_bool() == Some(true) {
                    " (new)"
                } else {
                    ""
                },
                s(v, "explain")
            ));
            lines.push(format!(
                "  see it: render {{\"view\":\"{}\",\"proposal\":\"{pid}\"}}",
                s(v, "view")
            ));
        }
        d
    } else {
        lines.push(format!(
            "preview unavailable: {}",
            p.get("error").unwrap_or(&p)
        ));
        Value::Null
    };
    ok_text(lines.join("\n"), json!({"ack": r, "preview": preview}))
}
