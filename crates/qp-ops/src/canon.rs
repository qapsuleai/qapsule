//! The visual grammar v0 in the authority: canonical form of views, validation of
//! layers and panels, and origins of numeric constants (`docs/GRAMMAR.md` §2–§11).
//!
//! Kept apart from op expansion (`gops.rs`) so that these rules can later move into
//! `qp-grammar` without touching the protocol code. Every function here is pure: the same
//! document and view give the same result (no clock, no randomness).

mod layer;
mod origin;
mod scale;

pub(crate) use layer::*;
pub use origin::*;
pub(crate) use scale::*;

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{
    ColorSpec, DataRef, Emphasis, Layer, Mark, Operand, Origin, Panel, Position, Scale, ScaleKind,
    SizeSpec, View,
};
use qp_schema::protocol::{Candidate, ErrorCode, Target};
use qp_schema::{Document, Fact, Finding, Series, SeriesData, SeriesSource};

use crate::apply::{all_ids, find_collection};
use crate::expand::{need_series, valid_id};
use qp_plane::error::{invalid, num, ptr, PResult, E};
use qp_plane::values::{main_output, study_outputs, values, Cache};

/// Named thresholds v0 (`docs/GRAMMAR.md` §10), sorted by name.
pub(crate) const THRESHOLDS: [(&str, f64); 4] = [
    ("macd.zero", 0.0),
    ("rsi.mid", 50.0),
    ("rsi.overbought", 70.0),
    ("rsi.oversold", 30.0),
];

/// At most this many panels per view.
pub(crate) const MAX_PANELS: usize = 16;
/// At most this many layers per view.
pub(crate) const MAX_LAYERS: usize = 64;

/// Theme colour tokens (`docs/GRAMMAR.md` §9).
pub(crate) const THEME_TOKENS: [&str; 17] = [
    "bg", "grid", "axis", "text", "muted", "up", "down", "accent", "warn", "s1", "s2", "s3", "s4",
    "s5", "s6", "s7", "s8",
];

/// Channel names of [`qp_schema::grammar::Encoding`].
pub(crate) const CHANNELS: [&str; 9] = [
    "x", "x2", "y", "y2", "color", "opacity", "width", "size", "text",
];

/// Theme names.
const THEMES: [&str; 2] = ["dark", "light"];

