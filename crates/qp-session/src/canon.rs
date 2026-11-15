//! The visual grammar v0 in the authority: canonical form of views, validation of
//! layers and panels, and origins of numeric constants (`docs/GRAMMAR.md` §2–§11).
//!
//! Kept apart from op expansion (`gops.rs`) so that these rules can later move into
//! `qp-grammar` without touching the protocol code. Every function here is pure: the same
//! document and view give the same result (no clock, no randomness).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{
    ColorSpec, DataRef, Emphasis, Layer, Mark, Operand, Origin, Panel, Position, Scale, ScaleKind,
    SizeSpec, View,
};
use qp_schema::protocol::{Candidate, ErrorCode, Target};
use qp_schema::{Document, Fact, Finding, Series, SeriesData, SeriesSource};

use crate::apply::{all_ids, find_collection};
use crate::error::{invalid, num, ptr, PResult, E};
use crate::expand::{need_series, valid_id};
use crate::values::{main_output, study_outputs, values, Cache};

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
pub(crate) const CHANNELS: [&str; 8] =
    ["x", "y", "y2", "color", "opacity", "width", "size", "text"];

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
        Position::Ref(_) => None,
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
// Scales and panels
// ---------------------------------------------------------------------------

fn check_domain(d: [f64; 2], log: bool, strict: bool, path: &str) -> PResult<()> {
    let ok =
        d[0].is_finite() && d[1].is_finite() && if strict { d[0] < d[1] } else { d[0] <= d[1] };
    if !ok {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "domain [{}, {}] must be finite with min {} max",
                num(d[0]),
                num(d[1]),
                if strict { "<" } else { "<=" }
            ),
        )
        .at(path)
        .expected([if strict {
            "[min, max] with min < max"
        } else {
            "[min, max] with min <= max"
        }])
        .build());
    }
    if log && d[0] <= 0.0 {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "a log scale needs a positive domain (got min {})",
                num(d[0])
            ),
        )
        .at(path)
        .expected(["min > 0"])
        .build());
    }
    Ok(())
}

/// Checks a panel's value scale (kind, id, domain); fills its id.
pub(crate) fn check_value_scale(s: &mut Scale, panel: u32, path: &str) -> PResult<()> {
    let id = format!("y{panel}");
    if s.kind == ScaleKind::Time {
        return Err(E::new(
            ErrorCode::Invalid,
            "a panel's value scale is `linear` or `log`; `time` is only the x axis",
        )
        .at(ptr(path, "kind"))
        .expected(["linear", "log"])
        .build());
    }
    if let Some(given) = &s.id {
        if *given != id {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("the scale of panel {panel} has id `{id}`, not `{given}`"),
            )
            .at(ptr(path, "id"))
            .expected([id])
            .build());
        }
    }
    if let Some(d) = s.domain {
        check_domain(d, s.kind == ScaleKind::Log, true, &ptr(path, "domain"))?;
    }
    s.id = Some(id);
    Ok(())
}

pub(crate) fn check_weight(w: f64, path: &str) -> PResult<()> {
    if w.is_finite() && w > 0.0 {
        Ok(())
    } else {
        Err(E::new(
            ErrorCode::Invalid,
            format!("panel weight must be > 0 (got {})", num(w)),
        )
        .at(path)
        .expected(["> 0"])
        .build())
    }
}

/// Checks a panel whose id is assigned; fills its scale id.
pub(crate) fn check_panel(p: &mut Panel, path: &str) -> PResult<()> {
    let id = p.id.unwrap_or_default();
    if let Some(w) = p.weight {
        check_weight(w, &ptr(path, "weight"))?;
    }
    check_value_scale(&mut p.y, id, &ptr(path, "y"))
}

fn check_x(x: &mut Scale, path: &str) -> PResult<()> {
    if x.kind != ScaleKind::Time {
        return Err(E::new(ErrorCode::Invalid, "the x scale is always `time`")
            .at(ptr(path, "kind"))
            .expected(["time"])
            .build());
    }
    if let Some(given) = &x.id {
        if given != "x" {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("the time axis has id `x`, not `{given}`"),
            )
            .at(ptr(path, "id"))
            .expected(["x"])
            .build());
        }
    }
    if let Some(d) = x.domain {
        check_domain(d, false, false, &ptr(path, "domain"))?;
    }
    x.id = Some("x".to_owned());
    Ok(())
}

