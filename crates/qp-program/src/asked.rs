//! What a program query asks about (PS §3.2): exactly one of `program` (IR), `source`
//! (text, lowered by `qp-lang-text` under the registry's `program.text.*` limits) or
//! `instance` (a program instance of the document: its IR, values and bar series);
//! the bar series to bind and the values; the reader's bars, gated by their data
//! entitlement as every series read is (`no_entitlement`, `no_data`).

use std::collections::BTreeMap;

use qp_lang::{Binding, Lang, Limit, Limits};
use qp_lang_text::{lower_with, SourceMap, TextLimits};
use qp_plane::error::{invalid, ptr, PResult, E};
use qp_plane::plane::is_bar_series;
use qp_plane::values::find_series;
use qp_schema::program::{Program, ProgramDoc, ProgramParamValue, ProgramRef};
use qp_schema::protocol::{ErrorCode, Language, ProgramError};
use qp_schema::{Bar, Series};
use qp_subsystem::SessionView;
use serde_json::Value;

use crate::inst::{input_of, instance_names, outputs_of, program_of};

/// A program query's fields.
pub(crate) struct Asked<'q> {
    pub(crate) program: Option<&'q Value>,
    pub(crate) source: Option<&'q String>,
    pub(crate) instance: Option<&'q String>,
    pub(crate) inputs: &'q [String],
    pub(crate) params: &'q BTreeMap<String, ProgramParamValue>,
    pub(crate) lang: Option<Language>,
}

/// The program a query is about, resolved.
pub(crate) struct Resolved {
    /// The IR as JSON (the checker's input).
    pub(crate) raw: Value,
    /// The text's source map, when text was given.
    pub(crate) map: Option<SourceMap>,
    /// The lowered program, when text was given and lowered to one.
    pub(crate) lowered: Option<Program>,
    /// The bar series to bind (given, or the instance's).
    pub(crate) input: Option<String>,
    /// Values to bind (the instance's, overlaid with the given ones).
    pub(crate) params: Value,
    /// The instance, when one was named.
    pub(crate) instance: Option<ProgramRef>,
}

/// The limits in force (the host's registry values).
pub(crate) fn limits(view: &dyn SessionView) -> Limits {
    Limits::from_params(|k| view.param(k))
}

/// The text language's limits from the registry (`program.text.*`).
fn text_limits(l: &Limits) -> TextLimits {
    let n = |k| usize::try_from(l.get(k)).unwrap_or(usize::MAX);
    TextLimits {
        max_source_chars: n(Limit::TextMaxSourceChars),
        max_token_chars: n(Limit::TextMaxTokenChars),
        max_depth: n(Limit::TextMaxDepth),
    }
}

/// The message language (`en` if absent, PS §3.2).
pub(crate) fn lang(l: Option<Language>) -> (Lang, Language) {
    match l {
        Some(Language::Tr) => (Lang::Tr, Language::Tr),
        _ => (Lang::En, Language::En),
    }
}

