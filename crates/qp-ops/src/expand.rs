//! Expansion of protocol operations ([`Op`]) into replayable changes ([`ChangeOp`]),
//! with validation (`docs/PROTOCOL.md` §5.1). Each op is expanded against a scratch copy
//! of the document that already contains the effects of the earlier ops of the same
//! command; the caller discards the copy if any op fails (all or nothing).

mod bind;
mod checks;
mod describe;
mod op;
mod undo;

pub use bind::*;
use checks::*;
pub use describe::summarize_ops;
pub use op::expand_op;
use undo::*;

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{ColorSpec, Operand, Origin, Panel, Position};
use qp_schema::protocol::{
    ChangeOp, ErrorCode, LogEntry, Narrative, Op, ProposalState, Seq, Show, Target,
};
use qp_schema::{
    Author, Capsule, CapsuleState, Condition, Depth, Document, Fact, Finding, Intent, Layer,
    LedgerEntry, Mark, Outcome, Provenance, Scale, ScaleKind, Series, SeriesData, SeriesSource,
    View,
};

use crate::apply::{all_ids, apply_all, find_collection, first_overlap, touched_keys};
use crate::canon::{canon_view, limit_panels};
use crate::gops::{expand_grammar, layer_in_use, new_layer};
use qp_plane::error::{invalid, num, ptr, PResult, E};
use qp_plane::values::{bar_series, condition_series, find_series, study_outputs};

/// Who writes, when, and how ids are numbered.
pub struct Ctx<'a> {
    pub author: Author,
    pub approved_by: Option<Author>,
    pub at: i64,
    /// Log position of the entry being written.
    pub seq: Seq,
    /// `seq` used in assigned ids (the proposal's seq when accepting a proposal).
    pub id_seq: Seq,
    pub n: u32,
    pub created: Vec<String>,
    /// Principal id of the speaker (for `undo`).
    pub speaker: &'a str,
    pub is_owner: bool,
    /// Whether the speaker may decide (needed to undo decisions).
    pub can_decide: bool,
    pub log: &'a [LogEntry],
    /// Set when the command is an undo.
    pub undoes: Option<Seq>,
    /// The data plane, when the document has bound series (§18): bound bars for
    /// `add_fact` / `set_view_range`, the speaker's entitlement for `add_fact`.
    pub data: Option<qp_plane::plane::Reader<'a>>,
    /// The data plane, always: the speaker's data entitlement for conditions on bound
    /// series (§18.3) and the session's `max_bind_bars` (§18.7).
    pub plane: &'a qp_plane::plane::DataPlane,
}

impl<'a> Ctx<'a> {
    /// Bound data for origins of constants (§18.3), read as the speaker.
    pub fn reads(&self) -> Option<crate::canon::Reads<'a>> {
        self.data.map(|reader| crate::canon::Reads {
            reader,
            writer: self.speaker,
        })
    }

    pub fn prov(&self, based_on: Vec<String>) -> Provenance {
        Provenance {
            author: self.author.clone(),
            at: self.at,
            based_on,
            approved_by: self.approved_by.clone(),
        }
    }

    /// Takes the given id (checked) or assigns `<kind>-<seq>-<n>`; records it as created.
    pub fn new_id(
        &mut self,
        doc: &Document,
        kind: &str,
        given: Option<&String>,
        path: &str,
    ) -> PResult<String> {
        let id = match given {
            Some(g) => {
                check_id(g, path)?;
                g.clone()
            }
            None => format!("{kind}-{}-{}", self.id_seq, self.n),
        };
        if let Some(c) = find_collection(doc, &id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("id `{id}` already exists (in {})", coll_name(c)),
            )
            .at(path)
            .hint("Choose another id or omit it to have one assigned.")
            .into());
        }
        self.n += 1;
        self.created.push(id.clone());
        Ok(id)
    }

    fn note_created(&mut self, id: &str) {
        self.n += 1;
        self.created.push(id.to_owned());
    }
}

