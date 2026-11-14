//! Applying primitive changes ([`ChangeOp`]) to a document, with their inverse.
//!
//! This is the only code that mutates a [`Document`]: the session uses it for every log
//! entry and [`crate::replay`] uses it to rebuild a document from a log. No study code,
//! presenter or clock is involved.

use std::collections::BTreeSet;

use qp_schema::protocol::{ChangeOp, Collection, LogEntry};
use qp_schema::{Document, SeriesData};

/// Which collection holds `id`, if any.
#[must_use]
pub fn find_collection(doc: &Document, id: &str) -> Option<Collection> {
    if doc.series.iter().any(|x| x.id == id) {
        Some(Collection::Series)
    } else if doc.views.iter().any(|x| x.id == id) {
        Some(Collection::Views)
    } else if doc.facts.iter().any(|x| x.id == id) {
        Some(Collection::Facts)
    } else if doc.findings.iter().any(|x| x.id == id) {
        Some(Collection::Findings)
    } else if doc.capsules.iter().any(|x| x.id == id) {
        Some(Collection::Capsules)
    } else if doc.intents.iter().any(|x| x.id == id) {
        Some(Collection::Intents)
    } else if doc.narratives.iter().any(|x| x.id == id) {
        Some(Collection::Narratives)
    } else if doc.proposals.iter().any(|x| x.id == id) {
        Some(Collection::Proposals)
    } else {
        None
    }
}

/// Every item id in the document, in collection order.
#[must_use]
pub fn all_ids(doc: &Document) -> Vec<&str> {
    let mut v: Vec<&str> = Vec::new();
    v.extend(doc.series.iter().map(|x| x.id.as_str()));
    v.extend(doc.views.iter().map(|x| x.id.as_str()));
    v.extend(doc.facts.iter().map(|x| x.id.as_str()));
    v.extend(doc.findings.iter().map(|x| x.id.as_str()));
    v.extend(doc.capsules.iter().map(|x| x.id.as_str()));
    v.extend(doc.intents.iter().map(|x| x.id.as_str()));
    v.extend(doc.narratives.iter().map(|x| x.id.as_str()));
    v.extend(doc.proposals.iter().map(|x| x.id.as_str()));
    v
}

/// Upsert `item` into `items` by id (a new item goes to `index`, default the end);
/// returns the replaced item.
fn upsert<T: Clone>(
    items: &mut Vec<T>,
    item: &T,
    index: Option<u32>,
    id: impl Fn(&T) -> &str,
) -> Result<Option<T>, String> {
    let key = id(item).to_owned();
    match items.iter().position(|x| id(x) == key) {
        Some(i) => Ok(Some(std::mem::replace(&mut items[i], item.clone()))),
        None => {
            let at = index.map_or(items.len(), |i| i as usize);
            if at > items.len() {
                return Err(format!("index {at} out of range for `{key}`"));
            }
            items.insert(at, item.clone());
            Ok(None)
        }
    }
}

macro_rules! put {
    ($doc:expr, $coll:ident, $item:expr, $index:expr, $variant:ident, $field:ident) => {{
        match upsert(&mut $doc.$coll, $item, *$index, |x| x.id.as_str())? {
            Some(old) => ChangeOp::$variant {
                $field: old,
                index: None,
            },
            None => ChangeOp::Delete {
                id: $item.id.clone(),
            },
        }
    }};
}

fn take<T>(items: &mut Vec<T>, id: &str, f: impl Fn(&T) -> &str) -> Option<(T, Option<u32>)> {
    items
        .iter()
        .position(|x| f(x) == id)
        .map(|i| (items.remove(i), u32::try_from(i).ok()))
}

