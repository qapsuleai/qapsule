//! Reference by meaning (R2, `docs/PROTOCOL.md` §17): structured selectors.
//!
//! Wherever a message field takes an item id, the client may send a *selector* object
//! instead (`{"study":"rsi","params":{"period":14}}`). Before the message is parsed into
//! its typed form, the authority finds every such field ([`sites`]), resolves each
//! selector against the current document ([`Resolver::resolve`]) and writes the id in
//! its place. The typed message, the log and proposals therefore only ever contain ids;
//! the pointers and ids are returned to the client (`ack.resolved`, `result.resolved`).
//!
//! Rules (deterministic; no fuzzy matching):
//! - every given key must hold (AND); text compares case-insensitively, numbers exactly;
//! - nested selectors (`shows`, `on`, `panel`, `fact`, `finding`) stand for the *set* of
//!   items they match (after their own `first`/`latest`); an id stands for itself;
//! - candidates are kept in the kind's order (series, views, proposals: document order;
//!   panels: stack order; layers: paint order; facts: bar, then document order;
//!   findings: latest fact bar, then document order);
//! - one match: resolved; several: `first`/`latest` pick the first/last, otherwise
//!   `ambiguous` with the matches as candidates in that order; none: `not_found` with the
//!   closest items (most keys satisfied, then the kind's order).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{Mark, Position, View};
use qp_schema::protocol::{
    Candidate, CapsuleSel, ErrorCode, FactRef, FactSel, FindingRef, FindingSel, LayerSel, PanelRef,
    PanelSel, ProposalSel, ProtocolError, ScaleSel, SeriesKind, SeriesRef, SeriesSel, ViewSel,
};
use qp_schema::{Document, Fact, Finding, Series, SeriesData, SeriesSource};
use serde_json::{json, Value};

use crate::canon::{layer_data, position_constant, series_main, Data};
use crate::error::{invalid, num, ptr, PResult, E};
use crate::expand::{need_view, summarize_ops, view_bar_series};
use crate::locate::{locate, Violation, ViolationKind};
use crate::summary::{drawing_outputs, layer_about, series_label};

/// At most this many candidates in an `ambiguous` error.
const MAX_AMBIGUOUS: usize = 10;
/// At most this many candidates in a `not_found` error.
const MAX_NOT_FOUND: usize = 5;

/// What kind of item a field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Series,
    View,
    Panel,
    Layer,
    Fact,
    Finding,
    Proposal,
    Scale,
    Item,
    /// (R4) A capsule with a story (`capsule` query).
    Capsule,
}

impl Kind {
    /// Name of the selector type in the generated schema's `$defs`.
    fn def(self) -> &'static str {
        match self {
            Kind::Series => "SeriesSel",
            Kind::View => "ViewSel",
            Kind::Panel => "PanelSel",
            Kind::Layer => "LayerSel",
            Kind::Fact => "FactSel",
            Kind::Finding => "FindingSel",
            Kind::Proposal => "ProposalSel",
            Kind::Scale => "ScaleSel",
            Kind::Item => "ItemSel",
            Kind::Capsule => "CapsuleSel",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Kind::Series => "series",
            Kind::View => "view",
            Kind::Panel => "panel",
            Kind::Layer => "layer",
            Kind::Fact => "fact",
            Kind::Finding => "finding",
            Kind::Proposal => "proposal",
            Kind::Scale => "scale",
            Kind::Item => "item",
            Kind::Capsule => "capsule",
        }
    }

    fn plural(self) -> &'static str {
        match self {
            Kind::Series => "series",
            Kind::Scale => "scales",
            k => match k.noun() {
                "view" => "views",
                "panel" => "panels",
                "layer" => "layers",
                "fact" => "facts",
                "finding" => "findings",
                "proposal" => "proposals",
                "capsule" => "capsules",
                _ => "items",
            },
        }
    }
}

/// A field of a message that holds an id or a selector.
#[derive(Debug, Clone)]
pub(crate) struct Site {
    /// JSON pointer of the field.
    pub path: String,
    pub kind: Kind,
    /// Pointer of the field that names the view a panel/layer/scale belongs to.
    pub view: Option<String>,
}

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

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A schema violation as a protocol error, its path prefixed with `prefix`.
pub(crate) fn violation_error(v: &Violation, prefix: &str) -> ProtocolError {
    let mut b = E::new(ErrorCode::Schema, v.message()).at(format!("{prefix}{}", v.path));
    match &v.kind {
        ViolationKind::UnknownField { field, expected } => {
            b = b
                .dym(field, expected.iter().map(String::as_str))
                .expected(expected.clone())
                .hint("Fix or remove the field; the authority rejects unknown fields.");
        }
        ViolationKind::UnknownValue {
            value: given,
            expected,
            ..
        } => {
            b = b
                .dym(given, expected.iter().map(String::as_str))
                .expected(expected.clone())
                .hint(
                    "Use one of the expected names; {\"kind\":\"describe\"} lists ops and queries.",
                );
        }
        ViolationKind::MissingField { field } => {
            b = b.expected([field.clone()]);
        }
        ViolationKind::WrongType { expected, .. } => {
            b = b.expected([expected.clone()]);
        }
    }
    b.build()
}

fn compact(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 160 {
        let mut t: String = s.chars().take(159).collect();
        t.push('…');
        t
    } else {
        s
    }
}

fn eq_text(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_owned()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// One candidate item, with the outcome of each selector key.
#[derive(Debug, Clone)]
struct Row {
    id: String,
    about: String,
    /// Replacement written into the message (a string id, or a number for panels).
    value: Value,
    checks: Vec<bool>,
}

impl Row {
    fn matches(&self) -> bool {
        self.checks.iter().all(|c| *c)
    }

    fn score(&self) -> usize {
        self.checks.iter().filter(|c| **c).count()
    }
}

/// Rows in the kind's order plus, per text key, the values present in the document
/// (for `did_you_mean`).
struct Outcome {
    rows: Vec<Row>,
    keys: Vec<&'static str>,
    vocab: Vec<(&'static str, String, Vec<String>)>,
}

impl Outcome {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            keys: Vec::new(),
            vocab: Vec::new(),
        }
    }
}