pub fn coll_name(c: qp_schema::protocol::Collection) -> &'static str {
    use qp_schema::protocol::Collection as C;
    match c {
        C::Series => "series",
        C::Views => "views",
        C::Facts => "facts",
        C::Findings => "findings",
        C::Capsules => "capsules",
        C::Intents => "intents",
        C::Narratives => "narratives",
        C::Comments => "comments",
        C::Proposals => "proposals",
    }
}

/// Id format `^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$`.
pub fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':' | b'-'))
}

fn series_ids(doc: &Document) -> Vec<&str> {
    doc.series.iter().map(|s| s.id.as_str()).collect()
}

pub fn need_series<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Series> {
    find_series(doc, id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("series `{id}` does not exist"))
            .at(path)
            .dym(id, series_ids(doc))
            .hint("Use an existing series id; list them with {\"kind\":\"list\",\"collection\":\"series\"}.")
            .build()
    })
}

pub fn need_view<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a View> {
    doc.views.iter().find(|v| v.id == id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("view `{id}` does not exist"))
            .at(path)
            .dym(id, doc.views.iter().map(|v| v.id.as_str()))
            .hint("Use an existing view id; list them with {\"kind\":\"list\",\"collection\":\"views\"}.")
            .build()
    })
}

pub fn need_finding<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Finding> {
    doc.findings.iter().find(|v| v.id == id).ok_or_else(|| {
        E::new(
            ErrorCode::NotFound,
            format!("finding `{id}` does not exist"),
        )
        .at(path)
        .dym(id, doc.findings.iter().map(|v| v.id.as_str()))
        .hint("List findings with {\"kind\":\"list\",\"collection\":\"findings\"}.")
        .build()
    })
}

pub fn need_item(doc: &Document, id: &str, path: &str) -> PResult<()> {
    if find_collection(doc, id).is_some() {
        Ok(())
    } else {
        Err(
            E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                .at(path)
                .dym(id, all_ids(doc))
                .build(),
        )
    }
}

/// Validates the series a condition names.
pub fn check_condition(doc: &Document, c: &Condition, path: &str) -> PResult<()> {
    for (field, id) in condition_series(c) {
        need_series(doc, id, &ptr(path, field))?;
    }
    if let Condition::Threshold { value, .. } = c {
        if !value.is_finite() {
            return Err(invalid(
                "threshold value must be finite",
                ptr(path, "value"),
            ));
        }
    }
    Ok(())
}

/// The first bar series drawn in a view, by kind (a bound series even when its bars are
/// not available).
pub fn view_bar_series_any<'a>(doc: &'a Document, view: &View) -> Option<&'a Series> {
    view.layers
        .iter()
        .filter_map(|l| l.series.as_deref())
        .find_map(|s| qp_plane::plane::bar_root(doc, s))
}

/// The first bar series drawn in a view.
pub fn view_bar_series<'a>(doc: &'a Document, view: &View) -> Option<&'a Series> {
    view.layers
        .iter()
        .filter_map(|l| l.series.as_deref())
        .find_map(|s| bar_series(doc, s))
}

/// Log positions that are undone (by an undo entry that is itself not undone).
pub fn undone_set(log: &[LogEntry]) -> BTreeSet<Seq> {
    let mut undone = BTreeSet::new();
    for e in log.iter().rev() {
        if let Some(x) = e.undoes {
            if !undone.contains(&e.seq) {
                undone.insert(x);
            }
        }
    }
    undone
}

pub fn need_proposal<'a>(
    doc: &'a Document,
    id: &str,
    path: &str,
) -> PResult<&'a qp_schema::protocol::Proposal> {
    let pr = doc.proposals.iter().find(|x| x.id == id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("proposal `{id}` does not exist"))
            .at(path)
            .dym(id, doc.proposals.iter().map(|x| x.id.as_str()))
            .hint("List pending proposals with {\"kind\":\"list\",\"collection\":\"proposals\",\"state\":\"pending\"}.")
            .build()
    })?;
    if pr.state != ProposalState::Pending {
        return Err(E::new(
            ErrorCode::NotPending,
            format!("proposal `{id}` is {}", proposal_state_name(pr.state)),
        )
        .at(path)
        .build());
    }
    Ok(pr)
}