/// Resolves `a` against the reader's document. `Ok(Err(errors))`: the text did not lower.
///
/// # Errors
/// Not exactly one of `program`, `source`, `instance`; an unknown instance.
#[allow(clippy::result_large_err)]
pub(crate) fn resolve(
    view: &dyn SessionView,
    a: &Asked<'_>,
    limits: &Limits,
) -> PResult<Result<Resolved, Vec<ProgramError>>> {
    let given = [
        a.program.is_some(),
        a.source.is_some(),
        a.instance.is_some(),
    ];
    if given.iter().filter(|g| **g).count() != 1 {
        return Err(E::new(
            ErrorCode::Invalid,
            "give exactly one of `program`, `source`, `instance`",
        )
        .at("/query")
        .expected(["program", "source", "instance"])
        .build());
    }
    if a.inputs.len() > 1 {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "a program takes exactly one bar series, got {}",
                a.inputs.len()
            ),
        )
        .at("/query/inputs")
        .reason("E_NOT_BAR_SERIES")
        .expected(["one bar series"])
        .build());
    }
    let mut params = serde_json::to_value(a.params).unwrap_or(Value::Null);
    let mut r = Resolved {
        raw: Value::Null,
        map: None,
        lowered: None,
        input: a.inputs.first().cloned(),
        params: Value::Null,
        instance: None,
    };
    if let Some(p) = a.program {
        r.raw = p.clone();
    } else if let Some(src) = a.source {
        let lowered = match lower_with(src, lang(a.lang).1, &text_limits(limits)) {
            Ok(l) => l,
            Err(errors) => return Ok(Err(errors)),
        };
        if let Ok(ProgramDoc::Program(p)) = lowered.doc() {
            r.lowered = Some(p);
        }
        r.raw = lowered.ir;
        r.map = Some(lowered.map);
    } else if let Some(name) = a.instance {
        let doc = view.document();
        let outs = outputs_of(doc, name);
        let Some(pref) = outs.first().and_then(|s| program_of(s)) else {
            return Err(
                E::new(ErrorCode::NotFound, format!("no program instance `{name}`"))
                    .at("/query/instance")
                    .dym(name, instance_names(doc))
                    .build(),
            );
        };
        r.raw = serde_json::to_value(&pref.ir).unwrap_or(Value::Null);
        if r.input.is_none() {
            r.input = outs.first().and_then(|s| input_of(s)).map(str::to_owned);
        }
        let mut merged = serde_json::to_value(&pref.params).unwrap_or(Value::Null);
        if let (Value::Object(m), Value::Object(given)) = (&mut merged, &params) {
            m.extend(given.clone());
        }
        params = merged;
        r.instance = Some(pref.clone());
    }
    r.params = params;
    Ok(Ok(r))
}

/// The bar series `id` of the reader's document, or `invalid` (`E_NOT_BAR_SERIES`).
#[allow(clippy::result_large_err)]
pub(crate) fn bar_series<'v>(view: &'v dyn SessionView, id: &str) -> PResult<&'v Series> {
    let path = "/query/inputs/0";
    match find_series(view.reading(), id) {
        Some(s) if is_bar_series(s) => Ok(s),
        Some(_) => Err(
            E::new(ErrorCode::Invalid, format!("`{id}` is not a bar series"))
                .at(path)
                .reason("E_NOT_BAR_SERIES")
                .build(),
        ),
        None => Err(E::new(ErrorCode::NotFound, format!("no series `{id}`"))
            .at(path)
            .dym(id, view.reading().series.iter().map(|s| s.id.as_str()))
            .build()),
    }
}

/// The check binding of bar series `s` for `principal` (its bar count only when the
/// reader may read the bars).
pub(crate) fn binding(view: &dyn SessionView, principal: &str, s: &Series) -> Binding {
    let open = view
        .plane()
        .gate(view.document(), Some(principal), &s.id)
        .is_none();
    Binding {
        interval: s.interval.clone(),
        bars: s.bars().filter(|_| open).map(|b| b.len() as u64),
    }
}

/// The closed bars of bar series `s` as `principal` may read them (the last bar is the
/// live one and is left out).
///
/// # Errors
/// `no_entitlement` / `no_data` (§18.3); `no_data` when the series has no bars.
#[allow(clippy::result_large_err)]
pub(crate) fn closed_bars<'v>(
    view: &'v dyn SessionView,
    principal: &str,
    s: &'v Series,
) -> PResult<&'v [Bar]> {
    let path = "/query/inputs/0";
    if let Some(b) = view.plane().gate(view.document(), Some(principal), &s.id) {
        return Err(b.error(&s.id, path));
    }
    match s.bars() {
        Some(b) if !b.is_empty() => Ok(&b[..b.len() - 1]),
        _ => Err(
            E::new(ErrorCode::NoData, format!("series `{}` has no bars", s.id))
                .at(path)
                .build(),
        ),
    }
}

/// The asker's principal.
#[allow(clippy::result_large_err)]
pub(crate) fn principal<'v>(view: &'v dyn SessionView, conn: &str) -> PResult<&'v str> {
    view.conn(conn)
        .map(|c| c.principal)
        .ok_or_else(|| invalid("unknown connection", "/"))
}

/// The bar series path of a query (`inputs`).
#[allow(clippy::result_large_err)]
pub(crate) fn need_input(r: &Resolved) -> PResult<&str> {
    r.input.as_deref().ok_or_else(|| {
        E::new(
            ErrorCode::Invalid,
            "name the bar series to run on (`inputs`)",
        )
        .at(ptr("/query", "inputs"))
        .build()
    })
}