// ---------------------------------------------------------------------------
// Colours and style channels
// ---------------------------------------------------------------------------

fn is_hex(c: &str) -> bool {
    let b = c.as_bytes();
    (b.len() == 7 || b.len() == 9) && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
}

pub(crate) fn check_color(c: &str, path: &str) -> PResult<()> {
    if is_hex(c) || THEME_TOKENS.contains(&c) {
        return Ok(());
    }
    Err(E::new(
        ErrorCode::Invalid,
        format!("`{c}` is not a colour: use #rrggbb, #rrggbbaa or a theme name"),
    )
    .at(path)
    .dym(c, THEME_TOKENS)
    .expected(
        ["#rrggbb", "#rrggbbaa"]
            .into_iter()
            .chain(THEME_TOKENS)
            .map(str::to_owned),
    )
    .build())
}

fn positive(v: f64, what: &str, path: &str) -> PResult<()> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(E::new(
            ErrorCode::Invalid,
            format!("{what} must be > 0 (got {})", num(v)),
        )
        .at(path)
        .expected(["> 0"])
        .build())
    }
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

/// Where the layer being checked sits: its view and the JSON pointer of the layer in
/// the incoming message (pointers inside the layer, `/encoding/y`, are appended).
pub(crate) struct Site<'a> {
    pub view: &'a View,
    pub base: &'a str,
    /// Bound data for origins, when a command writes (absent on load).
    pub reads: Option<Reads<'a>>,
}

/// What a writing command may read for an origin (§18.3): the `value` of a fact on a
/// bound series comes from the reading document, if the writer is entitled to it.
#[derive(Clone, Copy)]
pub(crate) struct Reads<'a> {
    /// The data plane's reader.
    pub reader: crate::plane::Reader<'a>,
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

fn unknown_field(field: &str, of: &str, fields: &[String], path: &str) -> ProtocolErr {
    if fields.is_empty() {
        return E::new(
            ErrorCode::Invalid,
            format!("{of} has no fields, so `{field}` cannot be drawn"),
        )
        .at(path)
        .hint("Give the layer a series, fact or finding, or use a constant with an origin.")
        .build();
    }
    E::new(ErrorCode::Invalid, format!("{of} has no field `{field}`"))
        .at(path)
        .dym(field, fields.iter().map(String::as_str))
        .expected(fields.iter().cloned())
        .build()
}

type ProtocolErr = qp_schema::protocol::ProtocolError;

fn data_name(d: Data<'_>) -> String {
    match d {
        Data::None => "a layer without data".to_owned(),
        Data::Series(s) => format!("series `{}`", s.id),
        Data::Fact(f) => format!("fact `{}`", f.id),
        Data::Finding(f) => format!("finding `{}`", f.id),
    }
}

fn check_field(doc: &Document, d: Data<'_>, field: &str, path: &str) -> PResult<()> {
    let fields = data_fields(doc, d);
    if fields.iter().any(|f| f == field) {
        Ok(())
    } else {
        Err(unknown_field(field, &data_name(d), &fields, path))
    }
}

fn check_ref(doc: &Document, d: Data<'_>, r: &DataRef, path: &str) -> PResult<()> {
    let target = match &r.series {
        Some(s) => Data::Series(need_series(doc, s, &ptr(path, "series"))?),
        None => d,
    };
    if matches!(target, Data::None) {
        return Err(E::new(
            ErrorCode::Invalid,
            "a field reference without `series` needs layer data",
        )
        .at(path)
        .hint("Name the series: {\"series\":…,\"field\":…}.")
        .build());
    }
    if let Some(f) = &r.field {
        check_field(doc, target, f, &ptr(path, "field"))?;
    }
    Ok(())
}

/// Checks a position channel; returns its constant, if it is one.
fn check_position(doc: &Document, d: Data<'_>, p: &Position, path: &str) -> PResult<Option<f64>> {
    match p {
        Position::Number(v) => Ok(Some(*v)),
        Position::Name(s) => {
            if let Some(v) = numeric(s) {
                if !v.is_finite() {
                    return Err(invalid(format!("constant `{s}` is not finite"), path));
                }
                return Ok(Some(v));
            }
            check_field(doc, d, s, path)?;
            Ok(None)
        }
        Position::Ref(r) => {
            check_ref(doc, d, r, path)?;
            Ok(None)
        }
    }
}