pub fn proposal_state_name(s: ProposalState) -> &'static str {
    match s {
        ProposalState::Pending => "pending",
        ProposalState::Accepted => "accepted",
        ProposalState::Rejected => "rejected",
        ProposalState::Expired => "expired",
    }
}

#[allow(clippy::too_many_lines)]
/// Validates a bound series of `put_series` (§18.2, §18.7).
fn check_bind(doc: &Document, series: &Series, sp: &str, max_bind_bars: u32) -> PResult<()> {
    let SeriesSource::Bind {
        symbol,
        interval,
        window,
        feed,
        anchor_t,
    } = &series.source
    else {
        return Ok(());
    };
    let src = ptr(sp, "source");
    if anchor_t.is_some() {
        return Err(E::new(
            ErrorCode::Invalid,
            "`anchor_t` is written by the authority, never by clients",
        )
        .at(ptr(&src, "anchor_t"))
        .hint("Omit `anchor_t`; the authority anchors the binding on its first resolution.")
        .build());
    }
    qp_source::validate_id("symbol", symbol).map_err(|m| invalid(m, ptr(&src, "symbol")))?;
    if let Some(f) = feed {
        qp_source::validate_id("feed", f).map_err(|m| invalid(m, ptr(&src, "feed")))?;
    }
    if let Err(m) = qp_source::validate_interval(interval) {
        return Err(E::new(ErrorCode::Invalid, m)
            .at(ptr(&src, "interval"))
            .expected([
                "1m…1440m (not a multiple of 60)",
                "1h…24h",
                "1d",
                "1w",
                "1M",
            ])
            .build());
    }
    if let Some(c) = canonical_interval(interval) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("interval `{interval}` is not written canonically; the same feed is `{c}`"),
        )
        .at(ptr(&src, "interval"))
        .suggest([c])
        .build());
    }
    window
        .validate(max_bind_bars)
        .map_err(|m| invalid(m, ptr(&src, "window")))?;
    if series.data.is_some() {
        return Err(invalid(
            "a bound series carries no data: its bars come from the data plane",
            ptr(sp, "data"),
        ));
    }
    if let Some(i) = &series.instrument {
        if &i.symbol != symbol {
            return Err(E::new(
                ErrorCode::Invalid,
                format!(
                    "instrument symbol `{}` differs from the binding's `{symbol}`",
                    i.symbol
                ),
            )
            .at(ptr(&ptr(sp, "instrument"), "symbol"))
            .expected([symbol.clone()])
            .build());
        }
    }
    if let Some(iv) = &series.interval {
        if iv != interval {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("interval `{iv}` differs from the binding's `{interval}`"),
            )
            .at(ptr(sp, "interval"))
            .expected([interval.clone()])
            .build());
        }
    }
    let bound = doc
        .series
        .iter()
        .filter(|s| s.id != series.id && matches!(s.source, SeriesSource::Bind { .. }))
        .count();
    if bound + 1 > MAX_BOUND_SERIES {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a document holds at most {MAX_BOUND_SERIES} bound series"),
        )
        .at(ptr(&src, "kind"))
        .expected([MAX_BOUND_SERIES.to_string()])
        .build());
    }
    Ok(())
}

