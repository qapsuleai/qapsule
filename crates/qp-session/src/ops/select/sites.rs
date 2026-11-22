//! Selector sites: where in a message a selector may stand.

use super::*;

// ---------------------------------------------------------------------------
// Where ids live in messages
// ---------------------------------------------------------------------------

fn push(out: &mut Vec<Site>, v: &Value, base: &str, key: &str, kind: Kind, view: Option<&str>) {
    if v.get(key).is_some_and(|x| !x.is_null()) {
        out.push(Site {
            path: ptr(base, key),
            kind,
            view: view.map(str::to_owned),
        });
    }
}

fn push_all(out: &mut Vec<Site>, v: &Value, base: &str, key: &str, kind: Kind) {
    if let Some(a) = v.get(key).and_then(Value::as_array) {
        let p = ptr(base, key);
        for i in 0..a.len() {
            out.push(Site {
                path: ptr(&p, i),
                kind,
                view: None,
            });
        }
    }
}

fn each<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = (usize, &'a Value)> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().enumerate())
        .into_iter()
        .flatten()
}

fn encoding_sites(out: &mut Vec<Site>, enc: &Value, ep: &str) {
    for ch in ["x", "y", "y2"] {
        if let Some(c) = enc.get(ch).filter(|c| c.is_object()) {
            push(out, c, &ptr(ep, ch), "series", Kind::Series, None);
        }
    }
    if let Some(than) = enc.pointer("/color/when/than").filter(|t| t.is_object()) {
        push(
            out,
            than,
            &format!("{ep}/color/when/than"),
            "series",
            Kind::Series,
            None,
        );
    }
}

fn layer_sites(out: &mut Vec<Site>, l: &Value, lp: &str, view: &str) {
    push(out, l, lp, "series", Kind::Series, None);
    push(out, l, lp, "fact", Kind::Fact, None);
    push(out, l, lp, "finding", Kind::Finding, None);
    push(out, l, lp, "panel", Kind::Panel, Some(view));
    if let Some(enc) = l.get("encoding") {
        encoding_sites(out, enc, &ptr(lp, "encoding"));
    }
}

fn target_sites(out: &mut Vec<Site>, t: &Value, tp: &str, fallback_view: Option<&str>) {
    push(out, t, tp, "view", Kind::View, None);
    let own = ptr(tp, "view");
    let view = if t.get("view").is_some() {
        Some(own.as_str())
    } else {
        fallback_view
    };
    push(out, t, tp, "panel", Kind::Panel, view);
    push(out, t, tp, "layer", Kind::Layer, view);
    push(out, t, tp, "series", Kind::Series, None);
    push(out, t, tp, "item", Kind::Item, None);
}

fn condition_sites(out: &mut Vec<Site>, c: &Value, cp: &str) {
    match c.get("kind").and_then(Value::as_str) {
        Some("threshold") => push(out, c, cp, "series", Kind::Series, None),
        Some("compare") => {
            push(out, c, cp, "left", Kind::Series, None);
            push(out, c, cp, "right", Kind::Series, None);
        }
        _ => {}
    }
}