/// Constants of a layer (pointer → value), in pointer order of the encoding.
#[allow(clippy::too_many_lines)]
fn check_channels(
    doc: &Document,
    d: Data<'_>,
    l: &Layer,
    site: &Site<'_>,
) -> PResult<Vec<(String, f64)>> {
    let e = &l.encoding;
    let mut consts = Vec::new();
    for (name, ch) in [("x", &e.x), ("y", &e.y), ("y2", &e.y2)] {
        if let Some(p) = ch {
            let pointer = format!("/encoding/{name}");
            if let Some(v) = check_position(doc, d, p, &site.at(&pointer))? {
                consts.push((pointer, v));
            }
        }
    }
    if let Some(c) = &e.color {
        match c {
            ColorSpec::Constant(s) => check_color(s, &site.at("/encoding/color"))?,
            ColorSpec::When(w) => {
                check_field(
                    doc,
                    d,
                    &w.when.field,
                    &site.at("/encoding/color/when/field"),
                )?;
                match &w.when.than {
                    Operand::Number(v) => {
                        consts.push(("/encoding/color/when/than".to_owned(), *v));
                    }
                    Operand::Ref(r) => check_ref(doc, d, r, &site.at("/encoding/color/when/than"))?,
                }
                check_color(&w.then, &site.at("/encoding/color/then"))?;
                check_color(&w.otherwise, &site.at("/encoding/color/else"))?;
            }
            ColorSpec::Steps(s) => {
                check_field(doc, d, &s.field, &site.at("/encoding/color/field"))?;
                for (i, t) in s.thresholds.iter().enumerate() {
                    let tp = site.at(&format!("/encoding/color/thresholds/{i}"));
                    if !t.is_finite() {
                        return Err(invalid("thresholds must be finite", tp));
                    }
                    if i > 0 && *t <= s.thresholds[i - 1] {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            "colour thresholds must be strictly ascending",
                        )
                        .at(tp)
                        .expected([format!("> {}", num(s.thresholds[i - 1]))])
                        .build());
                    }
                    consts.push((format!("/encoding/color/thresholds/{i}"), *t));
                }
                if s.colors.len() != s.thresholds.len() + 1 {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "{} thresholds need {} colours (got {})",
                            s.thresholds.len(),
                            s.thresholds.len() + 1,
                            s.colors.len()
                        ),
                    )
                    .at(site.at("/encoding/color/colors"))
                    .expected([(s.thresholds.len() + 1).to_string()])
                    .build());
                }
                for (i, c) in s.colors.iter().enumerate() {
                    check_color(c, &site.at(&format!("/encoding/color/colors/{i}")))?;
                }
            }
        }
    }
    if let Some(o) = e.opacity {
        if !(o.is_finite() && (0.0..=1.0).contains(&o)) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("opacity {} is outside [0, 1]", num(o)),
            )
            .at(site.at("/encoding/opacity"))
            .expected(["0..=1"])
            .build());
        }
    }
    if let Some(w) = e.width {
        positive(w, "width", &site.at("/encoding/width"))?;
    }
    match &e.size {
        Some(SizeSpec::Constant(s)) => positive(*s, "size", &site.at("/encoding/size"))?,
        Some(SizeSpec::ByField(s)) => {
            check_field(doc, d, &s.field, &site.at("/encoding/size/field"))?;
            let [a, b] = s.range;
            positive(a, "size range min", &site.at("/encoding/size/range/0"))?;
            positive(b, "size range max", &site.at("/encoding/size/range/1"))?;
            if a > b {
                return Err(E::new(ErrorCode::Invalid, "size range must be [min, max]")
                    .at(site.at("/encoding/size/range"))
                    .expected(["min <= max"])
                    .build());
            }
        }
        None => {}
    }
    if let Some(t) = &e.text {
        check_field(doc, d, t, &site.at("/encoding/text"))?;
    }
    Ok(consts)
}

