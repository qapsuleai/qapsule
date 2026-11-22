//! `ask` finds over the story nouns: capsules, stories, facts, instrument.

use super::*;

// ---------------------------------------------------------------------------------------
// Finds.

pub(super) fn stage_list(w: &Where) -> Vec<String> {
    w.stage
        .as_ref()
        .map(|st| {
            st.to_vec()
                .iter()
                .filter_map(|x| serde_json::to_value(x).ok())
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn ser(v: &impl serde::Serialize) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

pub(super) fn find_capsules(
    io: &mut Io<'_, '_>,
    w: &Where,
    expand: &[Expand],
    limit: usize,
) -> Result<Out, Fail> {
    let one = w
        .capsule
        .clone()
        .or_else(|| w.id.clone().map(CapsuleRef::Id));
    if let Some(r) = one {
        let detail = io.capsule_detail(&r)?;
        let item = detail.get("item").cloned().unwrap_or(Value::Null);
        let mut out = Out {
            total: 1,
            ..Out::default()
        };
        out.items.push(capsule_item(&item, &detail, expand));
        if let Some(wh) = detail.get("withheld") {
            out.extra.push((
                "withheld",
                wh.clone(),
                wh.clone(),
                vec![format!("withheld: {wh}")],
            ));
        }
        return Ok(out);
    }
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &w.instrument {
        q["instrument"] = json!(io.instrument(i, "/where/instrument")?);
    }
    let stages = stage_list(w);
    if !stages.is_empty() {
        q["stages"] = json!(stages);
    }
    if let Some(o) = w.open {
        q["open"] = json!(o);
    }
    if let Some(m) = &w.meaning {
        q["meaning"] = ser(m);
    }
    if let Some(b) = w.since_bar {
        q["since_bar"] = json!(b);
    }
    if let Some(x) = w.since_t {
        q["since_t"] = json!(x);
    }
    if w.audience == Some(true) {
        q["audience"] = json!(true);
    }
    let local = w.direction.is_some() || w.holdings == Some(true);
    let latest = w.latest == Some(true);
    if latest && !local {
        q["latest"] = json!(true);
    }
    let data = io.query(q)?;
    let mut list: Vec<Value> = arr(&data, "capsules").to_vec();
    let mut holdings_note = false;
    if let Some(d) = &w.direction {
        let d = ser(d);
        list.retain(|c| c.get("direction") == Some(&d));
    }
    if w.holdings == Some(true) {
        // The authority marks what concerns the person (§20.2 `relevant`: a position or
        // an active watch of theirs; an agent reads its person's audience, §25.7).
        list.retain(|c| c.get("relevant") == Some(&Value::Bool(true)));
        if list.is_empty() {
            holdings_note = true;
        }
    }
    if latest && local {
        // The most recently born (birth bar, then id), as the protocol's `latest`.
        if let Some(best) = list
            .iter()
            .max_by(|a, b| {
                (a.get("born_bar").and_then(Value::as_u64), s(a, "id"))
                    .cmp(&(b.get("born_bar").and_then(Value::as_u64), s(b, "id")))
            })
            .cloned()
        {
            list = vec![best];
        }
    }
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for c in list.iter().take(limit) {
        let detail = io.capsule_detail(&CapsuleRef::Id(s(c, "id").to_owned()))?;
        out.items.push(capsule_item(c, &detail, expand));
    }
    if let Some(sc) = server_comments(&data) {
        out.extra.push(sc);
    }
    if holdings_note {
        let n = "holdings: the authority marks no capsule as relevant (a position or an active watch) for you";
        out.extra
            .push(("note", json!(n), json!(n), vec![n.to_owned()]));
    }
    Ok(out)
}

pub(super) fn find_stories(
    io: &mut Io<'_, '_>,
    w: &Where,
    expand: &[Expand],
    limit: usize,
) -> Result<Out, Fail> {
    let Some(i) = &w.instrument else {
        let syms = io.symbols()?;
        return Err(Fail(
            json!({"error": {"code": "schema", "path": "/where/instrument",
            "message": "stories need `where.instrument`", "expected": syms,
            "hint": "e.g. {\"find\":\"stories\",\"where\":{\"instrument\":\"THYAO\"}}"}}),
        ));
    };
    let sym = io.instrument(i, "/where/instrument")?;
    let mut q = json!({"kind": "stories", "instrument": sym});
    if let Some(r) = &w.range {
        q["range"] = ser(r);
    }
    if let Some(a) = &w.after {
        q["after"] = json!(a);
    }
    let data = io.query(q)?;
    let mut rows: Vec<Value> = arr(&data, "stories").to_vec();
    if let Some(m) = &w.meaning {
        let m = ser(m);
        rows.retain(|r| r.get("kind") == Some(&m));
    }
    if let Some(d) = &w.direction {
        let d = ser(d);
        rows.retain(|r| r.get("direction") == Some(&d));
    }
    let stages = stage_list(w);
    if !stages.is_empty() {
        rows.retain(|r| stages.iter().any(|x| x == s(r, "stage")));
    }
    if let Some(o) = w.open {
        rows.retain(|r| r.get("closed_bar").is_none() == o);
    }
    let mut out = Out {
        total: rows.len(),
        ..Out::default()
    };
    for r in rows.iter().take(limit) {
        let mut sum = pick(
            r,
            &[
                "id",
                "finding",
                "kind",
                "direction",
                "stage",
                "outcome",
                "validity",
                "tier",
                "born_bar",
                "last_bar",
                "closed_bar",
                "in_feed",
                "hidden",
                "coin_flip",
                "redacted",
            ],
        );
        if let Some(f) = r.get("face") {
            sum["face"] = pick(
                f,
                &["event", "when", "moved", "validity_phrase", "ledger_short"],
            );
        }
        let face = r.get("face").cloned().unwrap_or(Value::Null);
        let card = vec![format!(
            "[{}] {} {} · stage {} · outcome {} · {} · tier {} · born bar {}, last bar {}, closed bar {} · in feed {}{} · {} · {} · ledger {}",
            s(r, "id"),
            s(r, "kind"),
            s(r, "direction"),
            s(r, "stage"),
            t(r, "outcome"),
            s(r, "validity"),
            t(r, "tier"),
            t(r, "born_bar"),
            t(r, "last_bar"),
            t(r, "closed_bar"),
            t(r, "in_feed"),
            r.get("hidden").map(|h| format!(" (hidden: {})", h.as_str().unwrap_or(""))).unwrap_or_default(),
            s(&face, "event"),
            s(&face, "when"),
            s(&face, "ledger_short"),
        )];
        let parts = if expand.is_empty() {
            Vec::new()
        } else {
            let detail = io.capsule_detail(&CapsuleRef::Id(s(r, "id").to_owned()))?;
            detail_parts(&detail, expand)
        };
        out.items.push(Item {
            id: s(r, "id").to_owned(),
            refs: vec![s(r, "finding").to_owned()],
            raw: r.clone(),
            summary: sum,
            card,
            parts,
        });
    }
    for k in ["series", "next"] {
        if let Some(v) = data.get(k) {
            out.extra.push((
                if k == "series" { "series" } else { "next" },
                v.clone(),
                v.clone(),
                vec![format!("{k}: {}", v.as_str().unwrap_or(""))],
            ));
        }
    }
    if let Some(n) = data.get("total") {
        out.extra.push((
            "instrument_total",
            n.clone(),
            n.clone(),
            vec![format!("stories of the instrument in range: {n}")],
        ));
    }
    if let Some(sc) = server_comments(&data) {
        out.extra.push(sc);
    }
    Ok(out)
}

pub(super) fn fact_item(f: &Value) -> Item {
    let m = f.get("meaning").cloned().unwrap_or(Value::Null);
    let mut sum = pick(f, &["id", "kind", "series", "bar", "t", "text"]);
    for (k, p) in [
        ("instrument", "/instrument"),
        ("meaning", "/kind"),
        ("direction", "/direction"),
        ("license", "/license/class"),
    ] {
        if let Some(x) = m.pointer(p) {
            sum[k] = x.clone();
        }
    }
    let ev = f.get("evidence").cloned().unwrap_or(json!({}));
    sum["evidence"] = ev.clone();
    if let Some(d) = f.pointer("/evidence_ref/digest") {
        sum["evidence_ref"] = d.clone();
    }
    let vals = ev
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let card = vec![format!(
        "[{}] {} · {} · bar {} · t {} · license {} · evidence {}",
        s(f, "id"),
        s(f, "kind"),
        t(&m, "instrument"),
        t(f, "bar"),
        t(f, "t"),
        m.pointer("/license/class")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        if vals.is_empty() {
            "-".to_owned()
        } else {
            vals
        },
    )];
    Item {
        id: s(f, "id").to_owned(),
        refs: Vec::new(),
        raw: f.clone(),
        summary: sum,
        card,
        parts: Vec::new(),
    }
}

pub(super) fn find_facts(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let only: Option<BTreeSet<String>> = match &w.capsule {
        Some(r) => {
            let d = io.capsule_detail(r)?;
            Some(
                arr(&d, "facts")
                    .iter()
                    .filter_map(|f| f.get("id").and_then(Value::as_str).map(str::to_owned))
                    .collect(),
            )
        }
        None => None,
    };
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "facts"}))?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    list.retain(|f| {
        let m = f.get("meaning").cloned().unwrap_or(Value::Null);
        w.id.as_ref().is_none_or(|i| s(f, "id") == i)
            && only.as_ref().is_none_or(|o| o.contains(s(f, "id")))
            && sym.as_ref().is_none_or(|x| s(&m, "instrument") == x)
            && w.meaning
                .as_ref()
                .is_none_or(|x| m.get("kind") == Some(&ser(x)))
            && w.direction
                .as_ref()
                .is_none_or(|x| m.get("direction") == Some(&ser(x)))
            && w.kind
                .as_ref()
                .is_none_or(|k| s(f, "kind").eq_ignore_ascii_case(k))
            && w.since_bar
                .is_none_or(|b| f.get("bar").and_then(Value::as_u64).is_some_and(|x| x >= b))
            && w.since_t
                .is_none_or(|b| f.get("t").and_then(Value::as_u64).is_some_and(|x| x >= b))
    });
    // Latest first.
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    out.items = list.iter().take(limit).map(fact_item).collect();
    if out.items.is_empty() {
        if let (Some(k), true) = (&w.kind, list.is_empty()) {
            let kinds: BTreeSet<String> = arr(&data, "items")
                .iter()
                .map(|f| s(f, "kind").to_owned())
                .collect();
            let dym = did_you_mean(k, kinds.iter().map(String::as_str));
            if !dym.is_empty() {
                out.extra.push((
                    "did_you_mean",
                    json!(dym),
                    json!(dym),
                    vec![format!("did_you_mean kind: {}", dym.join(", "))],
                ));
            }
        }
    }
    Ok(out)
}