/// Sites of one op (`p` = its pointer). View fields come before the panel/layer fields
/// that depend on them.
fn op_sites(op: &Value, p: &str) -> Vec<Site> {
    use Kind as K;
    let mut out = Vec::new();
    let o = &mut out;
    let view = ptr(p, "view");
    let vw = Some(view.as_str());
    let name = op.get("op").and_then(Value::as_str).unwrap_or("");
    match name {
        "add_study" => {
            push_all(o, op, p, "inputs", K::Series);
            if let Some(show) = op.get("show").filter(|s| s.is_object()) {
                let sp = ptr(p, "show");
                let sv = ptr(&sp, "view");
                push(o, show, &sp, "view", K::View, None);
                push(o, show, &sp, "with", K::Series, None);
                push(o, show, &sp, "panel", K::Panel, Some(&sv));
            }
        }
        "set_params" | "add_fact" | "append_bars" | "update_last_bar" => {
            push(o, op, p, "series", K::Series, None);
        }
        "add_layer" | "insert_layer" => {
            push(o, op, p, "view", K::View, None);
            if let Some(l) = op.get("layer") {
                layer_sites(o, l, &ptr(p, "layer"), &view);
            }
            push(o, op, p, "after", K::Layer, vw);
            push(o, op, p, "before", K::Layer, vw);
        }
        "remove_layer" | "set_view_range" => push(o, op, p, "view", K::View, None),
        "add_finding" | "update_finding" => {
            push_all(o, op, p, "facts", K::Fact);
            push(o, op, p, "finding", K::Finding, None);
            if let Some(c) = op.get("invalidation") {
                condition_sites(o, c, &ptr(p, "invalidation"));
            }
        }
        "approve_finding" | "reject_finding" | "show_finding" => {
            push(o, op, p, "finding", K::Finding, None);
        }
        "add_intent" => {
            if let Some(c) = op.get("trigger") {
                condition_sites(o, c, &ptr(p, "trigger"));
            }
        }
        "add_narrative" => {
            push(o, op, p, "about", K::Item, None);
            let sp = ptr(p, "steps");
            for (i, s) in each(op, "steps") {
                if let Some(t) = s.get("target") {
                    target_sites(o, t, &ptr(&ptr(&sp, i), "target"), None);
                }
            }
        }
        "delete_layer" | "split_layer" | "move_layer" | "set_encoding" => {
            push(o, op, p, "view", K::View, None);
            push(o, op, p, "layer", K::Layer, vw);
            push(o, op, p, "panel", K::Panel, vw);
            push(o, op, p, "after", K::Layer, vw);
            push(o, op, p, "before", K::Layer, vw);
            if let Some(enc) = op.get("encoding") {
                encoding_sites(o, enc, &ptr(p, "encoding"));
            }
        }
        "insert_panel" => {
            push(o, op, p, "view", K::View, None);
            let lp = ptr(p, "layers");
            for (i, l) in each(op, "layers") {
                layer_sites(o, l, &ptr(&lp, i), &view);
            }
            push(o, op, p, "after", K::Panel, vw);
            push(o, op, p, "before", K::Panel, vw);
        }
        "delete_panel" | "move_panel" | "set_panel" => {
            push(o, op, p, "view", K::View, None);
            push(o, op, p, "panel", K::Panel, vw);
            push(o, op, p, "after", K::Panel, vw);
            push(o, op, p, "before", K::Panel, vw);
        }
        "set_scale" => {
            push(o, op, p, "view", K::View, None);
            push(o, op, p, "scale", K::Scale, vw);
        }
        "set_emphasis" => {
            push(o, op, p, "view", K::View, None);
            let ep = ptr(p, "emphasis");
            for (i, e) in each(op, "emphasis") {
                if let Some(t) = e.get("target") {
                    target_sites(o, t, &ptr(&ptr(&ep, i), "target"), vw);
                }
            }
        }
        "remove" => push(o, op, p, "id", K::Item, None),
        "accept_proposal" | "reject_proposal" => push(o, op, p, "proposal", K::Proposal, None),
        _ => {}
    }
    out
}

/// The id fields of a client message, grouped: one group per op for a command, one
/// group otherwise. Only fields that are present are listed.
pub(crate) fn sites(msg: &Value) -> Vec<Vec<Site>> {
    match msg.get("type").and_then(Value::as_str) {
        Some("command") => each(msg, "ops")
            .map(|(i, op)| op_sites(op, &ptr("/ops", i)))
            .collect(),
        Some("query") => {
            let mut out = Vec::new();
            if let Some(q) = msg.get("query") {
                let p = "/query";
                match q.get("kind").and_then(Value::as_str) {
                    Some("get") => push(&mut out, q, p, "id", Kind::Item, None),
                    Some("study" | "at") => push(&mut out, q, p, "series", Kind::Series, None),
                    Some("hit") => push(&mut out, q, p, "view", Kind::View, None),
                    Some("present") => push(&mut out, q, p, "finding", Kind::Finding, None),
                    Some("verify_evidence") => push(&mut out, q, p, "item", Kind::Item, None),
                    Some("binding") => push(&mut out, q, p, "series", Kind::Series, None),
                    Some("capsule") => push(&mut out, q, p, "capsule", Kind::Capsule, None),
                    _ => {}
                }
            }
            vec![out]
        }
        Some("subscribe") => {
            let mut out = Vec::new();
            if let Some(t) = msg.get("topic") {
                let p = "/topic";
                match t.get("kind").and_then(Value::as_str) {
                    Some("changes") => push_all(&mut out, t, p, "ids", Kind::Item),
                    Some("finding") => push(&mut out, t, p, "finding", Kind::Finding, None),
                    Some("view") => push(&mut out, t, p, "view", Kind::View, None),
                    Some("condition") => {
                        if let Some(c) = t.get("condition") {
                            condition_sites(&mut out, c, "/topic/condition");
                        }
                    }
                    _ => {}
                }
            }
            vec![out]
        }
        Some("input") => {
            // (R4/S1) `open.capsule` takes a capsule selector too (§17.2).
            let mut out = Vec::new();
            if let Some(g) = msg.get("gesture") {
                if g.get("kind").and_then(Value::as_str) == Some("open") {
                    push(&mut out, g, "/gesture", "capsule", Kind::Capsule, None);
                }
            }
            vec![out]
        }
        Some("attend") => {
            let mut out = Vec::new();
            if let Some(d) = msg.get("directive") {
                for (i, s) in each(d, "steps") {
                    if let Some(t) = s.get("target") {
                        let tp = format!("/directive/steps/{i}/target");
                        target_sites(&mut out, t, &tp, None);
                    }
                }
            }
            vec![out]
        }
        _ => Vec::new(),
    }
}

/// Whether the value at a site is a selector (an object; ids are strings or numbers).
pub(crate) fn is_selector(msg: &Value, site: &Site) -> bool {
    msg.pointer(&site.path).is_some_and(Value::is_object)
}
