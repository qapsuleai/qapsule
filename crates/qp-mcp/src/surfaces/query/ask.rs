//! The `ask` tool: one query in the small language, answered from `qp/0` queries.

use super::*;

// ---------------------------------------------------------------------------------------
// The tools.

#[allow(clippy::too_many_lines)]
pub(super) fn ask(ctx: &mut Ctx<'_>, args: Value) -> Result<Value, Fail> {
    let mut args = args;
    check_keys(&args, "", &ASK_KEYS, "ask")?;
    let raw_find = args
        .get("find")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let Some(raw_find) = raw_find else {
        return Err(Fail::schema(
            "/find",
            "`find` is required (a noun)".to_owned(),
            &FINDS,
            vec![],
        ));
    };
    let canon = ALIASES
        .iter()
        .find(|(a, _)| *a == raw_find)
        .map_or(raw_find.as_str(), |(_, c)| *c)
        .to_owned();
    if !FINDS.contains(&canon.as_str()) {
        return Err(Fail::schema(
            "/find",
            format!("`{raw_find}` is not something ask can find"),
            &FINDS,
            did_you_mean(&raw_find, FINDS),
        ));
    }
    args["find"] = json!(canon);
    let find: Find = serde_json::from_value(json!(canon)).map_err(|e| serde_fail(&e, "find"))?;
    if let Some(w) = args.get("where") {
        if !w.is_null() {
            check_keys(
                w,
                "/where",
                where_keys(find),
                &format!("where (find {canon})"),
            )?;
        }
    }
    if let Some(f) = args.get("format").and_then(Value::as_str) {
        const FORMATS: [&str; 3] = ["card", "json", "layered"];
        if !FORMATS.contains(&f) {
            return Err(Fail::schema(
                "/format",
                format!("unknown format `{f}`"),
                &FORMATS,
                did_you_mean(f, FORMATS),
            ));
        }
    }
    let a: AskArgs = serde_json::from_value(args).map_err(|e| serde_fail(&e, "ask"))?;
    if !a.expand.is_empty() && !matches!(a.find, Find::Capsules | Find::Stories) {
        return Err(Fail::invalid(
            "/expand",
            format!("`expand` applies to capsules and stories, not {canon}"),
            "Drop `expand`, or ask for capsules (e.g. {\"find\":\"capsules\",\"where\":{\"id\":\"…\"},\"expand\":[\"evidence\"]}).",
        ));
    }
    let default = match a.find {
        Find::Facts | Find::Changes | Find::Comments => 20,
        Find::Stories => 50,
        Find::Watches | Find::Glossary => 32,
        _ => 10,
    };
    let limit = match a.limit {
        Some(0) => {
            return Err(Fail::invalid(
                "/limit",
                "`limit` must be at least 1".to_owned(),
                "Omit it for the default.",
            ));
        }
        Some(n) => (n as usize).min(100),
        None => default,
    };
    let mut expand = a.expand.clone();
    expand.sort_unstable();
    expand.dedup();
    let w = &a.where_;
    let mut io = Io { ctx, seq: None };
    let out = match a.find {
        Find::Capsules => find_capsules(&mut io, w, &expand, limit)?,
        Find::Stories => find_stories(&mut io, w, &expand, limit)?,
        Find::Facts => find_facts(&mut io, w, limit)?,
        Find::Instrument => find_instrument(&mut io, w)?,
        Find::Ledger => find_ledger(&mut io, w)?,
        Find::Market => find_market(&mut io, w)?,
        Find::Glossary => find_glossary(&mut io, w, limit)?,
        Find::Changes => find_changes(&mut io, w, limit)?,
        Find::Comments => find_comments(&mut io, w, limit)?,
        Find::Watches => find_watches(&mut io, w, limit)?,
        Find::Proposals => find_proposals(&mut io, w, limit)?,
    };
    // `select`: every key must exist on some item (a typo is an error, not silence).
    if !a.select.is_empty() && !out.items.is_empty() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for it in &out.items {
            for v in [&it.summary, &it.raw] {
                if let Some(o) = v.as_object() {
                    known.extend(o.keys().cloned());
                }
            }
        }
        for k in &a.select {
            if !known.contains(k) {
                let exp: Vec<&str> = known.iter().map(String::as_str).collect();
                return Err(Fail::schema(
                    "/select",
                    format!("no {canon} item has a field `{k}`"),
                    &exp,
                    did_you_mean(k, exp.iter().copied()),
                ));
            }
        }
    }
    let seq = io.seq;
    Ok(shape(
        &canon,
        a.format.unwrap_or_default(),
        &a.select,
        out,
        seq,
    ))
}
