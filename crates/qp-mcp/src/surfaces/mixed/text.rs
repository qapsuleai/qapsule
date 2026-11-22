//! Results and capsule texts of the mixed surface: tool results, protocol errors, the
//! capsule card and layered forms (also used by the thin surface).

use super::*;

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

pub(super) fn text_result(text: String) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "isError": false,
    })
}

pub(super) fn json_result(v: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": v.to_string()}],
        "structuredContent": v,
        "isError": false,
    })
}

/// A text result with the protocol reply as structured content.
pub(super) fn reply_result(text: String, reply: Value, is_error: bool) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": reply,
        "isError": is_error,
    })
}

/// A tool error before any message was sent (the agent's arguments do not fit).
pub(super) fn usage_error(text: String) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "isError": true,
    })
}

pub(super) fn is_error(reply: &Value) -> bool {
    reply.get("type").and_then(Value::as_str) == Some("error")
}

/// A protocol error as text an agent can act on: code, message, path, reason, hint,
/// suggestions, candidates and (comment refusals) every reason with its hint.
pub(super) fn error_text(tool: &str, reply: &Value) -> String {
    let e = reply.get("error").unwrap_or(reply);
    let mut t = format!(
        "{tool}: {} — {}",
        s(e, "code"),
        e.get("message").and_then(Value::as_str).unwrap_or("")
    );
    for k in ["path", "reason", "hint"] {
        if let Some(v) = e.get(k).and_then(Value::as_str) {
            let _ = write!(t, "\n{k}: {v}");
        }
    }
    if let Some(v) = e.get("did_you_mean") {
        let _ = write!(t, "\ndid_you_mean: {v}");
    }
    if let Some(v) = e.get("expected") {
        let _ = write!(t, "\nexpected: {v}");
    }
    if let Some(c) = e.get("candidates").and_then(Value::as_array) {
        t.push_str("\ncandidates:");
        for x in c {
            let _ = write!(
                t,
                "\n- {}",
                x.get("id").map_or_else(|| x.to_string(), plain)
            );
            if let Some(a) = x.get("about").and_then(Value::as_str) {
                let _ = write!(t, ": {a}");
            }
        }
    }
    if let Some(rs) = e.get("reasons").and_then(Value::as_array) {
        t.push_str("\nreasons (fix each and send again):");
        for r in rs {
            let _ = write!(t, "\n- [{}] {}", s(r, "check"), s(r, "code"));
            if let Some(tok) = r.get("token").and_then(Value::as_str) {
                let _ = write!(t, " token `{tok}`");
            }
            if let Some(n) = r.get("sentence") {
                let _ = write!(t, " sentence {n}");
            }
            if let Some(h) = r.get("hint").and_then(Value::as_str) {
                let _ = write!(t, ": {h}");
            }
        }
    }
    t
}

pub(super) fn protocol_error(tool: &str, reply: Value) -> Value {
    reply_result(error_text(tool, &reply), reply, true)
}

// ---------------------------------------------------------------------------
// Text helpers (numbers exactly as the protocol wrote them)
// ---------------------------------------------------------------------------

/// A JSON value as plain text: strings without quotes, numbers as their JSON text.
pub(super) fn plain(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => "-".to_owned(),
        other => other.to_string(),
    }
}

pub(super) fn s(v: &Value, key: &str) -> String {
    v.get(key).map_or_else(|| "-".to_owned(), plain)
}

pub(super) fn ledger_text(l: &Value) -> String {
    format!("{} of {} realized", s(l, "realized"), s(l, "n"))
}

/// One line for a capsule item.
pub(crate) fn item_line(c: &Value) -> String {
    let mut flags = String::new();
    if c.get("relevant").and_then(Value::as_bool) == Some(true) {
        flags.push_str(", RELEVANT to the person");
    }
    if c.get("redacted").and_then(Value::as_bool) == Some(true) {
        flags.push_str(", redacted (no licence for the numbers)");
    }
    format!(
        "- {} [{} {} {}] stage {}, {}, tier {}, importance {}{flags}\n  {}\n  finding {}; born bar {}, last bar {} (t {}); ledger at birth {}",
        s(c, "id"),
        s(c, "instrument"),
        s(c, "kind"),
        s(c, "direction"),
        s(c, "stage"),
        s(c, "validity"),
        s(c, "tier"),
        s(c, "importance"),
        c.pointer("/sentence/text").map_or_else(String::new, plain),
        s(c, "finding"),
        s(c, "born_bar"),
        s(c, "last_bar"),
        s(c, "t"),
        c.get("ledger_at_birth").map_or_else(String::new, ledger_text),
    )
}

/// The summary of a capsule item (`layered`).
pub(crate) fn item_summary(c: &Value) -> Value {
    let mut o = Map::new();
    for k in [
        "id",
        "finding",
        "instrument",
        "kind",
        "direction",
        "stage",
        "validity",
        "tier",
        "importance",
        "born_bar",
        "last_bar",
        "t",
        "ledger_at_birth",
        "relevant",
        "redacted",
    ] {
        if let Some(v) = c.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    if let Some(t) = c.pointer("/sentence/text") {
        o.insert("sentence".into(), t.clone());
    }
    Value::Object(o)
}

pub(super) fn comment_line(c: &Value) -> String {
    let by = c
        .pointer("/provenance/author/name")
        .map_or_else(|| "-".to_owned(), plain);
    let mut t = format!("- {} by {by}", s(c, "id"));
    if let Some(p) = c.get("on_behalf").and_then(Value::as_str) {
        let _ = write!(t, " for {p}");
    }
    let _ = write!(
        t,
        " (cites {}): {}",
        c.get("cites").map_or_else(String::new, Value::to_string),
        s(c, "text")
    );
    t
}

pub(super) fn evidence_line(e: &Value) -> String {
    let mut t = format!(
        "- {} ({}, {})",
        s(e, "fact"),
        s(e, "family"),
        s(e, "license")
    );
    if e.get("open").and_then(Value::as_bool) == Some(true) {
        let vals: Vec<String> = e
            .get("values")
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| format!("{k}={}", plain(v))).collect())
            .unwrap_or_default();
        let _ = write!(t, ": {}", vals.join(", "));
    } else {
        let _ = write!(t, ": values need `{}`", s(e, "requires"));
    }
    let _ = write!(t, "; rests on {}", s(e, "rests_on"));
    t
}

