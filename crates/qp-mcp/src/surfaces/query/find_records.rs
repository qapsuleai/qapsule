//! `ask` finds over the record nouns: ledger, market, glossary, changes, comments,
//! watches, proposals.

use super::*;

pub(super) fn find_ledger(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let mut q = json!({"kind": "ledger"});
    if let Some(k) = &w.key {
        q["key"] = json!(k);
    }
    if let Some(m) = &w.meaning {
        q["meaning"] = ser(m);
    }
    if let Some(d) = &w.detector {
        q["detector"] = json!(d);
    }
    if let Some(d) = &w.direction {
        q["direction"] = ser(d);
    }
    if w.by_kind == Some(true) {
        q["by_kind"] = json!(true);
    }
    let data = io.query(q)?;
    let rows = arr(&data, "rows");
    let mut out = Out {
        total: rows.len(),
        ..Out::default()
    };
    for r in rows {
        out.items.push(Item {
            id: s(r, "key").to_owned(),
            refs: Vec::new(),
            raw: r.clone(),
            summary: r.clone(),
            card: vec![format!(
                "[{}] closed {} · realized {}{}",
                s(r, "key"),
                t(r, "n"),
                t(r, "realized"),
                r.get("direction")
                    .map(|d| format!(" · direction {}", d.as_str().unwrap_or("")))
                    .unwrap_or_default()
            )],
            parts: Vec::new(),
        });
    }
    if let Some(tot) = data.get("total") {
        out.extra.push((
            "ledger_total",
            tot.clone(),
            tot.clone(),
            vec![format!(
                "total: closed {} · realized {}",
                t(tot, "n"),
                t(tot, "realized")
            )],
        ));
    }
    Ok(out)
}

/// The market status as the authority projects it for this principal (§24.2).
pub(super) fn market_json(io: &Io<'_, '_>) -> Option<Value> {
    let p = io.ctx.principal().to_owned();
    let m = io.ctx.hub().borrow().session().market_for(&p)?;
    serde_json::to_value(m).ok()
}