/// Applies one change and returns the change that undoes it.
///
/// # Errors
/// The change does not fit the document (missing item, index out of range, ...).
#[allow(clippy::too_many_lines)]
pub fn apply_change(doc: &mut Document, change: &ChangeOp) -> Result<ChangeOp, String> {
    Ok(match change {
        ChangeOp::PutSeries { series, index } => {
            put!(doc, series, series, index, PutSeries, series)
        }
        ChangeOp::PutView { view, index } => put!(doc, views, view, index, PutView, view),
        ChangeOp::PutFact { fact, index } => put!(doc, facts, fact, index, PutFact, fact),
        ChangeOp::PutFinding { finding, index } => {
            put!(doc, findings, finding, index, PutFinding, finding)
        }
        ChangeOp::PutCapsule { capsule, index } => {
            put!(doc, capsules, capsule, index, PutCapsule, capsule)
        }
        ChangeOp::PutIntent { intent, index } => {
            put!(doc, intents, intent, index, PutIntent, intent)
        }
        ChangeOp::PutNarrative { narrative, index } => {
            put!(doc, narratives, narrative, index, PutNarrative, narrative)
        }
        ChangeOp::PutProposal { proposal, index } => {
            put!(doc, proposals, proposal, index, PutProposal, proposal)
        }
        ChangeOp::InsertLayer { view, index, layer } => {
            let v = view_mut(doc, view)?;
            let i = *index as usize;
            if i > v.layers.len() {
                return Err(format!("layer index {i} out of range in view `{view}`"));
            }
            v.layers.insert(i, layer.clone());
            ChangeOp::DeleteLayer {
                layer: None,
                view: view.clone(),
                index: *index,
            }
        }
        ChangeOp::DeleteLayer { view, index, .. } => {
            let v = view_mut(doc, view)?;
            let i = *index as usize;
            if i >= v.layers.len() {
                return Err(format!("layer index {i} out of range in view `{view}`"));
            }
            let layer = v.layers.remove(i);
            ChangeOp::InsertLayer {
                view: view.clone(),
                index: *index,
                layer,
            }
        }
        ChangeOp::SetXDomain { view, domain } => {
            let v = view_mut(doc, view)?;
            let old = std::mem::replace(&mut v.x.domain, *domain);
            ChangeOp::SetXDomain {
                view: view.clone(),
                domain: old,
            }
        }
        ChangeOp::SetCapsuleState { capsule, state } => {
            let c = doc
                .capsules
                .iter_mut()
                .find(|c| &c.id == capsule)
                .ok_or_else(|| format!("no capsule `{capsule}`"))?;
            let old = std::mem::replace(&mut c.state, *state);
            ChangeOp::SetCapsuleState {
                capsule: capsule.clone(),
                state: old,
            }
        }
        ChangeOp::PushLedger { entry } => {
            doc.ledger.entries.push(entry.clone());
            ChangeOp::PopLedger {}
        }
        ChangeOp::PopLedger {} => {
            let entry = doc
                .ledger
                .entries
                .pop()
                .ok_or_else(|| "ledger is empty".to_owned())?;
            ChangeOp::PushLedger { entry }
        }
        ChangeOp::Delete { id } => {
            if let Some((series, index)) = take(&mut doc.series, id, |x| &x.id) {
                ChangeOp::PutSeries { series, index }
            } else if let Some((view, index)) = take(&mut doc.views, id, |x| &x.id) {
                ChangeOp::PutView { view, index }
            } else if let Some((fact, index)) = take(&mut doc.facts, id, |x| &x.id) {
                ChangeOp::PutFact { fact, index }
            } else if let Some((finding, index)) = take(&mut doc.findings, id, |x| &x.id) {
                ChangeOp::PutFinding { finding, index }
            } else if let Some((capsule, index)) = take(&mut doc.capsules, id, |x| &x.id) {
                ChangeOp::PutCapsule { capsule, index }
            } else if let Some((intent, index)) = take(&mut doc.intents, id, |x| &x.id) {
                ChangeOp::PutIntent { intent, index }
            } else if let Some((narrative, index)) = take(&mut doc.narratives, id, |x| &x.id) {
                ChangeOp::PutNarrative { narrative, index }
            } else if let Some((proposal, index)) = take(&mut doc.proposals, id, |x| &x.id) {
                ChangeOp::PutProposal { proposal, index }
            } else {
                return Err(format!("no item `{id}`"));
            }
        }
        ChangeOp::AppendBars { series, bars } => {
            let s = doc
                .series
                .iter_mut()
                .find(|s| &s.id == series)
                .ok_or_else(|| format!("no series `{series}`"))?;
            match &mut s.data {
                Some(SeriesData::Bars { bars: b }) => b.extend(bars.iter().cloned()),
                None => {
                    s.data = Some(SeriesData::Bars { bars: bars.clone() });
                }
                Some(SeriesData::Points { .. }) => {
                    return Err(format!("series `{series}` holds points, not bars"))
                }
            }
            ChangeOp::TruncateBars {
                series: series.clone(),
                count: u32::try_from(bars.len()).map_err(|_| "too many bars".to_owned())?,
            }
        }
        ChangeOp::TruncateBars { series, count } => {
            let b = bars_mut(doc, series)?;
            let n = *count as usize;
            if n > b.len() {
                return Err(format!("series `{series}` has fewer than {n} bars"));
            }
            let removed = b.split_off(b.len() - n);
            ChangeOp::AppendBars {
                series: series.clone(),
                bars: removed,
            }
        }
        ChangeOp::PutLayer { .. }
        | ChangeOp::PutPanel { .. }
        | ChangeOp::DeletePanel { .. }
        | ChangeOp::SetEmphasis { .. } => {
            return Err("grammar changes are not implemented yet (R2/T5)".to_owned())
        }
        ChangeOp::SetLastBar { series, bar } => {
            let b = bars_mut(doc, series)?;
            let last = b
                .last_mut()
                .ok_or_else(|| format!("series `{series}` has no bars"))?;
            let old = std::mem::replace(last, bar.clone());
            ChangeOp::SetLastBar {
                series: series.clone(),
                bar: old,
            }
        }
    })
}