/// The `value` of fact `f` when its series rests on a bound series (bars not in the
/// document): from the reading document, gated by the writer's entitlement (§18.3).
/// `Ok(None)`: not a bound fact, or not computable (the caller says why).
fn bound_fact_value(
    doc: &Document,
    f: &Fact,
    reads: Option<Reads<'_>>,
    path: &str,
) -> PResult<Option<OriginValue>> {
    let bound = crate::plane::bar_root(doc, &f.series)
        .is_some_and(|r| r.bars().is_none() && matches!(r.source, SeriesSource::Bind { .. }));
    if !bound {
        return Ok(None);
    }
    let Some(r) = reads else {
        return Ok(Some(OriginValue::Unknown));
    };
    if let Some(b) = r.reader.plane.gate(doc, Some(r.writer), &f.series) {
        return Err(b.error(&f.series, &ptr(path, "fact")));
    }
    let v = values(r.reader.rd, &mut Cache::default(), &f.series)
        .ok()
        .and_then(|v| {
            v.main_values()
                .get(usize::try_from(f.bar).ok()?)
                .copied()
                .flatten()
        });
    Ok(v.map(OriginValue::Value))
}

/// The value of an origin.
fn origin_value(
    doc: &Document,
    o: &Origin,
    path: &str,
    reads: Option<Reads<'_>>,
) -> PResult<OriginValue> {
    match o {
        Origin::Fact { fact, field } => {
            let f = need_fact(doc, fact, &ptr(path, "fact"))?;
            if field == "value" && !f.evidence.contains_key("value") {
                if let Some(v) = bound_fact_value(doc, f, reads, path)? {
                    return Ok(v);
                }
            }
            match fact_value(doc, f, field) {
                Some(v) => Ok(OriginValue::Value(v)),
                None => {
                    let mut ok: Vec<String> = vec!["bar".to_owned(), "t".to_owned()];
                    ok.extend(f.evidence.keys().cloned());
                    if !ok.iter().any(|x| x == "value") {
                        ok.push("value".to_owned());
                    }
                    Err(E::new(
                        ErrorCode::Invalid,
                        format!("fact `{fact}` has no numeric field `{field}`"),
                    )
                    .at(ptr(path, "field"))
                    .dym(field, ok.iter().map(String::as_str))
                    .expected(ok)
                    .build())
                }
            }
        }
        Origin::Threshold { name } => threshold(name).map(OriginValue::Value).ok_or_else(|| {
            E::new(ErrorCode::NotFound, format!("no named threshold `{name}`"))
                .at(ptr(path, "name"))
                .dym(name, THRESHOLDS.iter().map(|(n, _)| *n))
                .expected(THRESHOLDS.iter().map(|(n, _)| (*n).to_owned()))
                .hint("{\"kind\":\"describe\"} lists the thresholds.")
                .build()
        }),
        Origin::Series { series, field, .. } => {
            let s = need_series(doc, series, &ptr(path, "series"))?;
            if let Some(f) = field {
                check_field(doc, Data::Series(s), f, &ptr(path, "field"))?;
            }
            Ok(OriginValue::Live)
        }
    }
}

/// A numeric field of a fact: `bar`, `t`, an evidence key, or `value` (the main output
/// of the fact's series at the fact's bar when the evidence has no `value`).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn fact_value(doc: &Document, f: &Fact, field: &str) -> Option<f64> {
    match field {
        "bar" => Some(f.bar as f64),
        "t" => Some(f.t as f64),
        k if f.evidence.contains_key(k) => f.evidence.get(k).copied(),
        "value" => {
            let v = values(doc, &mut Cache::default(), &f.series).ok()?;
            v.main_values()
                .get(usize::try_from(f.bar).ok()?)
                .copied()
                .flatten()
        }
        _ => None,
    }
}

fn origin_label(o: &Origin) -> String {
    match o {
        Origin::Fact { fact, field } => format!("{fact}.{field}"),
        Origin::Threshold { name } => name.clone(),
        Origin::Series { series, field, .. } => match field {
            Some(f) => format!("{series}.{f} last"),
            None => format!("{series} last"),
        },
    }
}

/// Short text for an origin, e.g. `rsi.overbought`.
pub(crate) fn origin_text(o: &Origin) -> String {
    origin_label(o)
}