/// The value of a named threshold.
pub(crate) fn threshold(name: &str) -> Option<f64> {
    THRESHOLDS.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

/// The named thresholds as a map (for `describe`).
#[must_use]
pub fn thresholds() -> BTreeMap<String, f64> {
    THRESHOLDS
        .iter()
        .map(|(n, v)| ((*n).to_owned(), *v))
        .collect()
}

/// A string channel value that is a numeric constant (`"70"`, the `qp/0` form).
pub(crate) fn numeric(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

/// The constant written in a position channel, if it is one.
pub(crate) fn position_constant(p: &Position) -> Option<f64> {
    match p {
        Position::Number(v) => Some(*v),
        Position::Name(s) => numeric(s),
        Position::Ref(_) | Position::Panel(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Data of a layer and its fields
// ---------------------------------------------------------------------------

/// What a layer draws.
#[derive(Clone, Copy)]
pub(crate) enum Data<'a> {
    None,
    Series(&'a Series),
    Fact(&'a Fact),
    Finding(&'a Finding),
}

/// Fields of a series: `o h l c v` for bars, the outputs of a study, `v` for points.
pub(crate) fn series_fields(s: &Series) -> Vec<String> {
    match (&s.source, &s.data) {
        // (RL1, PS §1.1) A program output series has its one output as its field.
        (
            SeriesSource::Study {
                program: Some(_),
                output: Some(o),
                ..
            },
            _,
        ) => vec![o.clone()],
        (SeriesSource::Study { study, .. }, _) => study_outputs(study),
        (_, Some(SeriesData::Points { .. })) => vec!["v".to_owned()],
        _ => ["o", "h", "l", "c", "v"].map(str::to_owned).to_vec(),
    }
}

/// Main field of a series.
pub(crate) fn series_main(s: &Series) -> String {
    match (&s.source, &s.data) {
        (SeriesSource::Study { study, output, .. }, _) => main_output(study, output.as_ref()),
        (_, Some(SeriesData::Points { .. })) => "v".to_owned(),
        _ => "c".to_owned(),
    }
}

fn fact_fields(f: &Fact) -> Vec<String> {
    let mut v: Vec<String> = ["bar", "t", "value", "text"].map(str::to_owned).to_vec();
    for k in f.evidence.keys() {
        if !v.contains(k) {
            v.push(k.clone());
        }
    }
    v
}

pub(crate) fn data_fields(doc: &Document, d: Data<'_>) -> Vec<String> {
    match d {
        Data::None => Vec::new(),
        Data::Series(s) => series_fields(s),
        Data::Fact(f) => fact_fields(f),
        Data::Finding(fnd) => {
            let mut v: Vec<String> = Vec::new();
            for fid in &fnd.facts {
                if let Some(f) = doc.facts.iter().find(|x| &x.id == fid) {
                    for k in fact_fields(f) {
                        if !v.contains(&k) {
                            v.push(k);
                        }
                    }
                }
            }
            for k in ["claim", "confidence"] {
                if !v.iter().any(|x| x == k) {
                    v.push(k.to_owned());
                }
            }
            v
        }
    }
}

/// Main field of a layer's data (`docs/GRAMMAR.md` §6).
pub(crate) fn data_main(doc: &Document, d: Data<'_>) -> Option<String> {
    let fact_main = |f: &Fact| {
        if f.evidence.contains_key("level") {
            "level".to_owned()
        } else {
            "value".to_owned()
        }
    };
    match d {
        Data::None => None,
        Data::Series(s) => Some(series_main(s)),
        Data::Fact(f) => Some(fact_main(f)),
        Data::Finding(fnd) => fnd
            .facts
            .first()
            .and_then(|fid| doc.facts.iter().find(|x| &x.id == fid))
            .map(fact_main),
    }
}

/// The data a layer refers to (already checked to exist).
pub(crate) fn layer_data<'a>(doc: &'a Document, l: &Layer) -> Data<'a> {
    if let Some(s) = &l.series {
        if let Some(x) = doc.series.iter().find(|x| &x.id == s) {
            return Data::Series(x);
        }
    }
    if let Some(f) = &l.fact {
        if let Some(x) = doc.facts.iter().find(|x| &x.id == f) {
            return Data::Fact(x);
        }
    }
    if let Some(f) = &l.finding {
        if let Some(x) = doc.findings.iter().find(|x| &x.id == f) {
            return Data::Finding(x);
        }
    }
    Data::None
}

fn need_fact<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Fact> {
    doc.facts.iter().find(|f| f.id == id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("fact `{id}` does not exist"))
            .at(path)
            .dym(id, doc.facts.iter().map(|f| f.id.as_str()))
            .hint("List facts with {\"kind\":\"list\",\"collection\":\"facts\"}.")
            .build()
    })
}

fn need_finding<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Finding> {
    crate::expand::need_finding(doc, id, path)
}

// ---------------------------------------------------------------------------
// Panels and layers of a view
// ---------------------------------------------------------------------------

fn panel_names(view: &View) -> Vec<String> {
    let mut v: Vec<String> = view
        .panels
        .iter()
        .filter_map(|p| p.id.map(|i| i.to_string()))
        .collect();
    v.extend(view.panels.iter().filter_map(|p| p.name.clone()));
    v
}

/// Position of panel `id` in the stack, or `not_found` naming the panels.
pub(crate) fn need_panel(view: &View, id: u32, path: &str) -> PResult<usize> {
    view.panels
        .iter()
        .position(|p| p.id == Some(id))
        .ok_or_else(|| {
            let names = panel_names(view);
            E::new(
                ErrorCode::NotFound,
                format!("panel {id} does not exist in view `{}`", view.id),
            )
            .at(path)
            .dym(&id.to_string(), names.iter().map(String::as_str))
            .expected(
                view.panels
                    .iter()
                    .filter_map(|p| p.id)
                    .map(|i| match view.panel(i).and_then(|p| p.name.as_deref()) {
                        Some(n) => format!("{i} ({n})"),
                        None => i.to_string(),
                    }),
            )
            .hint("Use an existing panel id (welcome.summary lists them), or add one with insert_panel.")
            .build()
        })
}

/// Position of layer `id` in the paint order, or `not_found` naming the layers.
pub(crate) fn need_layer(view: &View, id: &str, path: &str) -> PResult<usize> {
    view.layers
        .iter()
        .position(|l| l.id.as_deref() == Some(id))
        .ok_or_else(|| {
            E::new(
                ErrorCode::NotFound,
                format!("layer `{id}` does not exist in view `{}`", view.id),
            )
            .at(path)
            .dym(id, view.layers.iter().filter_map(|l| l.id.as_deref()))
            .expected(view.layers.iter().filter_map(|l| l.id.clone()))
            .hint("Use an existing layer id (welcome.summary lists them).")
            .build()
        })
}

/// The smallest `l<k>` not in `taken`.
pub(crate) fn next_layer_id(taken: &BTreeSet<String>) -> String {
    (0u32..)
        .map(|k| format!("l{k}"))
        .find(|c| !taken.contains(c))
        .unwrap_or_else(|| "l0".to_owned())
}

pub(crate) fn layer_ids(view: &View) -> BTreeSet<String> {
    view.layers.iter().filter_map(|l| l.id.clone()).collect()
}

/// Checks a given layer id: valid and not used by another layer of the view.
pub(crate) fn check_new_layer_id(view: &View, id: &str, path: &str) -> PResult<()> {
    if !valid_id(id) {
        return Err(
            E::new(ErrorCode::Invalid, format!("invalid layer id `{id}`"))
                .at(path)
                .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
                .build(),
        );
    }
    if view.layers.iter().any(|l| l.id.as_deref() == Some(id)) {
        return Err(E::new(
            ErrorCode::Duplicate,
            format!("layer `{id}` already exists in view `{}`", view.id),
        )
        .at(path)
        .hint("Choose another id or omit it to have one assigned.")
        .build());
    }
    Ok(())
}

pub(crate) fn limit_layers(view: &View, adding: usize, path: &str) -> PResult<()> {
    if view.layers.len() + adding > MAX_LAYERS {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a view holds at most {MAX_LAYERS} layers"),
        )
        .at(path)
        .expected([MAX_LAYERS.to_string()])
        .hint("Delete a layer first.")
        .build());
    }
    Ok(())
}

pub(crate) fn limit_panels(view: &View, adding: usize, path: &str) -> PResult<()> {
    if view.panels.len() + adding > MAX_PANELS {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a view holds at most {MAX_PANELS} panels"),
        )
        .at(path)
        .expected([MAX_PANELS.to_string()])
        .hint("Delete a panel first.")
        .build());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

/// Where the layer being checked sits: its view and the JSON pointer of the layer in
/// the incoming message (pointers inside the layer, `/encoding/y`, are appended).
pub struct Site<'a> {
    pub view: &'a View,
    pub base: &'a str,
    /// Bound data for origins, when a command writes (absent on load).
    pub(crate) reads: Option<Reads<'a>>,
}