/// Picks the matching rows: all of them, or the first/last one.
fn pick(rows: Vec<Row>, first: bool, latest: bool, path: &str) -> PResult<Vec<Row>> {
    if first && latest {
        return Err(E::new(
            ErrorCode::Invalid,
            "a selector takes `first` or `latest`, not both",
        )
        .at(path)
        .build());
    }
    let mut m: Vec<Row> = rows.into_iter().filter(Row::matches).collect();
    if first {
        m.truncate(1);
    } else if latest && m.len() > 1 {
        m.drain(..m.len() - 1);
    }
    Ok(m)
}

/// The set a nested selector stands for (after its own `first`/`latest`); an empty set
/// is reported where the nested selector is, as `not_found`.
fn nested<S: serde::Serialize>(
    kind: Kind,
    sel: &S,
    o: Outcome,
    first: bool,
    latest: bool,
    path: &str,
) -> PResult<Vec<Row>> {
    let rows = pick(o.rows.clone(), first, latest, path)?;
    if rows.is_empty() {
        let v = serde_json::to_value(sel).unwrap_or(Value::Null);
        return match decide(kind, path, &v, o, first, latest) {
            Err(e) => Err(e),
            Ok(_) => Ok(rows),
        };
    }
    Ok(rows)
}

fn candidates(rows: &[Row], max: usize) -> Vec<Candidate> {
    rows.iter()
        .take(max)
        .map(|r| Candidate {
            id: r.id.clone(),
            about: r.about.clone(),
        })
        .collect()
}

/// Exactly one match, or `ambiguous` / `not_found`.
fn decide(
    kind: Kind,
    path: &str,
    sel: &Value,
    out: Outcome,
    first: bool,
    latest: bool,
) -> PResult<Row> {
    let all = out.rows.clone();
    let mut m = pick(out.rows, first, latest, path)?;
    if m.len() == 1 {
        return Ok(m.remove(0));
    }
    let text = compact(sel);
    if m.len() > 1 {
        let n = m.len();
        return Err(E::new(
            ErrorCode::Ambiguous,
            format!(
                "{n} {} match {text}; the authority does not pick one",
                kind.plural()
            ),
        )
        .at(path)
        .hint(format!(
            "Add a key that tells them apart, add \"first\":true or \"latest\":true (candidates are in {} order), or use one of the candidate ids.",
            order_name(kind)
        ))
        .build_with(candidates(&m, MAX_AMBIGUOUS)));
    }
    // Not found: closest first (most keys satisfied, then the kind's order).
    let mut ranked: Vec<(usize, usize, Row)> = all
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.score(), i, r))
        .filter(|(s, _, _)| *s > 0)
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut closest: Vec<Row> = ranked.into_iter().map(|(_, _, r)| r).collect();
    let mut message = format!("no {} matches {text}", kind.noun());
    let mut dym = Vec::new();
    for (key, given, values) in &out.vocab {
        if !values.iter().any(|v| eq_text(v, given)) {
            message.push_str(&format!("; no {} has {key} `{given}`", kind.noun()));
            dym.extend(crate::suggest::did_you_mean(
                given,
                values.iter().map(String::as_str),
            ));
        }
    }
    if closest.is_empty() {
        closest = all;
    }
    let mut e = E::new(ErrorCode::NotFound, message)
        .at(path)
        .hint(if kind == Kind::Capsule {
            "Candidates are the closest capsules; the `capsules` query lists every capsule with its instrument, kind, direction and stage.".to_owned()
        } else {
            format!(
                "Candidates are the closest {}; welcome.summary shows a `select` for each item that resolves to it.",
                kind.plural()
            )
        })
        .build_with(candidates(&closest, MAX_NOT_FOUND));
    dym.dedup();
    dym.truncate(5);
    e.did_you_mean = dym;
    Err(e)
}

fn order_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Series | Kind::View | Kind::Proposal | Kind::Item => "document",
        Kind::Panel | Kind::Scale => "stack (top to bottom)",
        Kind::Layer => "paint",
        Kind::Fact => "bar",
        Kind::Finding => "latest-fact-bar",
        Kind::Capsule => "latest-transition-bar",
    }
}

trait BuildWith {
    fn build_with(self, c: Vec<Candidate>) -> ProtocolError;
}

impl BuildWith for E {
    fn build_with(self, c: Vec<Candidate>) -> ProtocolError {
        let mut e = self.build();
        e.candidates = c;
        e
    }
}

fn series_kind(s: &Series) -> SeriesKind {
    match (&s.source, &s.data) {
        (SeriesSource::Study { .. }, _) => SeriesKind::Study,
        (_, Some(SeriesData::Points { .. })) => SeriesKind::Points,
        _ => SeriesKind::Bars,
    }
}

fn series_about(s: &Series) -> String {
    match &s.source {
        SeriesSource::Study {
            study,
            inputs,
            params,
            ..
        } => {
            let p: Vec<String> = params
                .iter()
                .map(|(k, v)| format!("{k}={}", num(*v)))
                .collect();
            format!(
                "{}: {study}({}) on {}",
                series_label(s),
                p.join(","),
                inputs.join(",")
            )
        }
        _ => match series_kind(s) {
            SeriesKind::Points => format!("{}: points", series_label(s)),
            _ => format!("{}: bars", series_label(s)),
        },
    }
}

fn fact_about(f: &Fact) -> String {
    format!(
        "{} on {} at bar {}: {}",
        f.kind,
        f.series,
        f.bar,
        short(&f.text, 60)
    )
}

pub(crate) fn finding_bar(doc: &Document, f: &Finding) -> Option<u64> {
    f.facts
        .iter()
        .filter_map(|id| doc.facts.iter().find(|x| &x.id == id))
        .map(|x| x.bar)
        .max()
}

fn finding_facts<'a>(doc: &'a Document, f: &'a Finding) -> impl Iterator<Item = &'a Fact> + 'a {
    f.facts
        .iter()
        .filter_map(move |id| doc.facts.iter().find(|x| &x.id == id))
}

