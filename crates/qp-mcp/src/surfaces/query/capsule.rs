//! Capsule answer items of the `query` surface: summary, card, expanded parts, comments.

use super::*;

// ---------------------------------------------------------------------------------------
// Items.

/// One answer item in its three shapes.
#[derive(Default)]
pub(super) struct Item {
    pub(super) id: String,
    /// Other ids it carries (finding, facts, cited ids).
    pub(super) refs: Vec<String>,
    /// `json`: the protocol object.
    pub(super) raw: Value,
    /// `layered`: the compact summary.
    pub(super) summary: Value,
    /// `card`: the text card (first line, then indented lines).
    pub(super) card: Vec<String>,
    /// Expanded parts: name → (raw, summary, card lines).
    pub(super) parts: Vec<(&'static str, Value, Value, Vec<String>)>,
}

/// An answer: items plus extra fields.
#[derive(Default)]
pub(super) struct Out {
    pub(super) items: Vec<Item>,
    pub(super) total: usize,
    /// Extra top-level fields: name → (raw, compact, card lines).
    pub(super) extra: Vec<(&'static str, Value, Value, Vec<String>)>,
}

pub(super) fn capsule_summary(item: &Value, facts: &[String]) -> Value {
    let mut v = pick(
        item,
        &[
            "id",
            "finding",
            "instrument",
            "kind",
            "direction",
            "stage",
            "validity",
            "tier",
            "born_bar",
            "last_bar",
            "t",
            "ledger_at_birth",
            "relevant",
            "redacted",
            "coin_flip",
        ],
    );
    v["sentence"] = item
        .pointer("/sentence/text")
        .cloned()
        .unwrap_or(Value::Null);
    v["facts"] = json!(facts);
    v
}

pub(super) fn capsule_card(item: &Value, facts: &[String]) -> Vec<String> {
    let lab = item.get("ledger_at_birth").cloned().unwrap_or(Value::Null);
    let mut flags = Vec::new();
    for f in ["relevant", "redacted", "coin_flip"] {
        if item.get(f) == Some(&Value::Bool(true)) {
            flags.push(f);
        }
    }
    let flags = if flags.is_empty() {
        String::new()
    } else {
        format!(" · {}", flags.join(", "))
    };
    vec![
        format!(
            "[{}] {} {} {} · stage {} · {} · tier {} · born bar {}, last bar {} · ledger at birth {} of {} realized{}",
            s(item, "id"),
            s(item, "instrument"),
            s(item, "kind"),
            s(item, "direction"),
            s(item, "stage"),
            s(item, "validity"),
            t(item, "tier"),
            t(item, "born_bar"),
            t(item, "last_bar"),
            t(&lab, "realized"),
            t(&lab, "n"),
            flags,
        ),
        format!("  {}", item.pointer("/sentence/text").and_then(Value::as_str).unwrap_or("")),
        format!("  finding {} · facts {}", s(item, "finding"), facts.join(", ")),
    ]
}

/// The expanded parts of a capsule detail.
pub(super) fn detail_parts(
    detail: &Value,
    expand: &[Expand],
) -> Vec<(&'static str, Value, Value, Vec<String>)> {
    let mut out = Vec::new();
    for e in expand {
        match e {
            Expand::Story => {
                let raw = detail.get("story").cloned().unwrap_or(Value::Null);
                let hist: Vec<Value> = arr(&raw, "history")
                    .iter()
                    .map(|h| pick(h, &["bar", "t", "stage", "reason", "facts"]))
                    .collect();
                let line = hist
                    .iter()
                    .map(|h| format!("bar {} {} ({})", t(h, "bar"), s(h, "stage"), s(h, "reason")))
                    .collect::<Vec<_>>()
                    .join(" → ");
                let mut sum = json!({"history": hist});
                for k in ["condition", "resolution"] {
                    if let Some(x) = raw.get(k) {
                        sum[k] = x.clone();
                    }
                }
                out.push(("story", raw, sum, vec![format!("  story: {line}")]));
            }
            Expand::Evidence => {
                let raw = detail.get("evidence").cloned().unwrap_or(json!([]));
                let mut sum = Vec::new();
                let mut lines = Vec::new();
                for ev in raw.as_array().map_or(&[][..], Vec::as_slice) {
                    let mut x = pick(
                        ev,
                        &[
                            "fact", "family", "license", "open", "values", "requires", "rests_on",
                        ],
                    );
                    if let Some(d) = ev.pointer("/evidence_ref/digest") {
                        x["evidence_ref"] = d.clone();
                    }
                    let what = if ev.get("open") == Some(&Value::Bool(true)) {
                        let vals = ev
                            .get("values")
                            .and_then(Value::as_object)
                            .map(|m| {
                                m.iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        format!("open: {vals}")
                    } else {
                        format!("closed, requires {}", t(ev, "requires"))
                    };
                    lines.push(format!(
                        "  evidence {} ({}, {}) {}; rests on {}{}",
                        s(ev, "fact"),
                        s(ev, "family"),
                        s(ev, "license"),
                        what,
                        s(ev, "rests_on"),
                        ev.pointer("/evidence_ref/digest")
                            .and_then(Value::as_str)
                            .map(|d| format!("; ref {d}"))
                            .unwrap_or_default(),
                    ));
                    sum.push(x);
                }
                out.push(("evidence", raw, Value::Array(sum), lines));
            }
            Expand::Ledger => {
                let raw = detail.get("ledger").cloned().unwrap_or(Value::Null);
                let line = format!("  ledger: {raw}");
                out.push(("ledger", raw.clone(), raw, vec![line]));
            }
            Expand::Comments => {
                let raw = detail.get("comments").cloned().unwrap_or(json!([]));
                let sum: Vec<Value> = raw
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(comment_summary)
                    .collect();
                let lines = sum
                    .iter()
                    .map(|c| {
                        format!(
                            "  comment [{}] {} (cites {})",
                            s(c, "id"),
                            s(c, "text"),
                            t(c, "cites")
                        )
                    })
                    .collect();
                out.push(("comments", raw, Value::Array(sum), lines));
            }
        }
    }
    out
}

pub(super) fn capsule_item(item: &Value, detail: &Value, expand: &[Expand]) -> Item {
    let facts: Vec<String> = arr(detail, "facts")
        .iter()
        .filter_map(|f| f.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let mut refs = vec![s(item, "finding").to_owned()];
    refs.extend(facts.iter().cloned());
    Item {
        id: s(item, "id").to_owned(),
        refs,
        raw: item.clone(),
        summary: capsule_summary(item, &facts),
        card: capsule_card(item, &facts),
        parts: detail_parts(detail, expand),
    }
}

pub(super) fn comment_summary(c: &Value) -> Value {
    let mut v = pick(
        c,
        &[
            "id",
            "about",
            "lang",
            "text",
            "cites",
            "external",
            "on_behalf",
        ],
    );
    if let Some(a) = c.pointer("/provenance/author/name") {
        v["author"] = a.clone();
    }
    v
}

/// The server comments (unentitled reader) of a `capsules`/`stories` answer.
pub(super) fn server_comments(data: &Value) -> Option<(&'static str, Value, Value, Vec<String>)> {
    let raw = data.get("comments")?.clone();
    let list = raw.as_array()?;
    if list.is_empty() {
        return None;
    }
    let sum: Vec<Value> = list
        .iter()
        .map(|c| {
            let mut x = pick(
                c,
                &["instrument", "series", "bar", "t", "codes", "tr", "en"],
            );
            if let Some(e) = c.pointer("/attachments/entitlement") {
                x["requires"] = e.clone();
            }
            x
        })
        .collect();
    let lines = sum
        .iter()
        .map(|c| {
            format!(
                "server comment {} bar {}: {} (raw figures require {})",
                s(c, "instrument"),
                t(c, "bar"),
                s(c, "tr"),
                t(c, "requires")
            )
        })
        .collect();
    Some(("server_comments", raw, Value::Array(sum), lines))
}