/// Studies drawn in panel `panel` (layers of `view` plus `extra`).
fn panel_studies<'a>(doc: &'a Document, view: &View, extra: &Layer) -> BTreeSet<&'a str> {
    let mut out = BTreeSet::new();
    let others = view
        .layers
        .iter()
        .filter(|l| l.id != extra.id && l.panel == extra.panel);
    for l in others.chain(std::iter::once(extra)) {
        if let Some(Series {
            source: SeriesSource::Study { study, .. },
            ..
        }) = l
            .series
            .as_ref()
            .and_then(|s| doc.series.iter().find(|x| &x.id == s))
        {
            out.insert(study.as_str());
        }
    }
    out
}

/// Series drawn in the panel of `extra`.
fn panel_series<'a>(view: &'a View, extra: &'a Layer) -> BTreeSet<&'a str> {
    view.layers
        .iter()
        .filter(|l| l.id != extra.id && l.panel == extra.panel)
        .chain(std::iter::once(extra))
        .filter_map(|l| l.series.as_deref())
        .collect()
}

fn origin_json(o: &Origin) -> String {
    serde_json::to_string(o).unwrap_or_default()
}

/// Candidate origins for constant `v` in the panel of `l`: thresholds of the panel's
/// studies and numeric evidence of facts on the panel's series, nearest value first.
fn candidates(doc: &Document, view: &View, l: &Layer, v: f64) -> Vec<Candidate> {
    let mut c: Vec<(f64, String, String)> = Vec::new();
    let studies = panel_studies(doc, view, l);
    for (name, tv) in THRESHOLDS {
        let study = name.split('.').next().unwrap_or_default();
        if studies.contains(study) {
            let o = Origin::Threshold {
                name: name.to_owned(),
            };
            c.push((
                (tv - v).abs(),
                origin_json(&o),
                format!("{name} = {}", num(tv)),
            ));
        }
    }
    let series = panel_series(view, l);
    for f in &doc.facts {
        if !series.contains(f.series.as_str()) {
            continue;
        }
        for (k, ev) in &f.evidence {
            if !ev.is_finite() {
                continue;
            }
            let o = Origin::Fact {
                fact: f.id.clone(),
                field: k.clone(),
            };
            c.push((
                (ev - v).abs(),
                origin_json(&o),
                format!("{} {k} {} (bar {})", f.kind, num(*ev), f.bar),
            ));
        }
    }
    c.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    c.into_iter()
        .take(10)
        .map(|(_, id, about)| Candidate { id, about })
        .collect()
}

/// Pointers of the layer that may carry an origin.
fn origin_slots(l: &Layer) -> Vec<String> {
    let mut v: Vec<String> = ["/encoding/x", "/encoding/y", "/encoding/y2"]
        .map(str::to_owned)
        .to_vec();
    match &l.encoding.color {
        Some(ColorSpec::When(w)) if matches!(w.when.than, Operand::Number(_)) => {
            v.push("/encoding/color/when/than".to_owned());
        }
        Some(ColorSpec::Steps(s)) => {
            for i in 0..s.thresholds.len() {
                v.push(format!("/encoding/color/thresholds/{i}"));
            }
        }
        _ => {}
    }
    v
}

fn position_slot<'a>(l: &'a mut Layer, pointer: &str) -> Option<&'a mut Option<Position>> {
    match pointer {
        "/encoding/x" => Some(&mut l.encoding.x),
        "/encoding/y" => Some(&mut l.encoding.y),
        "/encoding/y2" => Some(&mut l.encoding.y2),
        _ => None,
    }
}

