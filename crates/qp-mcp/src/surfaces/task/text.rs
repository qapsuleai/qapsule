//! Results and capsule lines of the task surface: argument parsing, text results,
//! protocol errors, paging and the one-line capsule texts.

use super::*;

pub(super) fn parse<T: for<'de> Deserialize<'de>>(tool: &str, args: Value) -> Result<T, RpcError> {
    serde_json::from_value(args).map_err(|e| RpcError::new(-32602, format!("tool `{tool}`: {e}")))
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

pub(super) fn ok_text(text: String, structured: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

pub(super) fn fail_text(text: String, structured: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": true,
    })
}

pub(super) fn is_result(r: &Value) -> bool {
    r.get("type").and_then(Value::as_str) == Some("result")
}

/// A protocol `error` reply as a tool error: code, path, message, hint, `did_you_mean`
/// and (for a rejected comment) every reason, as text and as structured content.
pub(super) fn protocol_error(tool: &str, reply: Value) -> Value {
    let e = reply.get("error").unwrap_or(&reply);
    let mut t = format!("{tool}: error `{}`", s(e, "code"));
    for k in ["reason", "path", "message"] {
        if let Some(v) = e.get(k).and_then(Value::as_str) {
            t.push_str(&format!(" · {k}: {v}"));
        }
    }
    if let Some(d) = e.get("did_you_mean").filter(|d| !d.is_null()) {
        t.push_str(&format!(" · did_you_mean: {d}"));
    }
    if let Some(x) = e.get("expected").filter(|d| !d.is_null()) {
        t.push_str(&format!(" · expected: {x}"));
    }
    if let Some(rs) = e.get("reasons").and_then(Value::as_array) {
        for r in rs {
            t.push_str(&format!("\n- {}/{}", s(r, "check"), s(r, "code")));
            if let Some(tok) = r.get("token").and_then(Value::as_str) {
                t.push_str(&format!(" `{tok}`"));
            }
            if let Some(n) = r.get("sentence") {
                t.push_str(&format!(" (sentence {n})"));
            }
            t.push_str(&format!(": {}", s(r, "hint")));
        }
    }
    if let Some(h) = e.get("hint").and_then(Value::as_str) {
        t.push_str(&format!("\nhint: {h}"));
    }
    fail_text(t, reply)
}

/// A string field (empty if absent).
pub(super) fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// A field's JSON text as the protocol gave it (numbers unchanged), `-` if absent.
pub(super) fn n(v: &Value, k: &str) -> String {
    match v.get(k) {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::String(x)) => x.clone(),
        Some(x) => x.to_string(),
    }
}

pub(super) fn limit_of(l: Option<u32>) -> usize {
    l.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize
}

/// The page of `items` after the id `after` (`Err`: not in the list).
pub(super) fn page<'a>(
    items: &'a [Value],
    after: Option<&str>,
    limit: usize,
) -> Result<(&'a [Value], Option<String>), String> {
    let start = match after {
        None => 0,
        Some(a) => {
            items
                .iter()
                .position(|c| s(c, "id") == a)
                .ok_or_else(|| format!("`after` `{a}` is not in this list"))?
                + 1
        }
    };
    let rest = &items[start..];
    let take = rest.len().min(limit);
    let next = (rest.len() > take).then(|| s(&rest[take - 1], "id").to_owned());
    Ok((&rest[..take], next))
}

pub(super) fn merge(into: &mut Value, extra: Value) {
    if let (Some(o), Value::Object(e)) = (into.as_object_mut(), extra) {
        for (k, v) in e {
            if !v.is_null() {
                o.insert(k, v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Capsule lines
// ---------------------------------------------------------------------------

pub(super) fn sentence(item: &Value, lang: Option<Language>) -> String {
    let sen = item.get("sentence").unwrap_or(&Value::Null);
    let k = match lang {
        None => "text",
        Some(Language::Tr) => "tr",
        Some(Language::En) => "en",
    };
    s(sen, k).to_owned()
}

/// One capsule (`CapsuleItem`): the facts line (ids and numbers).
pub(super) fn item_line(c: &Value) -> String {
    let lab = c.get("ledger_at_birth").unwrap_or(&Value::Null);
    let mut t = format!(
        "{} · {} {} {} · {} ({}) · tier {} · confirmations {} · born bar {} · last bar {} · t {} · record at birth {} of {} realized · finding {}",
        s(c, "id"),
        s(c, "instrument"),
        s(c, "kind"),
        s(c, "direction"),
        s(c, "stage"),
        s(c, "validity"),
        n(c, "tier"),
        n(c, "confirmations"),
        n(c, "born_bar"),
        n(c, "last_bar"),
        n(c, "t"),
        n(lab, "realized"),
        n(lab, "n"),
        s(c, "finding"),
    );
    for (flag, word) in [
        ("relevant", "holdings/watch"),
        ("redacted", "redacted (no data license)"),
        ("coin_flip", "coin flip"),
    ] {
        if c.get(flag).and_then(Value::as_bool) == Some(true) {
            t.push_str(&format!(" · {word}"));
        }
    }
    if s(c, "trust") == "server" {
        t.push_str(" · server-verified");
    }
    t
}

/// A capsule into `out`: card = facts line + sentence; layered summary = id + stage +
/// sentence, detail = facts line.
pub(super) fn push_item(out: &mut Out, c: &Value, lang: Option<Language>, prefix: &str) {
    let sen = sentence(c, lang);
    let facts = format!("{prefix}{}", item_line(c));
    out.summary
        .push(format!("{prefix}{}: {} — {sen}", s(c, "id"), s(c, "stage")));
    out.card.push(facts.clone());
    out.card.push(format!("  {sen}"));
    out.detail.push(facts);
}

pub(super) fn server_comments(out: &mut Out, comments: Option<&Value>, lang: Option<Language>) {
    let Some(cs) = comments.and_then(Value::as_array).filter(|c| !c.is_empty()) else {
        return;
    };
    out.both("server comments (licensed data you are not entitled to; no figures):".to_owned());
    let k = if lang == Some(Language::En) {
        "en"
    } else {
        "tr"
    };
    for c in cs {
        out.both(format!(
            "- {} bar {} t {}: {}",
            s(c, "instrument"),
            n(c, "bar"),
            n(c, "t"),
            s(c, k)
        ));
    }
}

pub(super) fn is_closed(c: &Value) -> bool {
    matches!(
        s(c, "stage"),
        "realized" | "invalidated" | "expired" | "retracted"
    )
}