pub(super) fn find_market(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let mut out = Out::default();
    let Some(mut m) = market_json(io) else {
        out.extra.push((
            "market",
            Value::Null,
            Value::Null,
            vec!["market: the host has given no market status".to_owned()],
        ));
        return Ok(out);
    };
    if let (Some(sym), Some(ph)) = (&sym, m.get_mut("phases").and_then(Value::as_object_mut)) {
        ph.retain(|k, _| k == sym);
    }
    let phases = m
        .get("phases")
        .and_then(Value::as_object)
        .map(|p| {
            p.iter()
                .map(|(k, v)| format!("{k} {}", s(v, "key")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    out.total = 1;
    out.items.push(Item {
        id: "market".to_owned(),
        refs: Vec::new(),
        raw: m.clone(),
        summary: m.clone(),
        card: vec![format!(
            "[market] open {} · holiday {} · source {} · delayed {} · stale {} · exchange time {} · phases {}",
            t(&m, "open"),
            t(&m, "holiday"),
            t(&m, "source"),
            t(&m, "delayed"),
            t(&m, "stale"),
            t(&m, "exchange_time"),
            if phases.is_empty() { "-".to_owned() } else { phases },
        )],
        parts: Vec::new(),
    });
    Ok(out)
}

/// Glossary: the connection's capsule voice is set to `selvi` for one read (capsule items
/// then carry `voice.glossary`, §20.2) and set back to `r4`.
pub(super) fn set_voice(io: &mut Io<'_, '_>, voice: &str) -> Result<(), Fail> {
    let id = io.ctx.request_id();
    let r = io
        .ctx
        .send(&json!({"type": "input", "id": id, "gesture": {"kind": "style", "voice": voice}}));
    if r.get("type").and_then(Value::as_str) == Some("error") {
        return Err(Fail::protocol("input style", &r));
    }
    Ok(())
}

pub(super) fn glossary_capsules(io: &mut Io<'_, '_>, w: &Where) -> Result<Vec<Value>, Fail> {
    let one = w
        .capsule
        .clone()
        .or_else(|| w.id.clone().map(CapsuleRef::Id));
    if let Some(r) = one {
        let d = io.capsule_detail(&r)?;
        return Ok(vec![d.get("item").cloned().unwrap_or(Value::Null)]);
    }
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &w.instrument {
        q["instrument"] = json!(io.instrument(i, "/where/instrument")?);
    }
    Ok(arr(&io.query(q)?, "capsules").to_vec())
}

pub(super) fn find_glossary(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    set_voice(io, "selvi")?;
    let read = glossary_capsules(io, w);
    set_voice(io, "r4")?;
    let caps = read?;
    let mut terms: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for c in &caps {
        for g in c
            .pointer("/voice/glossary")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            let e = terms
                .entry(s(g, "term").to_owned())
                .or_insert_with(|| (s(g, "text").to_owned(), Vec::new()));
            e.1.push(s(c, "id").to_owned());
        }
    }
    let mut out = Out {
        total: terms.len(),
        ..Out::default()
    };
    for (term, (text, caps)) in terms.into_iter().take(limit) {
        let v = json!({"id": term, "term": term, "text": text, "capsules": caps});
        out.items.push(Item {
            id: term.clone(),
            refs: caps.clone(),
            raw: v.clone(),
            summary: v,
            card: vec![format!("[{term}] {text} (in {})", caps.join(", "))],
            parts: Vec::new(),
        });
    }
    Ok(out)
}

/// Ids an entry's changes touch (the `id` of each changed item, or a named id field).
pub(super) fn touched(entry: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ch in arr(entry, "changes") {
        if let Some(o) = ch.as_object() {
            for (k, v) in o {
                if k == "op" {
                    continue;
                }
                let id = v.get("id").and_then(Value::as_str).or_else(|| {
                    matches!(
                        k.as_str(),
                        "id" | "capsule"
                            | "finding"
                            | "series"
                            | "view"
                            | "intent"
                            | "proposal"
                            | "comment"
                    )
                    .then(|| v.as_str())
                    .flatten()
                });
                if let Some(id) = id {
                    if !out.iter().any(|x| x == id) {
                        out.push(id.to_owned());
                    }
                }
            }
        }
    }
    out
}

pub(super) fn find_changes(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let mut q = json!({"kind": "log", "limit": limit});
    if let Some(x) = w.after_seq {
        q["after_seq"] = json!(x);
    }
    if let Some(x) = w.since_t {
        q["since_t"] = json!(x);
    }
    if let Some(x) = w.within_ms {
        q["within_ms"] = json!(x);
    }
    if let Some(a) = &w.author {
        q["author"] = json!(a);
    }
    let data = io.query(q)?;
    let entries = arr(&data, "entries");
    let mut out = Out {
        total: entries.len(),
        ..Out::default()
    };
    for e in entries {
        let ops: Vec<Value> = arr(e, "changes")
            .iter()
            .filter_map(|c| c.get("op").cloned())
            .collect();
        let ids = touched(e);
        let mut sum = pick(
            e,
            &["seq", "at", "author", "summary", "undoes", "approved_by"],
        );
        sum["ops"] = json!(ops);
        sum["ids"] = json!(ids);
        out.items.push(Item {
            id: format!("seq {}", t(e, "seq")),
            refs: ids.clone(),
            raw: e.clone(),
            summary: sum,
            card: vec![format!(
                "[seq {}] {} · {} · {} change(s){}",
                t(e, "seq"),
                s(e, "author"),
                s(e, "summary"),
                ops.len(),
                if ids.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", ids.join(", "))
                }
            )],
            parts: Vec::new(),
        });
    }
    if let Some(b) = w.since_bar {
        let caps = io.query(json!({"kind": "capsules", "since_bar": b}))?;
        let moved: Vec<Value> = arr(&caps, "capsules")
            .iter()
            .map(|c| {
                pick(
                    c,
                    &["id", "instrument", "kind", "direction", "stage", "last_bar"],
                )
            })
            .collect();
        let lines = moved
            .iter()
            .map(|c| {
                format!(
                    "moved [{}] {} {} {} · stage {} at bar {}",
                    s(c, "id"),
                    s(c, "instrument"),
                    s(c, "kind"),
                    s(c, "direction"),
                    s(c, "stage"),
                    t(c, "last_bar")
                )
            })
            .collect();
        out.extra.push(("moved", caps, Value::Array(moved), lines));
    }
    Ok(out)
}

pub(super) fn find_comments(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let cap = match &w.capsule {
        Some(r) => Some(
            s(
                &io.capsule_detail(r)?
                    .get("item")
                    .cloned()
                    .unwrap_or(Value::Null),
                "id",
            )
            .to_owned(),
        ),
        None => None,
    };
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let inst_caps: Option<BTreeSet<String>> = match &sym {
        Some(x) => Some(
            arr(
                &io.query(json!({"kind": "capsules", "instrument": x}))?,
                "capsules",
            )
            .iter()
            .map(|c| s(c, "id").to_owned())
            .collect(),
        ),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "comments"}))?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    list.retain(|c| {
        let about_cap = c.pointer("/about/capsule").and_then(Value::as_str);
        let about_ins = c.pointer("/about/instrument").and_then(Value::as_str);
        w.id.as_ref().is_none_or(|i| s(c, "id") == i)
            && cap.as_ref().is_none_or(|x| about_cap == Some(x.as_str()))
            && sym.as_ref().is_none_or(|x| {
                about_ins == Some(x.as_str())
                    || about_cap.is_some_and(|a| inst_caps.as_ref().is_some_and(|s| s.contains(a)))
            })
    });
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for c in list.iter().take(limit) {
        let sum = comment_summary(c);
        let about = c.get("about").cloned().unwrap_or(Value::Null);
        out.items.push(Item {
            id: s(c, "id").to_owned(),
            refs: arr(c, "cites")
                .iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect(),
            raw: c.clone(),
            card: vec![format!(
                "[{}] about {} · {} · by {}{} · \"{}\" · cites {}",
                s(c, "id"),
                about,
                t(c, "lang"),
                t(&sum, "author"),
                c.get("on_behalf")
                    .map(|p| format!(" for {}", p.as_str().unwrap_or("")))
                    .unwrap_or_default(),
                s(c, "text"),
                t(c, "cites"),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

pub(super) fn find_watches(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "intents"}))?;
    let mut list: Vec<Value> = arr(&data, "items")
        .iter()
        .filter(|i| i.get("watch").is_some())
        .cloned()
        .collect();
    list.retain(|i| {
        w.id.as_ref().is_none_or(|x| s(i, "id") == x)
            && w.state
                .as_ref()
                .is_none_or(|x| i.get("state").and_then(Value::as_str).unwrap_or("active") == x)
            && sym.as_ref().is_none_or(|x| {
                i.pointer("/watch/match/instrument").and_then(Value::as_str) == Some(x.as_str())
            })
    });
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for i in list.iter().take(limit) {
        let wch = i.get("watch").cloned().unwrap_or(Value::Null);
        let mut sum = pick(i, &["id", "text", "state", "matches", "active"]);
        for k in ["for", "match", "expires_t", "max_matches"] {
            if let Some(x) = wch.get(k) {
                sum[k] = x.clone();
            }
        }
        out.items.push(Item {
            id: s(i, "id").to_owned(),
            refs: Vec::new(),
            raw: i.clone(),
            card: vec![format!(
                "[{}] for {} · state {} · matches {} · \"{}\" · match {}{}{}",
                s(i, "id"),
                t(&wch, "for"),
                i.get("state").and_then(Value::as_str).unwrap_or("active"),
                t(i, "matches"),
                s(i, "text"),
                t(&wch, "match"),
                wch.get("expires_t")
                    .map(|x| format!(" · expires_t {x}"))
                    .unwrap_or_default(),
                wch.get("max_matches")
                    .map(|x| format!(" · max_matches {x}"))
                    .unwrap_or_default(),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

pub(super) fn find_proposals(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    const STATES: [&str; 4] = ["pending", "accepted", "rejected", "expired"];
    let mut q = json!({"kind": "list", "collection": "proposals"});
    if let Some(st) = &w.state {
        if !STATES.contains(&st.as_str()) {
            return Err(Fail::schema(
                "/where/state",
                format!("`{st}` is not a proposal state"),
                &STATES,
                did_you_mean(st, STATES),
            ));
        }
        q["state"] = json!(st);
    }
    let data = io.query(q)?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    if let Some(i) = &w.id {
        list.retain(|p| s(p, "id") == i);
    }
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for p in list.iter().take(limit) {
        let ops: Vec<Value> = arr(p, "ops")
            .iter()
            .filter_map(|o| o.get("op").cloned())
            .collect();
        let mut sum = pick(
            p,
            &["id", "state", "reason", "note", "expires_at", "decided_seq"],
        );
        sum["ops"] = json!(ops);
        if let Some(a) = p.pointer("/provenance/author/name") {
            sum["author"] = a.clone();
        }
        if let Some(a) = p.pointer("/decided_by/name") {
            sum["decided_by"] = a.clone();
        }
        out.items.push(Item {
            id: s(p, "id").to_owned(),
            refs: Vec::new(),
            raw: p.clone(),
            card: vec![format!(
                "[{}] {} · by {} · ops {}{}",
                s(p, "id"),
                s(p, "state"),
                t(&sum, "author"),
                ops.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
                p.get("reason")
                    .map(|r| format!(" · reason \"{}\"", r.as_str().unwrap_or("")))
                    .unwrap_or_default(),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}