/// Resolves the origins of a layer's constants (`docs/GRAMMAR.md` §10): explicit
/// origins must resolve and match their constant bit for bit; a missing constant is
/// written from its origin; a constant without an origin is bound to the one named
/// threshold of the panel's studies with exactly its value, else `no_origin`.
fn resolve_origins(
    doc: &Document,
    l: &mut Layer,
    consts: &[(String, f64)],
    site: &Site<'_>,
) -> PResult<()> {
    let slots = origin_slots(l);
    let given: Vec<(String, Origin)> = l
        .origin
        .iter()
        .map(|(k, o)| (k.clone(), o.clone()))
        .collect();
    for (k, o) in &given {
        let op = ptr(&site.at("/origin"), k);
        if !slots.contains(k) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("`{k}` is not a constant of this layer"),
            )
            .at(op)
            .dym(k, slots.iter().map(String::as_str))
            .expected(slots.clone())
            .build());
        }
        let value = origin_value(doc, o, &op, site.reads)?;
        let literal = consts.iter().find(|(p, _)| p == k).map(|(_, v)| *v);
        let is_position = position_slot(l, k).is_some();
        if is_position {
            let field_there = position_slot(l, k)
                .and_then(|s| s.as_ref())
                .is_some_and(|p| position_constant(p).is_none());
            if field_there {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`{k}` draws a field, not a constant; it takes no origin"),
                )
                .at(op)
                .build());
            }
        }
        match (value, literal) {
            (OriginValue::Unknown, Some(_)) => {}
            (OriginValue::Unknown, None) => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!(
                        "`{}` rests on bound data the authority cannot read here: write the constant at `{k}`",
                        origin_label(o)
                    ),
                )
                .at(site.at(k))
                .build())
            }
            (OriginValue::Live, Some(_)) => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("a `series` origin is live: do not write a constant at `{k}`"),
                )
                .at(site.at(k))
                .hint("Remove the constant, or bind it to a fact or a named threshold.")
                .build())
            }
            (OriginValue::Live, None) => {
                if !is_position {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("a `series` origin can only bind a position channel, not `{k}`"),
                    )
                    .at(op)
                    .expected(["/encoding/x", "/encoding/y", "/encoding/y2"])
                    .build());
                }
            }
            (OriginValue::Value(v), Some(lit)) => {
                if v.to_bits() != lit.to_bits() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "constant {} at `{k}` differs from its origin `{}` = {}",
                            num(lit),
                            origin_label(o),
                            num(v)
                        ),
                    )
                    .at(site.at(k))
                    .expected([num(v)])
                    .hint("Write the origin's value, or omit the constant and the authority writes it.")
                    .build());
                }
            }
            (OriginValue::Value(v), None) => {
                if let Some(slot) = position_slot(l, k) {
                    *slot = Some(Position::Number(v));
                }
            }
        }
    }
    for (k, v) in consts {
        if l.origin.contains_key(k) {
            continue;
        }
        let studies = panel_studies(doc, site.view, l);
        let names: BTreeSet<&str> = THRESHOLDS
            .iter()
            .filter(|(n, tv)| {
                tv.to_bits() == v.to_bits()
                    && studies.contains(n.split('.').next().unwrap_or_default())
            })
            .map(|(n, _)| *n)
            .collect();
        if names.len() == 1 {
            let name = names.into_iter().next().unwrap_or_default().to_owned();
            l.origin.insert(k.clone(), Origin::Threshold { name });
            continue;
        }
        let lid = l.id.clone().unwrap_or_default();
        let mut err = E::new(
            ErrorCode::NoOrigin,
            format!(
                "layer `{lid}` of view `{}`: constant {} at {k} has no origin",
                site.view.id,
                num(*v)
            ),
        )
        .at(site.at(k))
        .hint(format!(
            "Bind it to a fact (add one with add_fact, evidence {{\"level\":{}}}) or a named threshold, via the layer's `origin`.",
            num(*v)
        ))
        .build();
        err.candidates = candidates(doc, site.view, l, *v);
        return Err(err);
    }
    Ok(())
}

