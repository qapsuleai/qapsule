//! The `explain` tool: one capsule with its story, evidence, track record and comments.

use super::*;

// ---------------------------------------------------------------------------
// explain
// ---------------------------------------------------------------------------

pub(super) fn explain(ctx: &mut Ctx<'_>, a: &ExplainArgs) -> Value {
    let r = ctx.query(json!({"kind": "capsule", "capsule": a.capsule}));
    if !is_result(&r) {
        return protocol_error("explain", r);
    }
    let d = r.pointer("/data/detail").cloned().unwrap_or(Value::Null);
    let item = d.get("item").unwrap_or(&Value::Null);
    let mut out = Out::new();
    out.both(item_line(item));
    out.both(format!("sentence: {}", sentence(item, a.lang)));
    let story = d.get("story").unwrap_or(&Value::Null);
    let ledger = d.get("ledger").unwrap_or(&Value::Null);
    out.both(format!(
        "track record of {} now: {} of {} realized",
        s(story, "ledger_key"),
        n(ledger, "realized"),
        n(ledger, "n")
    ));
    // Story.
    out.deep(format!(
        "story: theme {} · {} · horizon {} bars · resolution {} · condition {}",
        s(story, "theme"),
        s(story, "time"),
        n(story, "horizon_bars"),
        story
            .pointer("/resolution/rule")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        story
            .get("condition")
            .map_or("-".to_owned(), Value::to_string),
    ));
    for h in story
        .get("history")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let facts: Vec<&str> = h
            .get("facts")
            .and_then(Value::as_array)
            .map(|f| f.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        out.deep(format!(
            "- bar {} t {} {} ({}){}",
            n(h, "bar"),
            n(h, "t"),
            s(h, "stage"),
            s(h, "reason"),
            if facts.is_empty() {
                String::new()
            } else {
                format!(" facts {}", facts.join(", "))
            }
        ));
    }
    // Evidence.
    out.deep("evidence:".to_owned());
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let head = format!(
            "- {} ({}, {})",
            s(e, "fact"),
            s(e, "family"),
            s(e, "license")
        );
        if e.get("open").and_then(Value::as_bool) == Some(true) {
            let vals: Vec<String> = e
                .get("values")
                .and_then(Value::as_object)
                .map(|m| m.iter().map(|(k, v)| format!("{k}={v}")).collect())
                .unwrap_or_default();
            let mut line = format!("{head}: {}", vals.join(" "));
            if let Some(er) = e.get("evidence_ref") {
                line.push_str(&format!(
                    " · frozen {} {} over {} bars, t {} to {}",
                    s(er, "alg"),
                    s(er, "digest"),
                    n(er, "bars"),
                    n(er, "from_t"),
                    n(er, "to_t")
                ));
            }
            line.push_str(&format!(" · rests on {}", s(e, "rests_on")));
            out.deep(line);
        } else {
            out.deep(format!(
                "{head}: values not shown to you — requires data license `{}` · rests on {}",
                n(e, "requires"),
                s(e, "rests_on")
            ));
        }
    }
    // Comments.
    if let Some(cs) = d
        .get("comments")
        .and_then(Value::as_array)
        .filter(|c| !c.is_empty())
    {
        out.deep("comments:".to_owned());
        for c in cs {
            let author = c
                .pointer("/provenance/author/name")
                .and_then(Value::as_str)
                .unwrap_or("-");
            let kind = c
                .pointer("/provenance/author/kind")
                .and_then(Value::as_str)
                .unwrap_or("-");
            let behalf = c
                .get("on_behalf")
                .and_then(Value::as_str)
                .map_or(String::new(), |b| format!(" for {b}"));
            out.deep(format!(
                "- {} ({kind} {author}{behalf}; cites {}): {}",
                s(c, "id"),
                c.get("cites").unwrap_or(&Value::Null),
                s(c, "text")
            ));
        }
    }
    if let Some(w) = d.get("withheld") {
        out.deep(format!(
            "withheld: newer licensed moves you are not entitled to; requires {}",
            w.get("requires").unwrap_or(&Value::Null)
        ));
        server_comments(&mut out, w.get("comments"), a.lang);
    }
    let mut ids = vec![s(item, "id").to_owned(), s(item, "finding").to_owned()];
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        ids.push(s(e, "fact").to_owned());
    }
    out.render(
        a.format,
        a.detail,
        ids,
        json!({"resolved": r.get("resolved")}),
        r.get("data").cloned().unwrap_or(Value::Null),
    )
}