/// Validates `show` and returns the view, the panel id and whether the panel is new
/// (`with`: that series' panel; `panel`: an existing panel id, or the next id to add a
/// panel; neither: a new panel with the next id, at the bottom).
fn check_show<'a>(doc: &'a Document, sh: &Show, p: &str) -> PResult<(&'a View, u32, bool)> {
    let v = need_view(doc, &sh.view, &ptr(p, "view"))?;
    let next = match v.panels.iter().filter_map(|x| x.id).max() {
        None => Some(0),
        Some(m) => m.checked_add(1),
    };
    if let Some(w) = &sh.with {
        need_series(doc, w, &ptr(p, "with"))?;
        let Some(l) = v.layers.iter().find(|l| l.series.as_deref() == Some(w)) else {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("series `{w}` is not drawn in view `{}`", sh.view),
            )
            .at(ptr(p, "with"))
            .expected(
                v.layers
                    .iter()
                    .filter_map(|l| l.series.clone())
                    .collect::<BTreeSet<_>>(),
            )
            .build());
        };
        return Ok((v, l.panel, false));
    }
    let new_panel = |id: Option<u32>, path: &str| -> PResult<(&'a View, u32, bool)> {
        let id = id.ok_or_else(|| invalid("no panel id is left above the largest one", path))?;
        limit_panels(v, 1, path)?;
        Ok((v, id, true))
    };
    if let Some(panel) = sh.panel {
        if v.panel(panel).is_some() {
            return Ok((v, panel, false));
        }
        if Some(panel) == next {
            return new_panel(next, &ptr(p, "panel"));
        }
        let mut expected: Vec<String> = v
            .panels
            .iter()
            .filter_map(|x| x.id)
            .map(|i| i.to_string())
            .collect();
        if let Some(n) = next {
            expected.push(format!("{n} (new panel)"));
        }
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "panel {panel} does not exist in view `{}`; the next new panel is {}",
                sh.view,
                next.map_or_else(|| "none".to_owned(), |n| n.to_string())
            ),
        )
        .at(ptr(p, "panel"))
        .expected(expected)
        .build());
    }
    new_panel(next, p)
}

/// Expands all ops on a scratch copy of `doc`. Returns the new document, the changes,
/// their inverse (reversed) and, per change, the index of the op it came from.
pub fn expand_ops(
    doc: &Document,
    ops: &[Op],
    base_path: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Expanded> {
    let mut scratch = doc.clone();
    let mut changes = Vec::new();
    let mut inverse = Vec::new();
    let mut origin = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let p = ptr(base_path, i);
        let cs = expand_op(&scratch, op, &p, ctx)?;
        let inv = apply_all(&mut scratch, &cs).map_err(|m| {
            let code = if matches!(op, Op::Undo { .. }) {
                ErrorCode::UndoConflict
            } else {
                ErrorCode::Invalid
            };
            E::new(code, format!("cannot apply: {m}"))
                .at(p.clone())
                .build()
        })?;
        origin.extend(std::iter::repeat_n(i, cs.len()));
        changes.extend(cs);
        inverse.push(inv);
    }
    let inverse = inverse.into_iter().rev().flatten().collect();
    Ok(Expanded {
        doc: scratch,
        changes,
        inverse,
        origin,
    })
}

pub struct Expanded {
    pub doc: Document,
    pub changes: Vec<ChangeOp>,
    pub inverse: Vec<ChangeOp>,
    pub origin: Vec<usize>,
}

/// The principal an audience op writes for (R3, §20): the speaker, or another person
/// for an `owner` only.
fn audience_target(principal: Option<&str>, p: &str, ctx: &Ctx<'_>) -> PResult<String> {
    match principal {
        None => Ok(ctx.speaker.to_owned()),
        Some(w) if w == ctx.speaker => Ok(w.to_owned()),
        Some(w) if ctx.is_owner => {
            check_id(w, &ptr(p, "principal"))?;
            Ok(w.to_owned())
        }
        Some(w) => Err(E::new(
            ErrorCode::Forbidden,
            format!("only an owner sets the capsule preferences of another person (`{w}`)"),
        )
        .at(ptr(p, "principal"))
        .hint("Leave `principal` out to set your own.")
        .build()),
    }
}