/// Validates one layer in its view and brings it to canonical form (origins of
/// constants; constants written from origins). The layer's id must be assigned and its
/// panel must be in `site.view.panels`; `site.view.layers` are the other layers (the
/// layer itself may be among them, matched by id).
pub(crate) fn check_layer(doc: &Document, l: &mut Layer, site: &Site<'_>) -> PResult<()> {
    let n = usize::from(l.series.is_some())
        + usize::from(l.fact.is_some())
        + usize::from(l.finding.is_some());
    if n > 1 {
        let second = if l.series.is_some() && l.fact.is_some() {
            "/fact"
        } else {
            "/finding"
        };
        return Err(E::new(
            ErrorCode::Invalid,
            "a layer draws at most one of `series`, `fact`, `finding`",
        )
        .at(site.at(second))
        .hint("Keep one data reference; draw the others in their own layers.")
        .build());
    }
    if let Some(s) = &l.series {
        need_series(doc, s, &site.at("/series"))?;
    }
    if let Some(f) = &l.fact {
        need_fact(doc, f, &site.at("/fact"))?;
    }
    if let Some(f) = &l.finding {
        need_finding(doc, f, &site.at("/finding"))?;
    }
    need_panel(site.view, l.panel, &site.at("/panel"))?;
    let d = layer_data(doc, l);
    match l.mark {
        Mark::Candle => {
            let ok = matches!(d, Data::Series(s) if crate::plane::is_bar_series(s));
            if !ok {
                return Err(E::new(ErrorCode::Invalid, "`candle` needs a bar series")
                    .at(site.at(if l.series.is_some() {
                        "/series"
                    } else {
                        "/mark"
                    }))
                    .expected(
                        doc.series
                            .iter()
                            .filter(|s| crate::plane::is_bar_series(s))
                            .map(|s| s.id.clone()),
                    )
                    .build());
            }
        }
        Mark::Band => {
            if l.encoding.y.is_none() || l.encoding.y2.is_none() {
                let missing = if l.encoding.y.is_none() { "y" } else { "y2" };
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`band` needs `y` and `y2`; `{missing}` is missing"),
                )
                .at(site.at("/encoding"))
                .expected(["y", "y2"])
                .build());
            }
        }
        Mark::Rule => {
            let has = |p: &Option<Position>, k: &str| {
                p.as_ref().and_then(position_constant).is_some() || l.origin.contains_key(k)
            };
            if !has(&l.encoding.y, "/encoding/y") && !has(&l.encoding.x, "/encoding/x") {
                return Err(E::new(
                    ErrorCode::Invalid,
                    "`rule` needs a constant `y` (horizontal) or `x` (vertical), or an origin for one",
                )
                .at(site.at("/encoding/y"))
                .hint("E.g. \"encoding\":{\"y\":70} with \"origin\":{\"/encoding/y\":{\"kind\":\"threshold\",\"name\":\"rsi.overbought\"}}.")
                .build());
            }
        }
        Mark::Text => {
            if l.encoding.text.is_none() {
                return Err(
                    E::new(ErrorCode::Invalid, "`text` needs the `text` channel")
                        .at(site.at("/encoding"))
                        .expected(["text"])
                        .build(),
                );
            }
        }
        Mark::Line | Mark::Area | Mark::Bar | Mark::Point => {}
    }
    let consts = check_channels(doc, d, l, site)?;
    resolve_origins(doc, l, &consts, site)
}

/// Checks a target inside view `view` (emphasis).
pub(crate) fn check_view_target(
    doc: &Document,
    view: &View,
    t: &Target,
    path: &str,
) -> PResult<()> {
    if let Some(v) = &t.view {
        if *v != view.id {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("an emphasis of view `{}` cannot target view `{v}`", view.id),
            )
            .at(ptr(path, "view"))
            .expected([view.id.clone()])
            .build());
        }
    }
    if let Some(p) = t.panel {
        need_panel(view, p, &ptr(path, "panel"))?;
    }
    if let Some(l) = &t.layer {
        need_layer(view, l, &ptr(path, "layer"))?;
    }
    if let Some(s) = &t.series {
        need_series(doc, s, &ptr(path, "series"))?;
    }
    if let Some(i) = &t.item {
        if find_collection(doc, i).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("item `{i}` does not exist"))
                    .at(ptr(path, "item"))
                    .dym(i, all_ids(doc))
                    .build(),
            );
        }
    }
    if let (Some(a), Some(b)) = (t.from_bar, t.to_bar) {
        if a > b {
            return Err(invalid(
                format!("from_bar {a} is after to_bar {b}"),
                ptr(path, "from_bar"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn check_emphasis(
    doc: &Document,
    view: &View,
    emphasis: &[Emphasis],
    path: &str,
) -> PResult<()> {
    for (i, e) in emphasis.iter().enumerate() {
        check_view_target(doc, view, &e.target, &ptr(&ptr(path, i), "target"))?;
    }
    Ok(())
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
    let mut out = doc.clone();
    for i in 0..doc.views.len() {
        let v = canon_view(doc, &doc.views[i], &format!("/views/{i}"), None).map_err(|e| {
            format!(
                "document `{}` is not valid: {} ({:?}) at {}",
                doc.id,
                e.message,
                e.code,
                e.path.unwrap_or_default()
            )
        })?;
        out.views[i] = v;
    }
    Ok(out)
}
