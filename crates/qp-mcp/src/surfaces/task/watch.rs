//! The `watch` tool and the watch part of `whats_new` (watches whose state or match count
//! changed since the last look), with their lines.

use super::*;

// ---------------------------------------------------------------------------
// whats_new
// ---------------------------------------------------------------------------

impl Task {
    /// Lines for watches whose state or match count changed since the last look.
    pub(super) fn watch_changes(&mut self, ctx: &mut Ctx<'_>) -> Vec<String> {
        let r = ctx.query(json!({"kind": "list", "collection": "intents"}));
        let mut lines = Vec::new();
        let Some(items) = r.pointer("/data/items").and_then(Value::as_array) else {
            return lines;
        };
        for w in items.iter().filter(|w| w.get("watch").is_some()) {
            let id = s(w, "id").to_owned();
            let now = (n(w, "state"), n(w, "matches"));
            if self.seen_watches.get(&id) != Some(&now) {
                lines.push(format!(
                    "- {id} · state {} · matches {} · \"{}\"",
                    now.0,
                    now.1,
                    s(w, "text")
                ));
                self.seen_watches.insert(id, now);
            }
        }
        lines
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn watch(&mut self, ctx: &mut Ctx<'_>, a: WatchArgs) -> Result<Value, RpcError> {
        match a.action {
            WatchAction::List => {
                let r = ctx.query(json!({"kind": "list", "collection": "intents"}));
                if !is_result(&r) {
                    return Ok(protocol_error("watch", r));
                }
                let items: Vec<Value> = r
                    .pointer("/data/items")
                    .and_then(Value::as_array)
                    .map(|v| {
                        v.iter()
                            .filter(|w| w.get("watch").is_some())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let mut lines = vec!["your watches:".to_owned()];
                for w in &items {
                    lines.push(watch_line(w));
                }
                if items.is_empty() {
                    lines.push("(none)".to_owned());
                }
                let ids: Vec<&str> = items.iter().map(|w| s(w, "id")).collect();
                Ok(ok_text(
                    lines.join("\n"),
                    json!({"ids": ids, "watches": items}),
                ))
            }
            WatchAction::Remove => {
                let Some(intent) = a.intent else {
                    return Err(RpcError::new(
                        -32602,
                        "tool `watch`: `remove` needs `intent` (a watch id from `watch` list)",
                    ));
                };
                let r = ctx.command(vec![json!({"op": "remove_watch", "intent": intent})]);
                if r.get("type").and_then(Value::as_str) != Some("ack") {
                    return Ok(protocol_error("watch", r));
                }
                Ok(ok_text(
                    format!("watch {intent} removed (seq {})", n(&r, "seq")),
                    r,
                ))
            }
            WatchAction::Add => {
                let (Some(text), Some(m)) = (a.text, a.match_) else {
                    return Err(RpcError::new(
                        -32602,
                        "tool `watch`: `add` needs `text` and `match`, e.g. \
                         {\"text\":\"…\",\"match\":{\"instrument\":\"THYAO\",\"negative\":true}}",
                    ));
                };
                let mut op = json!({"op": "add_watch", "text": text, "match": m});
                if let Some(t) = a.expires_t {
                    op["expires_t"] = json!(t);
                }
                if let Some(x) = a.max_matches {
                    op["max_matches"] = json!(x);
                }
                let r = ctx.command(vec![op]);
                if r.get("type").and_then(Value::as_str) != Some("ack") {
                    return Ok(protocol_error("watch", r));
                }
                // Matches come as `intent_matched` notifications: subscribe once.
                if !self.subscribed {
                    let id = ctx.request_id();
                    let s = ctx.send(
                        &json!({"type": "subscribe", "id": id, "topic": {"kind": "intents"}}),
                    );
                    self.subscribed = s.get("type").and_then(Value::as_str) == Some("ack");
                }
                let id = r
                    .pointer("/created/0")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                Ok(ok_text(
                    format!(
                        "watch {id} added (seq {}); matches arrive as `intent_matched` \
                         notifications (`notifications` tool) and in `whats_new`",
                        n(&r, "seq")
                    ),
                    r,
                ))
            }
        }
    }
}

pub(super) fn by_birth(c: &Value) -> &'static str {
    if is_closed(c) {
        "closed"
    } else if c["born_bar"] == c["last_bar"] {
        "born"
    } else {
        "changed"
    }
}

pub(super) fn watch_line(w: &Value) -> String {
    let watch = w.get("watch").unwrap_or(&Value::Null);
    format!(
        "- {} · state {} · matches {} · for {} · match {} · expires_t {} · max_matches {} · \"{}\"",
        s(w, "id"),
        n(w, "state"),
        n(w, "matches"),
        s(watch, "for"),
        watch.get("match").unwrap_or(&Value::Null),
        n(watch, "expires_t"),
        n(watch, "max_matches"),
        s(w, "text"),
    )
}