pub(crate) fn finding_about(doc: &Document, f: &Finding) -> String {
    let kinds: BTreeSet<&str> = finding_facts(doc, f).map(|x| x.kind.as_str()).collect();
    let kinds: Vec<&str> = kinds.into_iter().collect();
    let state = doc
        .capsules
        .iter()
        .find(|c| c.finding == f.id)
        .and_then(|c| serde_json::to_value(c.state).ok())
        .and_then(|v| v.as_str().map(str::to_owned));
    let bar = finding_bar(doc, f).map_or_else(String::new, |b| format!(" at bar {b}"));
    let state = state.map_or_else(String::new, |s| format!(", {s}"));
    format!("{} ({}{bar}{state})", short(&f.claim, 60), kinds.join("/"))
}

/// Effective title of a view: its own, or `SYMBOL interval` of its first bar series.
pub(crate) fn view_title(doc: &Document, v: &View) -> Option<String> {
    if let Some(t) = &v.title {
        return Some(t.clone());
    }
    let s = view_bar_series(doc, v)?;
    Some(match (s.symbol(), &s.interval) {
        (Some(sym), Some(i)) => format!("{sym} {i}"),
        (Some(sym), None) => sym.to_owned(),
        _ => s.id.clone(),
    })
}

/// Whether a layer draws `field` (for `output`). Strict (the selector rule): the layer
/// draws that output **by itself** — its `y` names it, or `y` is absent and it draws
/// exactly one default output, that one. Loose (only to explain a miss): also among
/// others — named in `y2`, or one of several default outputs (a Bollinger line).
fn layer_draws(doc: &Document, l: &qp_schema::Layer, field: &str, loose: bool) -> bool {
    let named = |p: &Option<Position>| match p {
        Some(Position::Name(n)) => eq_text(n, field),
        Some(Position::Ref(r)) => {
            (r.series.is_none() || r.series == l.series)
                && r.field.as_deref().is_some_and(|f| eq_text(f, field))
        }
        _ => false,
    };
    if named(&l.encoding.y) || (loose && named(&l.encoding.y2)) {
        return true;
    }
    if l.encoding.y.is_some() {
        return false;
    }
    let defaults: Vec<String> = match layer_data(doc, l) {
        Data::Series(s) => {
            if l.mark == Mark::Line {
                drawing_outputs(s)
            } else if l.mark == Mark::Candle {
                ["o", "h", "l", "c"].map(str::to_owned).to_vec()
            } else {
                vec![series_main(s)]
            }
        }
        d => crate::canon::data_main(doc, d).into_iter().collect(),
    };
    if loose {
        defaults.iter().any(|o| eq_text(o, field))
    } else {
        defaults.len() == 1 && eq_text(&defaults[0], field)
    }
}

fn layer_label(doc: &Document, l: &qp_schema::Layer) -> Option<String> {
    if let Some(t) = &l.label {
        return Some(t.clone());
    }
    match layer_data(doc, l) {
        Data::Series(s) => Some(series_label(s)),
        _ => None,
    }
}

fn layer_constants(l: &qp_schema::Layer) -> Vec<f64> {
    [&l.encoding.y, &l.encoding.x]
        .into_iter()
        .filter_map(|p| p.as_ref().and_then(position_constant))
        .collect()
}

/// Resolves selectors against one document.
pub(crate) struct Resolver<'a> {
    pub doc: &'a Document,
    /// The generated schema whose `$defs` hold the selector types (to check them).
    pub root: Option<&'a Value>,
}

impl<'a> Resolver<'a> {
    // -- nested sets -------------------------------------------------------