pub(super) fn find_instrument(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let syms = match &w.instrument {
        Some(i) => vec![io.instrument(i, "/where/instrument")?],
        None => io.symbols()?,
    };
    let series = io.query(json!({"kind": "list", "collection": "series"}))?;
    let bindings = io.query(json!({"kind": "binding"}))?;
    let market = market_json(io);
    let mut out = Out {
        total: syms.len(),
        ..Out::default()
    };
    for sym in &syms {
        let mut ser_rows = Vec::new();
        let mut last = Value::Null;
        for x in arr(&series, "items") {
            if x.pointer("/instrument/symbol").and_then(Value::as_str) != Some(sym.as_str()) {
                continue;
            }
            let id = s(x, "id").to_owned();
            let b = arr(&bindings, "bindings")
                .iter()
                .find(|b| s(b, "series") == id);
            let state = b.map_or(json!("document"), |b| {
                b.get("state").cloned().unwrap_or(Value::Null)
            });
            let mut row = json!({"id": id, "interval": x.get("interval").cloned().unwrap_or(Value::Null), "data": state});
            if let Some(reason) = b.and_then(|b| b.get("reason")) {
                row["reason"] = reason.clone();
            }
            ser_rows.push(row);
            if last.is_null() {
                last = match io.query(json!({"kind": "study", "series": id, "reduce": "last"})) {
                    Ok(d) => {
                        let v = d.get("values").cloned().unwrap_or(Value::Null);
                        let mut l = json!({"series": id});
                        for k in ["o", "h", "l", "c", "v"] {
                            if let Some(x) = v.get(k) {
                                l["bar"] = x.get("bar").cloned().unwrap_or(Value::Null);
                                l["t"] = x.get("t").cloned().unwrap_or(Value::Null);
                                l[k] = x.get("value").cloned().unwrap_or(Value::Null);
                            }
                        }
                        l
                    }
                    Err(Fail(e)) => {
                        json!({"series": id, "unavailable": e.pointer("/error/code").cloned().unwrap_or(Value::Null)})
                    }
                };
            }
        }
        let caps = io.query(json!({"kind": "capsules", "instrument": sym}))?;
        let list = arr(&caps, "capsules");
        let open: Vec<String> = list
            .iter()
            .filter(|c| !CLOSED.contains(&s(c, "stage")))
            .map(|c| s(c, "id").to_owned())
            .collect();
        let phase = market
            .as_ref()
            .and_then(|m| m.pointer(&format!("/phases/{sym}/key")).cloned())
            .unwrap_or(Value::Null);
        let sum = json!({
            "id": sym, "series": ser_rows, "last": last,
            "capsules": list.len(), "open_capsules": open,
            "server_comments": arr(&caps, "comments").len(), "phase": phase,
        });
        let last_txt = if last.get("c").is_some() {
            format!(
                "last bar {} (t {}): o {} h {} l {} c {} v {}",
                t(&last, "bar"),
                t(&last, "t"),
                t(&last, "o"),
                t(&last, "h"),
                t(&last, "l"),
                t(&last, "c"),
                t(&last, "v")
            )
        } else {
            format!("last bar unavailable ({})", t(&last, "unavailable"))
        };
        let card = vec![
            format!(
                "[{sym}] series {} · {last_txt}",
                ser_rows
                    .iter()
                    .map(|r| format!("{} {} data {}", s(r, "id"), t(r, "interval"), t(r, "data")))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            format!(
                "  capsules {} (open: {}) · server comments {} · session phase {}",
                list.len(),
                if open.is_empty() {
                    "none".to_owned()
                } else {
                    open.join(", ")
                },
                arr(&caps, "comments").len(),
                phase.as_str().unwrap_or("-"),
            ),
        ];
        out.items.push(Item {
            id: sym.clone(),
            refs: open.clone(),
            raw: json!({"series": ser_rows, "last": last, "capsules": caps, "phase": phase}),
            summary: sum,
            card,
            parts: Vec::new(),
        });
    }
    Ok(out)
}