fn view_mut<'a>(doc: &'a mut Document, id: &str) -> Result<&'a mut qp_schema::View, String> {
    doc.views
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("no view `{id}`"))
}

fn bars_mut<'a>(doc: &'a mut Document, id: &str) -> Result<&'a mut Vec<qp_schema::Bar>, String> {
    let s = doc
        .series
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("no series `{id}`"))?;
    match &mut s.data {
        Some(SeriesData::Bars { bars }) => Ok(bars),
        _ => Err(format!("series `{id}` has no bars")),
    }
}

/// Applies changes in order; returns the inverse changes (already reversed).
///
/// # Errors
/// The first change that does not fit (the document is then partially changed; callers
/// work on a copy).
pub fn apply_all(doc: &mut Document, changes: &[ChangeOp]) -> Result<Vec<ChangeOp>, String> {
    let mut inverse = Vec::with_capacity(changes.len());
    for (i, c) in changes.iter().enumerate() {
        inverse.push(apply_change(doc, c).map_err(|e| format!("change {i}: {e}"))?);
    }
    inverse.reverse();
    Ok(inverse)
}

/// Rebuilds a document from the initial document and a log, applying only `changes`.
///
/// # Errors
/// A log entry out of sequence or a change that does not fit.
pub fn replay(initial: &Document, log: &[LogEntry]) -> Result<Document, String> {
    let mut doc = initial.clone();
    for (i, e) in log.iter().enumerate() {
        if e.seq != i as u64 + 1 {
            return Err(format!(
                "log entry {i} has seq {} (expected {})",
                e.seq,
                i + 1
            ));
        }
        apply_all(&mut doc, &e.changes).map_err(|m| format!("seq {}: {m}", e.seq))?;
    }
    Ok(doc)
}

/// Item ids a change touches (`ledger` for ledger changes).
#[must_use]
pub fn touched(change: &ChangeOp) -> &str {
    match change {
        ChangeOp::PutSeries { series, .. } => &series.id,
        ChangeOp::PutView { view, .. } => &view.id,
        ChangeOp::InsertLayer { view, .. }
        | ChangeOp::DeleteLayer { view, .. }
        | ChangeOp::SetXDomain { view, .. } => view,
        ChangeOp::PutFact { fact, .. } => &fact.id,
        ChangeOp::PutFinding { finding, .. } => &finding.id,
        ChangeOp::PutCapsule { capsule, .. } => &capsule.id,
        ChangeOp::SetCapsuleState { capsule, .. } => capsule,
        ChangeOp::PutIntent { intent, .. } => &intent.id,
        ChangeOp::PushLedger { .. } | ChangeOp::PopLedger {} => "ledger",
        ChangeOp::PutNarrative { narrative, .. } => &narrative.id,
        ChangeOp::PutProposal { proposal, .. } => &proposal.id,
        ChangeOp::Delete { id } => id,
        ChangeOp::AppendBars { series, .. }
        | ChangeOp::TruncateBars { series, .. }
        | ChangeOp::SetLastBar { series, .. } => series,
        ChangeOp::PutLayer { view, .. }
        | ChangeOp::PutPanel { view, .. }
        | ChangeOp::DeletePanel { view, .. }
        | ChangeOp::SetEmphasis { view, .. } => view,
    }
}

/// The set of ids a list of changes touches.
#[must_use]
pub fn touched_ids(changes: &[ChangeOp]) -> BTreeSet<String> {
    changes.iter().map(|c| touched(c).to_owned()).collect()
}
