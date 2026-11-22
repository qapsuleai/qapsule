//! The `instrument` and `track_record` tools: an instrument's stories and the session
//! clock; the ledger.

use super::*;

// ---------------------------------------------------------------------------
// instrument
// ---------------------------------------------------------------------------

pub(super) fn instrument(ctx: &mut Ctx<'_>, a: &InstrumentArgs) -> Value {
    let Some(sym) = &a.symbol else {
        let r = ctx.query(json!({"kind": "list", "collection": "series"}));
        if !is_result(&r) {
            return protocol_error("instrument", r);
        }
        let items = r
            .pointer("/data/items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut lines = vec!["instruments (symbol: bar series, interval):".to_owned()];
        let mut syms = Vec::new();
        for it in &items {
            if let Some(sy) = it.pointer("/instrument/symbol").and_then(Value::as_str) {
                lines.push(format!("- {sy}: {} {}", s(it, "id"), s(it, "interval")));
                syms.push(sy.to_owned());
            }
        }
        return ok_text(lines.join("\n"), json!({"instruments": syms}));
    };
    let mut q = json!({"kind": "stories", "instrument": sym, "limit": limit_of(a.limit)});
    if let Some(l) = a.last {
        q["range"] = json!({"last": l});
    }
    if let Some(x) = &a.after {
        q["after"] = json!(x);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("instrument", r);
    }
    let d = r.get("data").cloned().unwrap_or(Value::Null);
    let mut out = Out::new();
    out.both(format!(
        "{} (series {}) · stories {} in range{} · now {}",
        s(&d, "instrument"),
        s(&d, "series"),
        n(&d, "total"),
        d.get("next")
            .and_then(Value::as_str)
            .map_or(String::new(), |x| format!(" · next: {x}")),
        n(&r, "now"),
    ));
    // The session clock (replay position), when the session is a replay.
    let id = ctx.request_id();
    let st = ctx.send(&json!({"type": "replay", "id": id, "control": {"kind": "status"}}));
    let clock = st.pointer("/data/replay").cloned();
    if let Some(c) = &clock {
        out.both(format!(
            "clock: replay t {} · step {} of {} · {}{}",
            n(c, "t"),
            n(c, "step"),
            n(c, "steps"),
            if c["paused"].as_bool() == Some(true) {
                "paused"
            } else {
                "running"
            },
            if c["done"].as_bool() == Some(true) {
                " · done"
            } else {
                ""
            }
        ));
    }
    let rows = d
        .get("stories")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in &rows {
        let f = row.get("face").unwrap_or(&Value::Null);
        out.both(format!(
            "- {}: {} {} — {}, {}",
            s(row, "id"),
            s(f, "event"),
            s(f, "when"),
            s(f, "validity_phrase"),
            s(f, "ledger_short")
        ));
        let mut line = format!(
            "- {} · {} {} · {} · validity {} · tier {} · born bar {} t {} · last bar {} t {}",
            s(row, "id"),
            s(row, "kind"),
            s(row, "direction"),
            s(row, "stage"),
            s(row, "validity"),
            n(row, "tier"),
            n(row, "born_bar"),
            n(row, "born_t"),
            n(row, "last_bar"),
            n(row, "last_t"),
        );
        if row.get("closed_bar").is_some() {
            line.push_str(&format!(
                " · closed bar {} t {} · outcome {}",
                n(row, "closed_bar"),
                n(row, "closed_t"),
                n(row, "outcome")
            ));
        }
        if row["in_feed"].as_bool() == Some(true) {
            line.push_str(" · in feed");
        } else {
            line.push_str(&format!(" · not in feed ({})", n(row, "hidden")));
        }
        if row["coin_flip"].as_bool() == Some(true) {
            line.push_str(" · coin flip");
        }
        if row["redacted"].as_bool() == Some(true) {
            line.push_str(" · redacted (no data license)");
        }
        if f.get("ledger_n").is_some() {
            line.push_str(&format!(" · record n {}", n(f, "ledger_n")));
        }
        line.push_str(&format!(" · finding {}", s(row, "finding")));
        out.deep(line);
    }
    if rows.is_empty() {
        out.both("No story in range.".to_owned());
    }
    server_comments(&mut out, d.get("comments"), None);
    let ids: Vec<String> = rows.iter().map(|x| s(x, "id").to_owned()).collect();
    let mut data = d;
    if let Some(c) = clock {
        data["clock"] = c;
    }
    let next = data.get("next").cloned();
    out.render(a.format, a.detail, ids, json!({"next": next}), data)
}

// ---------------------------------------------------------------------------
// track_record
// ---------------------------------------------------------------------------

pub(super) fn track_record(ctx: &mut Ctx<'_>, a: &TrackRecordArgs) -> Value {
    let mut q = json!({"kind": "ledger"});
    if let Some(m) = a.meaning {
        q["meaning"] = json!(m);
    }
    if let Some(d) = a.direction {
        q["direction"] = json!(d);
    }
    if let Some(k) = &a.key {
        q["key"] = json!(k);
    }
    if a.by_kind {
        q["by_kind"] = json!(true);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("track_record", r);
    }
    let d = r.get("data").cloned().unwrap_or(Value::Null);
    let total = d.get("total").unwrap_or(&Value::Null);
    let mut lines = vec![format!(
        "track record (closed capsules; retracted never count): total {} of {} realized",
        n(total, "realized"),
        n(total, "n")
    )];
    for row in d
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let dir = row
            .get("direction")
            .and_then(Value::as_str)
            .map_or(String::new(), |x| format!(" ({x})"));
        lines.push(format!(
            "- {}{dir}: {} of {} realized",
            s(row, "key"),
            n(row, "realized"),
            n(row, "n")
        ));
    }
    ok_text(lines.join("\n"), d)
}