#[allow(clippy::too_many_lines)]
pub(crate) fn capsule_card(d: &Value) -> String {
    let i = d.get("item").cloned().unwrap_or(Value::Null);
    let mut t = format!(
        "Capsule {} (finding {})\n{} {} {} · stage {} · state {} · validity {} · tier {} · importance {} · confirmations {} · trust {}",
        s(&i, "id"),
        s(&i, "finding"),
        s(&i, "instrument"),
        s(&i, "kind"),
        s(&i, "direction"),
        s(&i, "stage"),
        s(&i, "state"),
        s(&i, "validity"),
        s(&i, "tier"),
        s(&i, "importance"),
        s(&i, "confirmations"),
        s(&i, "trust"),
    );
    if i.get("redacted").and_then(Value::as_bool) == Some(true) {
        t.push_str(" · redacted (no licence for the numbers)");
    }
    let _ = write!(
        t,
        "\nSentence: {}",
        i.pointer("/sentence/text").map_or_else(String::new, plain)
    );
    let hist = d
        .pointer("/story/history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let _ = write!(t, "\nStory ({} transitions):", hist.len());
    for h in &hist {
        let _ = write!(
            t,
            "\n- bar {} (t {}) {}: {} [facts {}]",
            s(h, "bar"),
            s(h, "t"),
            s(h, "stage"),
            s(h, "reason"),
            h.get("facts").map_or_else(String::new, Value::to_string)
        );
    }
    let _ = write!(
        t,
        "\nLedger now: {}; at birth: {}",
        d.get("ledger").map_or_else(String::new, ledger_text),
        i.get("ledger_at_birth")
            .map_or_else(String::new, ledger_text)
    );
    t.push_str("\nEvidence:");
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let _ = write!(t, "\n{}", evidence_line(e));
    }
    let comments = d
        .get("comments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let _ = write!(t, "\nComments ({}):", comments.len());
    for c in &comments {
        let _ = write!(t, "\n{}", comment_line(c));
    }
    if let Some(w) = d.get("withheld") {
        let _ = write!(
            t,
            "\nWithheld: newer licensed activity needs {}",
            w.get("requires").map_or_else(String::new, Value::to_string)
        );
    }
    let facts: Vec<String> = d
        .get("facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|f| s(f, "id"))
        .collect();
    let _ = write!(
        t,
        "\nIds: capsule {}, finding {}, facts {}",
        s(&i, "id"),
        s(&i, "finding"),
        facts.join(", ")
    );
    t
}

#[allow(clippy::expect_used)]
pub(crate) fn capsule_layered(d: &Value, detail: bool) -> Value {
    let i = d.get("item").cloned().unwrap_or(Value::Null);
    let mut sum = item_summary(&i);
    let o = sum.as_object_mut().expect("object");
    for k in ["state", "confirmations", "trust"] {
        if let Some(v) = i.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    if let Some(l) = d.get("ledger") {
        o.insert("ledger".into(), l.clone());
    }
    let facts: Vec<Value> = d
        .get("facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|f| f.get("id").cloned())
        .collect();
    o.insert("facts".into(), Value::Array(facts));
    let ev = d.get("evidence").and_then(Value::as_array);
    o.insert(
        "evidence_open".into(),
        json!(ev.map_or(0, |e| e.iter().filter(|x| x["open"] == true).count())),
    );
    o.insert(
        "comments".into(),
        json!(d
            .get("comments")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)),
    );
    let mut out = json!({"kind": "capsule", "summary": sum});
    if detail {
        out["story"] = d.pointer("/story/history").cloned().unwrap_or(json!([]));
        out["evidence"] = d.get("evidence").cloned().unwrap_or(json!([]));
        out["comments"] = d.get("comments").cloned().unwrap_or(json!([]));
        if let Some(w) = d.get("withheld") {
            out["withheld"] = w.clone();
        }
    } else {
        out["more"] = json!(
            "call again with detail:true for the story, the evidence values and the comments"
        );
    }
    out
}

/// Capsule items laid out in `format`.
pub(super) fn items_out(items: &[Value], format: Format, detail: bool) -> (String, Value) {
    let mut text = String::new();
    for c in items {
        let _ = write!(text, "\n{}", item_line(c));
    }
    let v = match format {
        Format::Json => Value::Array(items.to_vec()),
        Format::Layered if detail => Value::Array(items.to_vec()),
        _ => Value::Array(items.iter().map(item_summary).collect()),
    };
    (text, v)
}

pub(super) fn limit_of(l: Option<u32>) -> u32 {
    l.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

pub(super) fn parse<T: for<'de> Deserialize<'de>>(tool: &str, args: Value) -> Result<T, RpcError> {
    serde_json::from_value(args).map_err(|e| RpcError::new(-32602, format!("tool `{tool}`: {e}")))
}

pub(super) fn data(reply: &Value) -> Value {
    reply.get("data").cloned().unwrap_or(Value::Null)
}