/// What a writing command may read for an origin (§18.3): the `value` of a fact on a
/// bound series comes from the reading document, if the writer is entitled to it.
#[derive(Clone, Copy)]
pub(crate) struct Reads<'a> {
    /// The data plane's reader.
    pub reader: qp_plane::plane::Reader<'a>,
    /// The principal who writes.
    pub writer: &'a str,
}

/// The value of an origin, as far as the authority can tell.
enum OriginValue {
    /// A live `series` origin: no constant.
    Live,
    /// The origin's value.
    Value(f64),
    /// The `value` of a fact on a bound series, with no data at hand (loading a
    /// document): the written constant is kept.
    Unknown,
}

impl Site<'_> {
    fn at(&self, pointer: &str) -> String {
        format!("{}{pointer}", self.base)
    }
}

// ---------------------------------------------------------------------------
// Views
// ---------------------------------------------------------------------------

/// Brings a view to canonical form and validates it (`docs/GRAMMAR.md` §11): `y` →
/// `panels`, panel and scale ids, layer ids, origins of constants, references. `base` is
/// the JSON pointer of the view in the incoming message (or document).
#[allow(clippy::too_many_lines)]
pub(crate) fn canon_view(
    doc: &Document,
    view: &View,
    base: &str,
    reads: Option<Reads<'_>>,
) -> PResult<View> {
    let mut v = view.clone();
    if let Some(t) = &v.theme {
        if !THEMES.contains(&t.as_str()) {
            return Err(E::new(ErrorCode::Invalid, format!("unknown theme `{t}`"))
                .at(ptr(base, "theme"))
                .dym(t, THEMES)
                .expected(THEMES.map(str::to_owned))
                .build());
        }
    }
    check_x(&mut v.x, &ptr(base, "x"))?;
    // Panels.
    let from_y = !v.y.is_empty();
    if from_y && !v.panels.is_empty() {
        return Err(E::new(
            ErrorCode::Invalid,
            "a view has `panels` or the deprecated `y`, not both",
        )
        .at(ptr(base, "y"))
        .hint("Remove `y`; give each panel its scale in `panels[i].y`.")
        .build());
    }
    if v.y.len() > MAX_PANELS {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a view holds at most {MAX_PANELS} panels"),
        )
        .at(ptr(base, "y"))
        .expected([MAX_PANELS.to_string()])
        .build());
    }
    if from_y {
        v.panels = std::mem::take(&mut v.y)
            .into_iter()
            .enumerate()
            .map(|(i, y)| Panel {
                id: Some(u32::try_from(i).unwrap_or(u32::MAX)),
                name: None,
                weight: None,
                y,
            })
            .collect();
    } else if v.panels.is_empty() {
        let mut ids: BTreeSet<u32> = v.layers.iter().map(|l| l.panel).collect();
        if ids.is_empty() {
            ids.insert(0);
        }
        if ids.len() > MAX_PANELS {
            return Err(E::new(
                ErrorCode::Limit,
                format!("a view holds at most {MAX_PANELS} panels"),
            )
            .at(ptr(base, "layers"))
            .expected([MAX_PANELS.to_string()])
            .build());
        }
        v.panels = ids
            .into_iter()
            .map(|id| Panel {
                id: Some(id),
                name: None,
                weight: None,
                y: Scale::new(ScaleKind::Linear),
            })
            .collect();
    }
    let pbase = if from_y {
        ptr(base, "y")
    } else {
        ptr(base, "panels")
    };
    if v.panels.len() > MAX_PANELS {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a view holds at most {MAX_PANELS} panels"),
        )
        .at(pbase)
        .expected([MAX_PANELS.to_string()])
        .build());
    }
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    for (i, p) in v.panels.iter().enumerate() {
        if let Some(id) = p.id {
            if !seen.insert(id) {
                return Err(E::new(
                    ErrorCode::Duplicate,
                    format!("panel id {id} is used twice in view `{}`", v.id),
                )
                .at(ptr(&ptr(&pbase, i), "id"))
                .build());
            }
        }
    }
    for i in 0..v.panels.len() {
        if v.panels[i].id.is_none() {
            let next = match seen.iter().next_back() {
                None => 0,
                Some(m) => m.checked_add(1).ok_or_else(|| {
                    invalid("no panel id is left above the largest one", ptr(&pbase, i))
                })?,
            };
            seen.insert(next);
            v.panels[i].id = Some(next);
        }
        let pp = ptr(&pbase, i);
        if from_y {
            let id = v.panels[i].id.unwrap_or_default();
            check_value_scale(&mut v.panels[i].y, id, &pp)?;
        } else {
            check_panel(&mut v.panels[i], &pp)?;
        }
    }
    // Layers.
    let lbase = ptr(base, "layers");
    if v.layers.len() > MAX_LAYERS {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a view holds at most {MAX_LAYERS} layers"),
        )
        .at(lbase)
        .expected([MAX_LAYERS.to_string()])
        .build());
    }
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for (j, l) in v.layers.iter().enumerate() {
        if let Some(id) = &l.id {
            let ip = ptr(&ptr(&lbase, j), "id");
            if !valid_id(id) {
                return Err(
                    E::new(ErrorCode::Invalid, format!("invalid layer id `{id}`"))
                        .at(ip)
                        .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
                        .build(),
                );
            }
            if !taken.insert(id.clone()) {
                return Err(E::new(
                    ErrorCode::Duplicate,
                    format!("layer id `{id}` is used twice in view `{}`", v.id),
                )
                .at(ip)
                .build());
            }
        }
    }
    for j in 0..v.layers.len() {
        if v.layers[j].id.is_none() {
            let id = next_layer_id(&taken);
            taken.insert(id.clone());
            v.layers[j].id = Some(id);
        }
    }
    for j in 0..v.layers.len() {
        let mut l = v.layers[j].clone();
        let lp = ptr(&lbase, j);
        check_layer(
            doc,
            &mut l,
            &Site {
                view: &v,
                base: &lp,
                reads,
            },
        )?;
        v.layers[j] = l;
    }
    let emphasis = v.emphasis.clone();
    check_emphasis(doc, &v, &emphasis, &ptr(base, "emphasis"))?;
    Ok(v)
}

/// Canonical form of every view of a loaded document (`docs/GRAMMAR.md` §10 rule 6,
/// §11). A view that cannot be made canonical makes the document invalid.
///
/// # Errors
/// The first problem, as text with its JSON pointer.
pub fn canon_document(doc: &Document) -> Result<Document, String> {
    canon_document_owned(doc.clone())
}

/// [`canon_document`] of a document the caller gives away (R4/T6: no second copy of a
/// large document).
///
/// # Errors
/// As [`canon_document`].
pub fn canon_document_owned(mut doc: Document) -> Result<Document, String> {
    let mut views = Vec::with_capacity(doc.views.len());
    for i in 0..doc.views.len() {
        let v = canon_view(&doc, &doc.views[i], &format!("/views/{i}"), None).map_err(|e| {
            format!(
                "document `{}` is not valid: {} ({:?}) at {}",
                doc.id,
                e.message,
                e.code,
                e.path.unwrap_or_default()
            )
        })?;
        views.push(v);
    }
    doc.views = views;
    Ok(doc)
}