    fn series_set(&self, r: &SeriesRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            SeriesRef::Id(id) => BTreeSet::from([id.clone()]),
            SeriesRef::Select(s) => {
                let o = self.series_rows(s, path)?;
                nested(Kind::Series, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn fact_set(&self, r: &FactRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            FactRef::Id(id) => BTreeSet::from([id.clone()]),
            FactRef::Select(s) => {
                let o = self.fact_rows(s, path)?;
                nested(Kind::Fact, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn finding_set(&self, r: &FindingRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            FindingRef::Id(id) => BTreeSet::from([id.clone()]),
            FindingRef::Select(s) => {
                let o = self.finding_rows(s, path)?;
                nested(Kind::Finding, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn panel_set(&self, view: &View, r: &PanelRef, path: &str) -> PResult<BTreeSet<u32>> {
        Ok(match r {
            PanelRef::Id(id) => BTreeSet::from([*id]),
            PanelRef::Select(s) => {
                let o = self.panel_rows(Some(view), s, path)?;
                nested(Kind::Panel, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .filter_map(|r| r.value.as_u64().and_then(|x| u32::try_from(x).ok()))
                    .collect()
            }
        })
    }

    // -- rows per kind ----------------------------------------------------

    fn series_rows(&self, sel: &SeriesSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let mut o = Outcome::new();
        let keys = &mut o.keys;
        if sel.kind.is_some() {
            keys.push("kind");
        }
        if sel.study.is_some() {
            keys.push("study");
        }
        if !sel.params.is_empty() {
            keys.push("params");
        }
        if on.is_some() {
            keys.push("on");
        }
        if sel.symbol.is_some() {
            keys.push("symbol");
        }
        if sel.label.is_some() {
            keys.push("label");
        }
        for s in &doc.series {
            let (study, params, inputs) = match &s.source {
                SeriesSource::Study {
                    study,
                    params,
                    inputs,
                    ..
                } => (Some(study.as_str()), Some(params), inputs.as_slice()),
                _ => (None, None, &[][..]),
            };
            let mut c = Vec::new();
            if let Some(k) = sel.kind {
                c.push(series_kind(s) == k);
            }
            if let Some(st) = &sel.study {
                c.push(study.is_some_and(|x| eq_text(x, st)));
            }
            if !sel.params.is_empty() {
                c.push(params.is_some_and(|p| {
                    sel.params
                        .iter()
                        .all(|(k, v)| p.get(k).is_some_and(|x| x == v))
                }));
            }
            if let Some(set) = &on {
                c.push(inputs.iter().any(|i| set.contains(i)));
            }
            if let Some(sym) = &sel.symbol {
                c.push(s.symbol().is_some_and(|x| eq_text(x, sym)));
            }
            if let Some(l) = &sel.label {
                c.push(eq_text(&series_label(s), l));
            }
            o.rows.push(Row {
                id: s.id.clone(),
                about: series_about(s),
                value: Value::String(s.id.clone()),
                checks: c,
            });
        }
        if let Some(st) = &sel.study {
            let v = doc
                .series
                .iter()
                .filter_map(|s| match &s.source {
                    SeriesSource::Study { study, .. } => Some(study.clone()),
                    _ => None,
                })
                .collect();
            o.vocab.push(("study", st.clone(), v));
        }
        if let Some(sym) = &sel.symbol {
            let v = doc
                .series
                .iter()
                .filter_map(|s| s.symbol().map(str::to_owned))
                .collect();
            o.vocab.push(("symbol", sym.clone(), v));
        }
        if let Some(l) = &sel.label {
            let v = doc.series.iter().map(series_label).collect();
            o.vocab.push(("label", l.clone(), v));
        }
        Ok(o)
    }

    fn view_rows(&self, sel: &ViewSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let shows = match &sel.shows {
            Some(r) => Some(self.series_set(r, &ptr(path, "shows"))?),
            None => None,
        };
        let mut o = Outcome::new();
        if sel.title.is_some() {
            o.keys.push("title");
        }
        if shows.is_some() {
            o.keys.push("shows");
        }
        for v in &doc.views {
            let title = view_title(doc, v);
            let mut c = Vec::new();
            if let Some(t) = &sel.title {
                c.push(title.as_deref().is_some_and(|x| eq_text(x, t)));
            }
            if let Some(set) = &shows {
                c.push(
                    v.layers
                        .iter()
                        .any(|l| l.series.as_ref().is_some_and(|s| set.contains(s))),
                );
            }
            o.rows.push(Row {
                id: v.id.clone(),
                about: format!(
                    "{}: {} panels, {} layers",
                    title.unwrap_or_else(|| v.id.clone()),
                    v.panels.len(),
                    v.layers.len()
                ),
                value: Value::String(v.id.clone()),
                checks: c,
            });
        }
        if let Some(t) = &sel.title {
            let vals = doc
                .views
                .iter()
                .filter_map(|v| view_title(doc, v))
                .collect();
            o.vocab.push(("title", t.clone(), vals));
        }
        Ok(o)
    }

    fn scope<'v>(&self, view: Option<&'v View>) -> Vec<&'v View>
    where
        'a: 'v,
    {
        match view {
            Some(v) => vec![v],
            None => self.doc.views.iter().collect(),
        }
    }

    fn panel_rows(&self, view: Option<&View>, sel: &PanelSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let shows = match &sel.shows {
            Some(r) => Some(self.series_set(r, &ptr(path, "shows"))?),
            None => None,
        };
        let mut o = Outcome::new();
        if sel.name.is_some() {
            o.keys.push("name");
        }
        if shows.is_some() {
            o.keys.push("shows");
        }
        if sel.mark.is_some() {
            o.keys.push("mark");
        }
        let mut names = Vec::new();
        for v in self.scope(view) {
            for p in &v.panels {
                let Some(id) = p.id else { continue };
                if let Some(n) = &p.name {
                    names.push(n.clone());
                }
                let layers: Vec<&qp_schema::Layer> =
                    v.layers.iter().filter(|l| l.panel == id).collect();
                let mut c = Vec::new();
                if let Some(n) = &sel.name {
                    c.push(p.name.as_deref().is_some_and(|x| eq_text(x, n)));
                }
                if let Some(set) = &shows {
                    c.push(
                        layers
                            .iter()
                            .any(|l| l.series.as_ref().is_some_and(|s| set.contains(s))),
                    );
                }
                if let Some(m) = sel.mark {
                    c.push(layers.iter().any(|l| l.mark == m));
                }
                let what: Vec<String> = layers.iter().map(|l| layer_about(doc, l)).collect();
                let name = p
                    .name
                    .as_ref()
                    .map_or_else(String::new, |n| format!(" \"{n}\""));
                o.rows.push(Row {
                    id: id.to_string(),
                    about: format!(
                        "panel {id}{name} of view {}: {}",
                        v.id,
                        if what.is_empty() {
                            "empty".to_owned()
                        } else {
                            what.join(", ")
                        }
                    ),
                    value: json!(id),
                    checks: c,
                });
            }
        }
        if let Some(n) = &sel.name {
            o.vocab.push(("name", n.clone(), names));
        }
        Ok(o)
    }

    fn layer_rows(
        &self,
        view: Option<&View>,
        sel: &LayerSel,
        path: &str,
        loose: bool,
    ) -> PResult<Outcome> {
        let doc = self.doc;
        let shows = match &sel.shows {
            Some(r) => Some(self.series_set(r, &ptr(path, "shows"))?),
            None => None,
        };
        let facts = match &sel.fact {
            Some(r) => Some(self.fact_set(r, &ptr(path, "fact"))?),
            None => None,
        };
        let findings = match &sel.finding {
            Some(r) => Some(self.finding_set(r, &ptr(path, "finding"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, on) in [
            ("shows", shows.is_some()),
            ("output", sel.output.is_some()),
            ("mark", sel.mark.is_some()),
            ("panel", sel.panel.is_some()),
            ("value", sel.value.is_some()),
            ("fact", facts.is_some()),
            ("finding", findings.is_some()),
            ("label", sel.label.is_some()),
        ] {
            if on {
                o.keys.push(k);
            }
        }
        let mut outputs = Vec::new();
        let mut labels = Vec::new();
        for v in self.scope(view) {
            let panels = match &sel.panel {
                Some(r) => Some(self.panel_set(v, r, &ptr(path, "panel"))?),
                None => None,
            };
            for l in &v.layers {
                let Some(id) = &l.id else { continue };
                let label = layer_label(doc, l);
                if let Some(t) = &label {
                    labels.push(t.clone());
                }
                if let Data::Series(s) = layer_data(doc, l) {
                    outputs.extend(crate::canon::series_fields(s));
                }
                let mut c = Vec::new();
                if let Some(set) = &shows {
                    c.push(l.series.as_ref().is_some_and(|s| set.contains(s)));
                }
                if let Some(f) = &sel.output {
                    c.push(layer_draws(doc, l, f, loose));
                }
                if let Some(m) = sel.mark {
                    c.push(l.mark == m);
                }
                if let Some(set) = &panels {
                    c.push(set.contains(&l.panel));
                }
                if let Some(x) = sel.value {
                    c.push(layer_constants(l).contains(&x));
                }
                if let Some(set) = &facts {
                    c.push(l.fact.as_ref().is_some_and(|f| set.contains(f)));
                }
                if let Some(set) = &findings {
                    c.push(l.finding.as_ref().is_some_and(|f| set.contains(f)));
                }
                if let Some(t) = &sel.label {
                    c.push(label.as_deref().is_some_and(|x| eq_text(x, t)));
                }
                o.rows.push(Row {
                    id: id.clone(),
                    about: format!(
                        "{} (panel {} of view {})",
                        layer_about(doc, l),
                        l.panel,
                        v.id
                    ),
                    value: Value::String(id.clone()),
                    checks: c,
                });
            }
        }
        if let Some(f) = &sel.output {
            o.vocab.push(("output", f.clone(), outputs));
        }
        if let Some(t) = &sel.label {
            o.vocab.push(("label", t.clone(), labels));
        }
        Ok(o)
    }

    fn fact_rows(&self, sel: &FactSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, g) in [
            ("kind", sel.kind.is_some()),
            ("on", on.is_some()),
            ("bar", sel.bar.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(u64, usize, &Fact)> = doc
            .facts
            .iter()
            .enumerate()
            .map(|(i, f)| (f.bar, i, f))
            .collect();
        order.sort_by_key(|(b, i, _)| (*b, *i));
        for (_, _, f) in order {
            let mut c = Vec::new();
            if let Some(k) = &sel.kind {
                c.push(eq_text(&f.kind, k));
            }
            if let Some(set) = &on {
                c.push(set.contains(&f.series));
            }
            if let Some(b) = sel.bar {
                c.push(f.bar == b);
            }
            o.rows.push(Row {
                id: f.id.clone(),
                about: fact_about(f),
                value: Value::String(f.id.clone()),
                checks: c,
            });
        }
        if let Some(k) = &sel.kind {
            let v = doc.facts.iter().map(|f| f.kind.clone()).collect();
            o.vocab.push(("kind", k.clone(), v));
        }
        Ok(o)
    }

    fn finding_rows(&self, sel: &FindingSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let facts = match &sel.fact {
            Some(r) => Some(self.fact_set(r, &ptr(path, "fact"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, g) in [
            ("kind", sel.kind.is_some()),
            ("on", on.is_some()),
            ("fact", facts.is_some()),
            ("state", sel.state.is_some()),
            ("instrument", sel.instrument.is_some()),
            ("meaning", sel.meaning.is_some()),
            ("stage", sel.stage.is_some()),
            ("open", sel.open.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(Option<u64>, usize, &Finding)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| (finding_bar(doc, f), i, f))
            .collect();
        order.sort_by_key(|(b, i, _)| (*b, *i));
        for (_, _, f) in order {
            let mut c = Vec::new();
            if let Some(k) = &sel.kind {
                c.push(finding_facts(doc, f).any(|x| eq_text(&x.kind, k)));
            }
            if let Some(set) = &on {
                c.push(finding_facts(doc, f).any(|x| set.contains(&x.series)));
            }
            if let Some(set) = &facts {
                c.push(f.facts.iter().any(|x| set.contains(x)));
            }
            if let Some(st) = sel.state {
                c.push(
                    doc.capsules
                        .iter()
                        .find(|cap| cap.finding == f.id)
                        .is_some_and(|cap| cap.state == st),
                );
            }
            // (R4) Keys about the finding's capsule story.
            let story = doc
                .capsules
                .iter()
                .find(|cap| cap.finding == f.id)
                .and_then(|cap| cap.story.as_ref());
            if let Some(i) = &sel.instrument {
                c.push(story.is_some_and(|s| eq_text(&s.instrument, i)));
            }
            if let Some(m) = sel.meaning {
                c.push(story.is_some_and(|s| s.kind == m));
            }
            if let Some(st) = sel.stage {
                c.push(story.is_some_and(|s| s.stage == st));
            }
            if let Some(op) = sel.open {
                c.push(story.is_some_and(|s| s.stage.is_closed() != op));
            }
            o.rows.push(Row {
                id: f.id.clone(),
                about: finding_about(doc, f),
                value: Value::String(f.id.clone()),
                checks: c,
            });
        }
        if let Some(k) = &sel.kind {
            let v = doc.facts.iter().map(|f| f.kind.clone()).collect();
            o.vocab.push(("kind", k.clone(), v));
        }
        Ok(o)
    }

    /// (R4) Capsules with a story, in the latest transition's bar order, then birth bar,
    /// then document order.
    fn capsule_rows(&self, sel: &CapsuleSel) -> Outcome {
        let doc = self.doc;
        let mut o = Outcome::new();
        for (k, g) in [
            ("instrument", sel.instrument.is_some()),
            ("meaning", sel.meaning.is_some()),
            ("direction", sel.direction.is_some()),
            ("stage", sel.stage.is_some()),
            ("open", sel.open.is_some()),
            ("born_bar", sel.born_bar.is_some()),
            ("series", sel.series.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(
            u64,
            u64,
            usize,
            &qp_schema::Capsule,
            &qp_schema::capsule::Story,
        )> = doc
            .capsules
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.story.as_ref().map(|s| (s.last_bar, s.born_bar, i, c, s)))
            .collect();
        order.sort_by_key(|(l, b, i, _, _)| (*l, *b, *i));
        for (_, _, _, c, st) in order {
            let mut ch = Vec::new();
            if let Some(i) = &sel.instrument {
                ch.push(eq_text(&st.instrument, i));
            }
            if let Some(m) = sel.meaning {
                ch.push(st.kind == m);
            }
            if let Some(d) = sel.direction {
                ch.push(st.direction == d);
            }
            if let Some(x) = sel.stage {
                ch.push(st.stage == x);
            }
            if let Some(op) = sel.open {
                ch.push(st.stage.is_closed() != op);
            }
            if let Some(b) = sel.born_bar {
                ch.push(st.born_bar == b);
            }
            if let Some(sr) = &sel.series {
                ch.push(st.series == *sr);
            }
            o.rows.push(Row {
                id: c.id.clone(),
                about: format!(
                    "{} {} {} ({}), born bar {}, last bar {}",
                    st.instrument,
                    st.kind.name(),
                    qp_capsule::engine::direction_name(st.direction),
                    serde_json::to_value(st.stage)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default(),
                    st.born_bar,
                    st.last_bar
                ),
                value: Value::String(c.id.clone()),
                checks: ch,
            });
        }
        if let Some(i) = &sel.instrument {
            let v = doc
                .capsules
                .iter()
                .filter_map(|c| c.story.as_ref().map(|s| s.instrument.clone()))
                .collect();
            o.vocab.push(("instrument", i.clone(), v));
        }
        o
    }

    fn proposal_rows(&self, sel: &ProposalSel) -> Outcome {
        let mut o = Outcome::new();
        if sel.state.is_some() {
            o.keys.push("state");
        }
        for p in &self.doc.proposals {
            let mut c = Vec::new();
            if let Some(s) = sel.state {
                c.push(p.state == s);
            }
            let state = serde_json::to_value(p.state)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            o.rows.push(Row {
                id: p.id.clone(),
                about: format!("{state}: {}", short(&summarize_ops(&p.ops, &[]), 80)),
                value: Value::String(p.id.clone()),
                checks: c,
            });
        }
        o
    }

    /// When `output` matched no layer but a multi-output layer draws it among others:
    /// `invalid`, naming that layer and the ready sequence (split, then style).
    fn split_first(&self, view: Option<&View>, s: &LayerSel, path: &str) -> Option<ProtocolError> {
        let o = self.layer_rows(view, s, path, true).ok()?;
        let m = pick(o.rows, s.first, s.latest, path).ok()?;
        let first = m.first()?;
        let out = s.output.clone().unwrap_or_default();
        let vid = view.map(|v| v.id.clone()).or_else(|| {
            self.doc
                .views
                .iter()
                .find(|v| v.layer(&first.id).is_some())
                .map(|v| v.id.clone())
        })?;
        let q = |x: &str| Value::String(x.to_owned()).to_string();
        let seq = format!(
            "[{{\"op\":\"split_layer\",\"view\":{v},\"layer\":{l}}},{{\"op\":\"set_encoding\",\"view\":{v},\"layer\":{{\"output\":{o}}},\"encoding\":{{…}}}}]",
            v = q(&vid),
            l = q(&first.id),
            o = q(&out)
        );
        Some(
            E::new(
                ErrorCode::Invalid,
                format!(
                    "no layer draws `{out}` by itself; layer `{}` draws it together with other outputs, so styling it would change them all",
                    first.id
                ),
            )
            .at(path)
            .hint(format!(
                "Split that layer first, in the same command: {seq}"
            ))
            .build_with(candidates(&m, MAX_AMBIGUOUS)),
        )
    }

    // -- entry points -----------------------------------------------------

    /// Checks a selector against its schema and parses it.
    fn parse<T: serde::de::DeserializeOwned>(
        &self,
        def: &str,
        v: &Value,
        path: &str,
    ) -> PResult<T> {
        if let Some(root) = self.root {
            let schema = json!({ "$ref": format!("#/$defs/{def}") });
            if let Some(viol) = locate(v, &schema, root) {
                return Err(violation_error(&viol, path));
            }
        }
        serde_json::from_value(v.clone()).map_err(|e| {
            E::new(ErrorCode::Schema, format!("invalid selector: {e}"))
                .at(path)
                .build()
        })
    }

    /// The view a panel/layer/scale site belongs to, if its message names one.
    fn context(&self, msg: &Value, site: &Site) -> PResult<Option<&'a View>> {
        let Some(vp) = &site.view else {
            return Ok(None);
        };
        match msg.pointer(vp).and_then(Value::as_str) {
            Some(id) => Ok(Some(need_view(self.doc, id, vp)?)),
            None => Ok(None),
        }
    }

    /// Resolves the selector at `site` of `msg`: the value to write there and the id.
    pub(crate) fn resolve(&self, msg: &Value, site: &Site) -> PResult<(Value, String)> {
        let sel = msg.pointer(&site.path).cloned().unwrap_or(Value::Null);
        let view = match site.kind {
            Kind::Panel | Kind::Layer | Kind::Scale => self.context(msg, site)?,
            _ => None,
        };
        let row = self.resolve_kind(site.kind, &sel, view, &site.path)?;
        Ok((row.value, row.id))
    }

    fn resolve_kind(
        &self,
        kind: Kind,
        sel: &Value,
        view: Option<&View>,
        path: &str,
    ) -> PResult<Row> {
        match kind {
            Kind::Series => {
                let s: SeriesSel = self.parse(kind.def(), sel, path)?;
                let o = self.series_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::View => {
                let s: ViewSel = self.parse(kind.def(), sel, path)?;
                let o = self.view_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Panel => {
                let s: PanelSel = self.parse(kind.def(), sel, path)?;
                let o = self.panel_rows(view, &s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Layer => {
                let s: LayerSel = self.parse(kind.def(), sel, path)?;
                let o = self.layer_rows(view, &s, path, false)?;
                match decide(kind, path, sel, o, s.first, s.latest) {
                    Err(e) if e.code == ErrorCode::NotFound && s.output.is_some() => {
                        Err(self.split_first(view, &s, path).unwrap_or(e))
                    }
                    r => r,
                }
            }
            Kind::Fact => {
                let s: FactSel = self.parse(kind.def(), sel, path)?;
                let o = self.fact_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Finding => {
                let s: FindingSel = self.parse(kind.def(), sel, path)?;
                let o = self.finding_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Capsule => {
                let s: CapsuleSel = self.parse(kind.def(), sel, path)?;
                let o = self.capsule_rows(&s);
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Proposal => {
                let s: ProposalSel = self.parse(kind.def(), sel, path)?;
                let o = self.proposal_rows(&s);
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Scale => {
                let s: ScaleSel = self.parse(kind.def(), sel, path)?;
                let pp = ptr(path, "panel");
                let id = match &s.panel {
                    PanelRef::Id(n) => *n,
                    PanelRef::Select(ps) => {
                        let o = self.panel_rows(view, ps, &pp)?;
                        let sel_v = sel.get("panel").cloned().unwrap_or(Value::Null);
                        let r = decide(Kind::Panel, &pp, &sel_v, o, ps.first, ps.latest)?;
                        r.value
                            .as_u64()
                            .and_then(|x| u32::try_from(x).ok())
                            .ok_or_else(|| invalid("panel id out of range", pp.clone()))?
                    }
                };
                let id = format!("y{id}");
                Ok(Row {
                    about: format!("value scale of panel {}", &id[1..]),
                    value: Value::String(id.clone()),
                    id,
                    checks: Vec::new(),
                })
            }
            Kind::Item => {
                const KINDS: [&str; 5] = ["series", "view", "fact", "finding", "proposal"];
                let obj = sel.as_object().filter(|o| o.len() == 1);
                let Some((k, inner)) = obj.and_then(|o| o.iter().next()) else {
                    return Err(E::new(
                        ErrorCode::Schema,
                        "an item selector names its kind with one key, e.g. {\"finding\":{\"kind\":\"breakout\",\"latest\":true}}",
                    )
                    .at(path)
                    .expected(KINDS)
                    .build());
                };
                let ip = ptr(path, k);
                let sub = match k.as_str() {
                    "series" => Kind::Series,
                    "view" => Kind::View,
                    "fact" => Kind::Fact,
                    "finding" => Kind::Finding,
                    "proposal" => Kind::Proposal,
                    _ => {
                        return Err(E::new(ErrorCode::Schema, format!("unknown field `{k}`"))
                            .at(ip)
                            .dym(k, KINDS)
                            .expected(KINDS)
                            .hint("An item selector names its kind: series, view, fact, finding or proposal.")
                            .build())
                    }
                };
                self.resolve_kind(sub, inner, None, &ip)
            }
        }
    }

    // -- hints (welcome.summary, screen) ----------------------------------

    fn ids<F>(&self, f: F) -> Vec<String>
    where
        F: FnOnce() -> PResult<(Outcome, bool, bool)>,
    {
        match f() {
            Ok((o, first, latest)) => pick(o.rows, first, latest, "")
                .map(|v| v.into_iter().map(|r| r.id).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn series_ids(&self, s: &SeriesSel) -> Vec<String> {
        self.ids(|| Ok((self.series_rows(s, "")?, s.first, s.latest)))
    }

    fn view_ids(&self, s: &ViewSel) -> Vec<String> {
        self.ids(|| Ok((self.view_rows(s, "")?, s.first, s.latest)))
    }

    fn panel_ids(&self, v: &View, s: &PanelSel) -> Vec<String> {
        self.ids(|| Ok((self.panel_rows(Some(v), s, "")?, s.first, s.latest)))
    }

    fn layer_ids(&self, v: &View, s: &LayerSel) -> Vec<String> {
        self.ids(|| Ok((self.layer_rows(Some(v), s, "", false)?, s.first, s.latest)))
    }

    fn fact_ids(&self, s: &FactSel) -> Vec<String> {
        self.ids(|| Ok((self.fact_rows(s, "")?, s.first, s.latest)))
    }

    fn finding_ids(&self, s: &FindingSel) -> Vec<String> {
        self.ids(|| Ok((self.finding_rows(s, "")?, s.first, s.latest)))
    }

    /// The first candidate that resolves to exactly `target`; then the first one whose
    /// matches start (`first`) or end (`latest`) with it.
    fn choose<S: Clone>(
        cands: Vec<S>,
        target: &str,
        ids: impl Fn(&S) -> Vec<String>,
        set_first: impl Fn(&mut S),
        set_latest: impl Fn(&mut S),
    ) -> Option<S> {
        let found: Vec<Vec<String>> = cands.iter().map(&ids).collect();
        for (c, m) in cands.iter().zip(&found) {
            if m.len() == 1 && m[0] == target {
                return Some(c.clone());
            }
        }
        for (c, m) in cands.iter().zip(&found) {
            if m.first().is_some_and(|x| x == target) {
                let mut c = c.clone();
                set_first(&mut c);
                return Some(c);
            }
            if m.last().is_some_and(|x| x == target) {
                let mut c = c.clone();
                set_latest(&mut c);
                return Some(c);
            }
        }
        None
    }

    /// A selector that resolves to series `s` now.
    pub(crate) fn series_hint(&self, s: &Series) -> Option<SeriesSel> {
        let mut cands = Vec::new();
        match &s.source {
            SeriesSource::Study { study, params, .. } => {
                cands.push(SeriesSel {
                    study: Some(study.clone()),
                    ..SeriesSel::default()
                });
                cands.push(SeriesSel {
                    study: Some(study.clone()),
                    params: params.clone(),
                    ..SeriesSel::default()
                });
            }
            _ => {
                let kind = series_kind(s);
                cands.push(SeriesSel {
                    kind: Some(kind),
                    ..SeriesSel::default()
                });
                if let Some(sym) = s.symbol() {
                    cands.push(SeriesSel {
                        kind: Some(kind),
                        symbol: Some(sym.to_owned()),
                        ..SeriesSel::default()
                    });
                }
            }
        }
        cands.push(SeriesSel {
            label: Some(series_label(s)),
            ..SeriesSel::default()
        });
        Self::choose(
            cands,
            &s.id,
            |c| self.series_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    fn series_ref_hint(&self, id: &str) -> Option<SeriesRef> {
        let s = self.doc.series.iter().find(|s| s.id == id)?;
        self.series_hint(s).map(SeriesRef::Select)
    }

    /// A selector that resolves to view `v` now.
    pub(crate) fn view_hint(&self, v: &View) -> Option<ViewSel> {
        let mut cands = vec![ViewSel::default()];
        if let Some(t) = view_title(self.doc, v) {
            cands.push(ViewSel {
                title: Some(t),
                ..ViewSel::default()
            });
        }
        for l in &v.layers {
            if let Some(r) = l.series.as_deref().and_then(|s| self.series_ref_hint(s)) {
                cands.push(ViewSel {
                    shows: Some(Box::new(r)),
                    ..ViewSel::default()
                });
            }
        }
        Self::choose(
            cands,
            &v.id,
            |c| self.view_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to panel `id` of view `v` now.
    pub(crate) fn panel_hint(&self, v: &View, id: u32) -> Option<PanelSel> {
        let p = v.panel(id)?;
        let layers: Vec<&qp_schema::Layer> = v.layers.iter().filter(|l| l.panel == id).collect();
        let mut cands = Vec::new();
        if let Some(n) = &p.name {
            cands.push(PanelSel {
                name: Some(n.clone()),
                ..PanelSel::default()
            });
        }
        if layers.iter().any(|l| l.mark == Mark::Candle) {
            cands.push(PanelSel {
                mark: Some(Mark::Candle),
                ..PanelSel::default()
            });
        }
        for l in &layers {
            if let Some(r) = l.series.as_deref().and_then(|s| self.series_ref_hint(s)) {
                cands.push(PanelSel {
                    shows: Some(Box::new(r)),
                    ..PanelSel::default()
                });
            }
        }
        for l in &layers {
            cands.push(PanelSel {
                mark: Some(l.mark),
                ..PanelSel::default()
            });
        }
        Self::choose(
            cands,
            &id.to_string(),
            |c| self.panel_ids(v, c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to layer `l` of view `v` now.
    pub(crate) fn layer_hint(&self, v: &View, l: &qp_schema::Layer) -> Option<LayerSel> {
        let id = l.id.as_deref()?;
        let mut cands = Vec::new();
        let shows = l.series.as_deref().and_then(|s| self.series_ref_hint(s));
        if let Some(r) = &shows {
            let base = LayerSel {
                shows: Some(Box::new(r.clone())),
                ..LayerSel::default()
            };
            cands.push(base.clone());
            cands.push(LayerSel {
                mark: Some(l.mark),
                ..base.clone()
            });
            if let Some(Position::Name(n)) = &l.encoding.y {
                if crate::canon::numeric(n).is_none() {
                    cands.push(LayerSel {
                        output: Some(n.clone()),
                        ..base.clone()
                    });
                }
            }
        }
        for c in layer_constants(l) {
            cands.push(LayerSel {
                mark: Some(l.mark),
                value: Some(c),
                ..LayerSel::default()
            });
        }
        if let Some(f) = l.fact.as_deref() {
            if let Some(h) = self
                .doc
                .facts
                .iter()
                .find(|x| x.id == f)
                .and_then(|x| self.fact_hint(x))
            {
                cands.push(LayerSel {
                    fact: Some(Box::new(FactRef::Select(h))),
                    ..LayerSel::default()
                });
            }
        }
        if let Some(f) = l.finding.as_deref() {
            if let Some(h) = self
                .doc
                .findings
                .iter()
                .find(|x| x.id == f)
                .and_then(|x| self.finding_hint(x))
            {
                cands.push(LayerSel {
                    finding: Some(Box::new(FindingRef::Select(h))),
                    ..LayerSel::default()
                });
            }
        }
        if let Some(t) = &l.label {
            cands.push(LayerSel {
                label: Some(t.clone()),
                ..LayerSel::default()
            });
        }
        cands.push(LayerSel {
            mark: Some(l.mark),
            ..LayerSel::default()
        });
        Self::choose(
            cands,
            id,
            |c| self.layer_ids(v, c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to fact `f` now.
    pub(crate) fn fact_hint(&self, f: &Fact) -> Option<FactSel> {
        let kind = FactSel {
            kind: Some(f.kind.clone()),
            ..FactSel::default()
        };
        let cands = vec![
            kind.clone(),
            FactSel {
                latest: true,
                ..kind.clone()
            },
            FactSel {
                first: true,
                ..kind.clone()
            },
            FactSel {
                bar: Some(f.bar),
                ..kind.clone()
            },
        ];
        Self::choose(
            cands,
            &f.id,
            |c| self.fact_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to finding `f` now.
    pub(crate) fn finding_hint(&self, f: &Finding) -> Option<FindingSel> {
        let doc = self.doc;
        let latest = finding_facts(doc, f).max_by_key(|x| x.bar)?;
        let kind = FindingSel {
            kind: Some(latest.kind.clone()),
            ..FindingSel::default()
        };
        let mut cands = vec![
            kind.clone(),
            FindingSel {
                latest: true,
                ..kind.clone()
            },
            FindingSel {
                first: true,
                ..kind.clone()
            },
        ];
        if let Some(h) = self.fact_hint(latest) {
            cands.push(FindingSel {
                fact: Some(Box::new(FactRef::Select(h))),
                ..FindingSel::default()
            });
        }
        Self::choose(
            cands,
            &f.id,
            |c| self.finding_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }
}

/// Resolves every selector of `msg` in place against `doc` (`groups` from [`sites`]),
/// recording `{pointer -> id}` in `out`.
pub(crate) fn resolve_group(
    r: &Resolver<'_>,
    msg: &mut Value,
    group: &[Site],
    out: &mut BTreeMap<String, String>,
) -> PResult<()> {
    for site in group {
        if !is_selector(msg, site) {
            continue;
        }
        let (value, id) = r.resolve(msg, site)?;
        if let Some(slot) = msg.pointer_mut(&site.path) {
            *slot = value;
        }
        out.insert(site.path.clone(), id);
    }
    Ok(())
}
