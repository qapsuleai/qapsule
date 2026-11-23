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

mod resolve;
mod rows;
mod sites;

pub use sites::*;

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{Mark, Position, View};
use qp_schema::protocol::{
    Candidate, CapsuleSel, ErrorCode, FactRef, FactSel, FindingRef, FindingSel, LayerSel, PanelRef,
    PanelSel, ProposalSel, ProtocolError, ScaleSel, SeriesKind, SeriesRef, SeriesSel, ViewSel,
};
use qp_schema::{Document, Fact, Finding, Series, SeriesData, SeriesSource};
use serde_json::{json, Value};

use crate::canon::{layer_data, position_constant, series_main, Data};
use crate::expand::{need_view, summarize_ops, view_bar_series};
use crate::locate::{locate, Violation, ViolationKind};
use crate::summary::{drawing_outputs, layer_about, series_label};
use qp_plane::error::{invalid, num, ptr, PResult, E};

/// At most this many candidates in an `ambiguous` error.
const MAX_AMBIGUOUS: usize = 10;
/// At most this many candidates in a `not_found` error.
const MAX_NOT_FOUND: usize = 5;

/// What kind of item a field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
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
pub struct Site {
    /// JSON pointer of the field.
    pub path: String,
    pub kind: Kind,
    /// Pointer of the field that names the view a panel/layer/scale belongs to.
    pub view: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A schema violation as a protocol error, its path prefixed with `prefix`.
pub fn violation_error(v: &Violation, prefix: &str) -> ProtocolError {
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
            dym.extend(qp_plane::suggest::did_you_mean(
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

pub fn finding_bar(doc: &Document, f: &Finding) -> Option<u64> {
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

pub fn finding_about(doc: &Document, f: &Finding) -> String {
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
pub fn view_title(doc: &Document, v: &View) -> Option<String> {
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
pub struct Resolver<'a> {
    pub doc: &'a Document,
    /// The generated schema whose `$defs` hold the selector types (to check them).
    pub root: Option<&'a Value>,
}

impl<'a> Resolver<'a> {
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
}

/// Resolves every selector of `msg` in place against `doc` (`groups` from [`sites`]),
/// recording `{pointer -> id}` in `out`.
pub fn resolve_group(
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
