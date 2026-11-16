//! Queries (§5.4), input gestures and presence (§4.6, §7).

use std::collections::BTreeMap;

use qp_schema::protocol::{
    AtBar, AttentionState, Candidate, Collection, Description, ErrorCode, Gesture, Op,
    OutboundLine, PanelInfo, Query, QueryResult, Range, Reduce, Reduced, ScreenInfo, SeriesValues,
    Target, Topic, Viewport, PROTOCOL_VERSION,
};
use qp_schema::{CapsuleState, Document, Mark, Series, SeriesSource, View};
use serde_json::Value;

use crate::apply::{all_ids, find_collection};
use crate::error::{invalid, ptr, PResult, E};
use crate::expand::{need_finding, need_series, need_view, view_bar_series};
use crate::present::{present, profile_for};
use crate::provider::DocProvider;
use crate::select::{finding_about, finding_bar};
use crate::session::{permissions, Conn, Session};
use crate::values::values;

pub(crate) fn check_target_pub(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    if let Some(v) = &t.view {
        need_view(doc, v, &ptr(path, "view"))?;
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

/// Names of the variants of a tagged enum, from its JSON Schema.
fn variant_names(schema: &Value, tag: &str) -> Vec<String> {
    schema
        .get("oneOf")
        .and_then(Value::as_array)
        .map(|bs| {
            bs.iter()
                .filter_map(|b| {
                    b.get("properties")
                        .and_then(|p| p.get(tag))
                        .and_then(|p| p.get("const"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn item_json<T: serde::Serialize>(x: &T) -> Value {
    serde_json::to_value(x).unwrap_or(Value::Null)
}

fn collection_items(doc: &Document, c: Collection) -> Vec<Value> {
    match c {
        Collection::Series => doc.series.iter().map(item_json).collect(),
        Collection::Views => doc.views.iter().map(item_json).collect(),
        Collection::Facts => doc.facts.iter().map(item_json).collect(),
        Collection::Findings => doc.findings.iter().map(item_json).collect(),
        Collection::Capsules => doc.capsules.iter().map(item_json).collect(),
        Collection::Intents => doc.intents.iter().map(item_json).collect(),
        Collection::Narratives => doc.narratives.iter().map(item_json).collect(),
        Collection::Proposals => doc.proposals.iter().map(item_json).collect(),
    }
}

/// Resolves a range to `(from, to)` inclusive bar indexes over `t`.
fn resolve_range(r: Option<&Range>, t: &[i64], path: &str) -> PResult<(usize, usize)> {
    let n = t.len();
    if n == 0 {
        return Err(invalid("the series has no bars", path));
    }
    let last = n - 1;
    let Some(r) = r else { return Ok((0, last)) };
    let rp = ptr(path, "range");
    let forms = [
        r.last.is_some(),
        r.from_bar.is_some() || r.to_bar.is_some(),
        r.from_t.is_some() || r.to_t.is_some(),
    ];
    if forms.iter().filter(|x| **x).count() != 1 {
        return Err(E::new(ErrorCode::Invalid, "give exactly one range form")
            .at(rp)
            .expected([
                "{\"last\":n}",
                "{\"from_bar\":a,\"to_bar\":b}",
                "{\"from_t\":a,\"to_t\":b}",
            ])
            .build());
    }
    let empty = || {
        E::new(ErrorCode::Invalid, "the range is empty")
            .at(rp.clone())
            .expected([format!("bars 0..={last}")])
            .build()
    };
    if let Some(k) = r.last {
        let k = usize::try_from(k).unwrap_or(usize::MAX).min(n);
        if k == 0 {
            return Err(empty());
        }
        return Ok((n - k, last));
    }
    if forms[1] {
        let a = r
            .from_bar
            .map_or(0, |x| usize::try_from(x).unwrap_or(usize::MAX));
        let b = r
            .to_bar
            .map_or(last, |x| usize::try_from(x).unwrap_or(usize::MAX).min(last));
        if a > b {
            return Err(empty());
        }
        return Ok((a, b));
    }
    let lo = r.from_t.unwrap_or(i64::MIN);
    let hi = r.to_t.unwrap_or(i64::MAX);
    let a = t.iter().position(|x| *x >= lo);
    let b = t.iter().rposition(|x| *x <= hi);
    match (a, b) {
        (Some(a), Some(b)) if a <= b => Ok((a, b)),
        _ => Err(empty()),
    }
}

fn reduce(vals: &[Option<f64>], t: &[i64], from: usize, how: Reduce) -> Reduced {
    let pts: Vec<(usize, f64)> = vals
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|x| (i, x)))
        .collect();
    let at = |i: usize, v: f64| Reduced {
        value: Some(v),
        bar: Some((from + i) as u64),
        t: Some(t[i]),
    };
    let none = Reduced {
        value: None,
        bar: None,
        t: None,
    };
    match how {
        Reduce::Count => Reduced {
            #[allow(clippy::cast_precision_loss)]
            value: Some(pts.len() as f64),
            bar: None,
            t: None,
        },
        Reduce::Mean => {
            if pts.is_empty() {
                none
            } else {
                let mut s = 0.0;
                for (_, v) in &pts {
                    s += v;
                }
                #[allow(clippy::cast_precision_loss)]
                let m = s / pts.len() as f64;
                Reduced {
                    value: Some(m),
                    bar: None,
                    t: None,
                }
            }
        }
        Reduce::First => pts.first().map_or(none, |(i, v)| at(*i, *v)),
        Reduce::Last => pts.last().map_or(none, |(i, v)| at(*i, *v)),
        Reduce::Min | Reduce::Max => {
            let mut best: Option<(usize, f64)> = None;
            for (i, v) in pts {
                let better = match best {
                    None => true,
                    Some((_, b)) => {
                        if how == Reduce::Min {
                            v < b
                        } else {
                            v > b
                        }
                    }
                };
                if better {
                    best = Some((i, v));
                }
            }
            best.map_or(none, |(i, v)| at(i, v))
        }
    }
}

impl Session {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn query(&mut self, ci: usize, q: &Query) -> PResult<QueryResult> {
        let qp = "/query";
        match q {
            Query::Describe {} => {
                let op_schema = schemars::schema_for!(Op).to_value();
                let q_schema = schemars::schema_for!(Query).to_value();
                let t_schema = schemars::schema_for!(Topic).to_value();
                let role = self.conns[ci].role;
                Ok(QueryResult::Description {
                    description: Description {
                        thresholds: crate::canon::thresholds(),
                        protocol: PROTOCOL_VERSION.to_owned(),
                        doc: rd!(self).id.clone(),
                        ops: variant_names(&op_schema, "op"),
                        queries: variant_names(&q_schema, "kind"),
                        topics: variant_names(&t_schema, "kind"),
                        studies: qp_compute::STUDY_NAMES
                            .iter()
                            .map(|s| {
                                (
                                    (*s).to_owned(),
                                    qp_compute::default_params(s).unwrap_or_default(),
                                )
                            })
                            .collect(),
                        principals: self.principals.clone(),
                        role,
                        permissions: permissions(role),
                    },
                })
            }
            Query::Get { id } => {
                let Some(c) = find_collection(&self.doc, id) else {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                            .at(ptr(qp, "id"))
                            .dym(id, all_ids(&self.doc))
                            .build(),
                    );
                };
                let item = collection_items(&self.doc, c)
                    .into_iter()
                    .find(|v| v.get("id").and_then(Value::as_str) == Some(id))
                    .unwrap_or(Value::Null);
                Ok(QueryResult::Item {
                    collection: c,
                    item,
                })
            }
            Query::List {
                collection,
                state,
                full,
            } => {
                if state.is_some() && *collection != Collection::Proposals {
                    return Err(invalid("`state` filters proposals only", ptr(qp, "state")));
                }
                let items = if let Some(st) = state {
                    self.doc
                        .proposals
                        .iter()
                        .filter(|p| p.state == *st)
                        .map(item_json)
                        .collect()
                } else {
                    collection_items(&self.doc, *collection)
                };
                let items = if full.unwrap_or(false) || *collection != Collection::Series {
                    items
                } else {
                    // Bars are bulky: list series without their data (v2, from the
                    // agent-writability measurement); `get` or `full: true` returns them.
                    items
                        .into_iter()
                        .map(|mut v| {
                            if let Some(o) = v.as_object_mut() {
                                o.remove("data");
                            }
                            v
                        })
                        .collect()
                };
                Ok(QueryResult::Items {
                    collection: *collection,
                    items,
                })
            }
            Query::Document {} => Ok(QueryResult::Document {
                document: Box::new(self.doc.clone()),
            }),
            Query::Log {
                after_seq,
                since_t,
                until_t,
                within_ms,
                author,
                limit,
            } => {
                let from_rel = within_ms.map(|w| {
                    self.clock
                        .saturating_sub(i64::try_from(w).unwrap_or(i64::MAX))
                });
                let mut entries: Vec<_> = self
                    .log
                    .iter()
                    .filter(|e| after_seq.is_none_or(|s| e.seq > s))
                    .filter(|e| since_t.is_none_or(|t| e.at >= t))
                    .filter(|e| until_t.is_none_or(|t| e.at <= t))
                    .filter(|e| from_rel.is_none_or(|t| e.at >= t))
                    .filter(|e| author.as_ref().is_none_or(|a| &e.author == a))
                    .cloned()
                    .collect();
                if let Some(l) = limit {
                    let l = *l as usize;
                    if entries.len() > l {
                        entries.drain(..entries.len() - l);
                    }
                }
                // (R3, §20) Engine items the asker may not see are projected out.
                if let Some(p) = self.proj.as_ref() {
                    let lang = self.lang_of(ci);
                    entries = entries
                        .iter()
                        .map(|e| self.project_entry(e, p, lang))
                        .collect();
                }
                // Claim evidence values are data-plane values (§18.3): filtered for the
                // asker; the stored entries stay intact for replay.
                let asker = &self.conns[ci].principal;
                for e in &mut entries {
                    if let Some(ev) = e.evidence.as_mut() {
                        *ev = crate::claims::for_principal(&self.doc, &self.plane, asker, ev);
                    }
                }
                Ok(QueryResult::Log { entries })
            }
            Query::Study {
                series,
                range,
                output,
                reduce: how,
            } => {
                need_series(rd!(self), series, &ptr(qp, "series"))?;
                self.gate_read(ci, series, &ptr(qp, "series"))?;
                let v = values(rd!(self), &mut self.cache, series)
                    .map_err(|m| invalid(m, ptr(qp, "series")))?;
                let (a, b) = resolve_range(range.as_ref(), &v.t, qp)?;
                let mut outputs: BTreeMap<String, Vec<Option<f64>>> = BTreeMap::new();
                if let Some(o) = output {
                    let Some(col) = v.outputs.get(o) else {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            format!("series `{series}` has no output `{o}`"),
                        )
                        .at(ptr(qp, "output"))
                        .expected(v.outputs.keys().cloned())
                        .dym(o, v.outputs.keys().map(String::as_str))
                        .build());
                    };
                    outputs.insert(o.clone(), col[a..=b].to_vec());
                } else {
                    for (k, col) in &v.outputs {
                        outputs.insert(k.clone(), col[a..=b].to_vec());
                    }
                }
                let t = v.t[a..=b].to_vec();
                match how {
                    None => Ok(QueryResult::Values {
                        values: SeriesValues {
                            series: series.clone(),
                            from_bar: a as u64,
                            t,
                            outputs,
                        },
                    }),
                    Some(r) => Ok(QueryResult::Reduced {
                        series: series.clone(),
                        reduce: *r,
                        bars: (b - a + 1) as u64,
                        values: outputs
                            .iter()
                            .map(|(k, col)| (k.clone(), reduce(col, &t, a, *r)))
                            .collect(),
                    }),
                }
            }
            Query::At { series, bar, t } => {
                need_series(rd!(self), series, &ptr(qp, "series"))?;
                self.gate_read(ci, series, &ptr(qp, "series"))?;
                let s = need_series(rd!(self), series, &ptr(qp, "series"))?;
                let bars = s.bars().ok_or_else(|| {
                    invalid(
                        format!("series `{series}` is not a bar series"),
                        ptr(qp, "series"),
                    )
                })?;
                if bars.is_empty() {
                    return Err(invalid("the series has no bars", ptr(qp, "series")));
                }
                let last = bars.len() - 1;
                let i = match (bar, t) {
                    (Some(_), Some(_)) => {
                        return Err(invalid("give `bar` or `t`, not both", qp));
                    }
                    (Some(b), None) => {
                        let b = usize::try_from(*b).unwrap_or(usize::MAX);
                        if b > last {
                            return Err(E::new(
                                ErrorCode::Invalid,
                                format!("bar {b} is past the last bar {last}"),
                            )
                            .at(ptr(qp, "bar"))
                            .expected([format!("0..={last}")])
                            .build());
                        }
                        b
                    }
                    (None, Some(t)) => bars
                        .iter()
                        .position(|x| x.t == *t)
                        .ok_or_else(|| invalid(format!("no bar has t = {t}"), ptr(qp, "t")))?,
                    (None, None) => last,
                };
                Ok(QueryResult::At {
                    at: self.at_bar(series, i),
                })
            }
            Query::Screen { principal } => {
                let p = principal
                    .clone()
                    .unwrap_or_else(|| self.conns[ci].principal.clone());
                self.need_principal(&p, &ptr(qp, "principal"))?;
                self.warm_values();
                let asker = self.conns[ci].principal.clone();
                let screens = self
                    .conns
                    .iter()
                    .filter(|c| c.principal == p && c.has_screen())
                    .filter_map(|c| self.screen_info(c, &asker))
                    .collect();
                Ok(QueryResult::Screen { screens })
            }
            Query::Attention { principal } => {
                self.need_principal(principal, &ptr(qp, "principal"))?;
                let conn = self
                    .conns
                    .iter()
                    .filter(|c| &c.principal == principal && c.has_screen())
                    .max_by_key(|c| c.last_input);
                // max_by_key returns the last maximum: with no input, the latest opened.
                let viewport = conn.and_then(|c| self.conn_viewport(c));
                Ok(QueryResult::Attention {
                    attention: AttentionState {
                        principal: principal.clone(),
                        viewport,
                        focus: self.focus.get(principal).cloned(),
                        at: conn.and_then(|c| c.last_at),
                        directives: self.directives.get(principal).cloned().unwrap_or_default(),
                    },
                })
            }
            Query::Hit {
                view,
                x,
                y,
                width,
                height,
            } => Ok(QueryResult::Hit {
                hit: self.hit(ci, view.as_deref(), *x, *y, (*width, *height))?,
            }),
            Query::Binding { series } => {
                let asker = self.conns[ci].principal.clone();
                Ok(QueryResult::Bindings {
                    bindings: self.bindings(&asker, series.as_deref(), &ptr(qp, "series"))?,
                })
            }
            Query::Capsules { .. } => self.capsules_query(ci, q),
            Query::Capsule { capsule } => self.capsule_query(ci, capsule, qp),
            Query::Ledger {
                key,
                meaning,
                detector,
                direction,
                by_kind,
            } => Ok(self.ledger_query(
                key.as_deref(),
                *meaning,
                detector.as_deref(),
                *direction,
                *by_kind,
            )),
            Query::VerifyEvidence { item } => {
                let asker = self.conns[ci].principal.clone();
                Ok(QueryResult::EvidenceCheck {
                    check: self.verify_evidence(&asker, item, &ptr(qp, "item"))?,
                })
            }
            Query::Present { finding, surface } => {
                let f = match finding {
                    Some(id) => need_finding(rd!(self), id, &ptr(qp, "finding"))?.clone(),
                    None => self.only_live_finding(qp)?,
                };
                let (kind, profile) = match surface {
                    Some(k) => (*k, profile_for(*k, None)),
                    None => (self.conns[ci].surface.kind, self.conns[ci].profile.clone()),
                };
                // A finding whose data the asker may not read: text and state, no
                // spark and no view (§18.3).
                let blocked = {
                    let asker = self.conns[ci].principal.as_str();
                    let facts_blocked = f.facts.iter().any(|id| {
                        self.doc
                            .facts
                            .iter()
                            .find(|x| &x.id == id)
                            .is_some_and(|x| {
                                self.plane.gate(&self.doc, Some(asker), &x.series).is_some()
                            })
                    });
                    let view_blocked = self
                        .doc
                        .capsules
                        .iter()
                        .find(|c| c.finding == f.id)
                        .and_then(|c| c.view.as_ref())
                        .and_then(|v| self.doc.views.iter().find(|x| &x.id == v))
                        .is_some_and(|v| {
                            v.layers
                                .iter()
                                .any(|l| self.plane.gate_layer(&self.doc, Some(asker), l).is_some())
                        });
                    facts_blocked || view_blocked
                };
                // (R3, §20) A capsule with a story speaks with the realizer's sentence for
                // the asker (its language and entitlement) when the authority runs the
                // engine or the asker reads another language (`Session::headlines`).
                let mut pr = present(
                    rd!(self),
                    &mut self.cache,
                    &f,
                    kind,
                    &profile,
                    &self.headlines,
                );
                if blocked {
                    pr.spark = None;
                    pr.view = None;
                    pr.capsule_view = None;
                }
                Ok(QueryResult::Presentation { presentation: pr })
            }
        }
    }

    fn need_principal(&self, p: &str, path: &str) -> PResult<()> {
        if self.principal(p).is_some() {
            Ok(())
        } else {
            Err(
                E::new(ErrorCode::NotFound, format!("unknown principal `{p}`"))
                    .at(path)
                    .dym(p, self.principals.iter().map(|x| x.id.as_str()))
                    .build(),
            )
        }
    }

    /// The finding `present` uses when none is named (K25): the only live finding.
    /// Several live findings → `ambiguous` with all of them as candidates, in the
    /// selector order of findings (§17.3: latest fact bar, ties in document order); the
    /// authority does not pick one. None → `not_found`.
    fn only_live_finding(&self, qp: &str) -> PResult<qp_schema::Finding> {
        let doc = rd!(self);
        let mut live: Vec<(Option<u64>, usize, &qp_schema::Finding)> = doc
            .findings
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                doc.capsules
                    .iter()
                    .any(|c| c.finding == f.id && c.state == CapsuleState::Live)
            })
            .map(|(i, f)| (finding_bar(doc, f), i, f))
            .collect();
        live.sort_by_key(|(b, i, _)| (*b, *i));
        match live.as_slice() {
            [] => Err(E::new(ErrorCode::NotFound, "there is no live finding")
                .at(qp)
                .hint("Name a finding with `finding`.")
                .build()),
            [(_, _, f)] => Ok((*f).clone()),
            _ => {
                let mut e = E::new(
                    ErrorCode::Ambiguous,
                    format!(
                        "{} live findings and no `finding`; the authority does not pick one",
                        live.len()
                    ),
                )
                .at(qp)
                .hint(
                    "Pass `finding` with one of the candidate ids, or a selector such as \
                     {\"latest\":true}. Candidates are in ascending order of their latest \
                     fact bar: the last candidate is the most recent, the one \
                     {\"latest\":true} picks.",
                )
                .build();
                e.candidates = live
                    .iter()
                    .map(|(_, _, f)| Candidate {
                        id: f.id.clone(),
                        about: finding_about(doc, f),
                    })
                    .collect();
                Err(e)
            }
        }
    }

    pub(crate) fn at_bar(&mut self, series: &str, i: usize) -> AtBar {
        let bars = rd!(self)
            .series
            .iter()
            .find(|s| s.id == series)
            .and_then(Series::bars)
            .unwrap_or_default();
        let ohlcv = bars[i].clone();
        let deps: Vec<String> = rd!(self)
            .series
            .iter()
            .filter(|s| {
                matches!(&s.source, SeriesSource::Study { inputs, .. } if inputs.first().map(String::as_str) == Some(series))
            })
            .map(|s| s.id.clone())
            .collect();
        let mut studies = BTreeMap::new();
        for d in deps {
            if let Ok(v) = values(rd!(self), &mut self.cache, &d) {
                let m: BTreeMap<String, Option<f64>> = v
                    .outputs
                    .iter()
                    .map(|(k, col)| (k.clone(), col.get(i).copied().flatten()))
                    .collect();
                studies.insert(d, m);
            }
        }
        AtBar {
            series: series.to_owned(),
            bar: i as u64,
            ohlcv,
            studies,
        }
    }

    /// Times of the view's future slots (after its last bar; point series such as a
    /// forecast fan, `docs/GRAMMAR.md` §3). A viewport may reach them (K25).
    pub(crate) fn future_times(&self, view: &View) -> Vec<i64> {
        qp_grammar::compile::future_times(view, &DocProvider::new(rd!(self), &self.cache))
    }

    /// Initial visible range of a view for a surface (§4.6): the slots whose time falls
    /// in `x.domain`, else the surface's last N bars and every future slot after them
    /// (K25: the default viewport covers the future points).
    pub(crate) fn initial_range(
        &self,
        view: &View,
        kind: qp_schema::protocol::SurfaceKind,
    ) -> Option<(u64, u64)> {
        use qp_schema::protocol::SurfaceKind as K;
        let bars = view_bar_series(rd!(self), view).and_then(Series::bars)?;
        if bars.is_empty() {
            return None;
        }
        let future = self.future_times(view);
        let last_slot = bars.len() - 1 + future.len();
        if let Some([a, b]) = view.x.domain {
            let times: Vec<i64> = bars
                .iter()
                .map(|x| x.t)
                .chain(future.iter().copied())
                .collect();
            #[allow(clippy::cast_precision_loss)]
            let from = times.iter().position(|t| *t as f64 >= a);
            #[allow(clippy::cast_precision_loss)]
            let to = times.iter().rposition(|t| *t as f64 <= b);
            if let (Some(f), Some(t)) = (from, to) {
                if f <= t {
                    return Some((f as u64, t as u64));
                }
            }
        }
        let n = match kind {
            K::Phone => 60,
            K::Watch => 20,
            K::Desktop | K::Voice | K::Headless => 120,
        };
        Some((bars.len().saturating_sub(n) as u64, last_slot as u64))
    }

    /// The last slot of a view: its last bar plus its future slots (K25); `None` without
    /// bars.
    pub(crate) fn last_slot(&self, view: &View) -> Option<u64> {
        let n = view_bar_series(rd!(self), view)
            .and_then(Series::bars)
            .map_or(0, <[qp_schema::Bar]>::len) as u64;
        let last = n.checked_sub(1)?;
        Some(last + self.future_times(view).len() as u64)
    }

    pub(crate) fn range_of(&self, c: &Conn, view: &View) -> Option<(u64, u64)> {
        let saved = c.viewports.get(&view.id).copied().and_then(|(a, b)| {
            // A saved viewport can outlive data that got shorter (an undone
            // `append_bars`): clamp it to the current slots (bars + future slots).
            let last = self.last_slot(view)?;
            let b = b.min(last);
            Some((a.min(b), b))
        });
        saved.or_else(|| self.initial_range(view, c.surface.kind))
    }

    /// The connection's current viewport: the last one set by input, else the initial
    /// range of the document's first view.
    pub(crate) fn conn_viewport(&self, c: &Conn) -> Option<Viewport> {
        if let Some(v) = &c.current {
            // Re-clamp through `range_of` (the data may have shrunk since).
            if let Some(view) = rd!(self).views.iter().find(|x| x.id == v.view) {
                if let Some((a, b)) = self.range_of(c, view) {
                    return Some(Viewport {
                        view: v.view.clone(),
                        from_bar: a,
                        to_bar: b,
                    });
                }
            }
            return Some(v.clone());
        }
        let view = rd!(self).views.first()?;
        let (a, b) = self.range_of(c, view)?;
        Some(Viewport {
            view: view.id.clone(),
            from_bar: a,
            to_bar: b,
        })
    }

    fn screen_info(&self, c: &Conn, asker: &str) -> Option<ScreenInfo> {
        let vp = self.conn_viewport(c)?;
        let view = rd!(self).views.iter().find(|v| v.id == vp.view)?;
        let bs = view_bar_series(rd!(self), view);
        let mut panels: BTreeMap<u32, (Vec<String>, Vec<Mark>)> = BTreeMap::new();
        for l in &view.layers {
            let e = panels.entry(l.panel).or_default();
            if let Some(s) = &l.series {
                if !e.0.contains(s) {
                    e.0.push(s.clone());
                }
            }
            e.1.push(l.mark);
        }
        let capsules = rd!(self)
            .capsules
            .iter()
            .filter(|cap| {
                self.doc
                    .findings
                    .iter()
                    .find(|f| f.id == cap.finding)
                    .is_some_and(|f| {
                        f.facts.iter().any(|fid| {
                            self.doc
                                .facts
                                .iter()
                                .find(|x| &x.id == fid)
                                .is_some_and(|x| x.bar >= vp.from_bar && x.bar <= vp.to_bar)
                        })
                    })
            })
            .map(|cap| cap.id.clone())
            .collect();
        // The viewport may end in a future slot (K25): the last close shown is the
        // close of the last bar in it.
        let last_close = bs
            .filter(|s| self.plane.gate(&self.doc, Some(asker), &s.id).is_none())
            .and_then(Series::bars)
            .and_then(|b| {
                b.get(
                    usize::try_from(vp.to_bar)
                        .ok()?
                        .min(b.len().checked_sub(1)?),
                )
            })
            .map(|b| b.c);
        // R2: name, scale and layers from the compiled scene (`docs/GRAMMAR.md` §16).
        let mut compiled = self.screen_panels(c, &vp, asker);
        // Stack order (top to bottom, §5.4), not panel id order: they differ after
        // `move_panel` / `insert_panel`.
        let stack = |id: u32| {
            view.panels
                .iter()
                .position(|p| p.id == Some(id))
                .unwrap_or(usize::MAX)
        };
        let mut panels: Vec<_> = panels.into_iter().collect();
        panels.sort_by_key(|(id, _)| (stack(*id), *id));
        Some(ScreenInfo {
            principal: c.principal.clone(),
            surface: c.surface.kind,
            viewport: vp,
            symbol: bs.and_then(|s| s.symbol().map(str::to_owned)),
            interval: bs.and_then(|s| s.interval.clone()),
            panels: panels
                .into_iter()
                .map(|(panel, (series, marks))| {
                    let (name, scale, layers) = match compiled.remove(&panel) {
                        Some((n, s, l)) => (n, s, l),
                        None => (None, None, Vec::new()),
                    };
                    PanelInfo {
                        select: crate::select::Resolver {
                            doc: rd!(self),
                            root: None,
                        }
                        .panel_hint(view, panel),
                        name,
                        scale,
                        layers,
                        panel,
                        series,
                        marks,
                    }
                })
                .collect(),
            capsules,
            last_close,
            view_state: (c.look != qp_schema::protocol::ViewState::default())
                .then(|| c.look.clone()),
        })
    }

    /// Handles a gesture; returns the result and the notifications it causes.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn input(
        &mut self,
        ci: usize,
        g: &Gesture,
    ) -> PResult<(QueryResult, Vec<OutboundLine>)> {
        let gp = "/gesture";
        let view_id = match g {
            Gesture::Look { view, .. }
            | Gesture::Pan { view, .. }
            | Gesture::Zoom { view, .. }
            | Gesture::Tap { view, .. } => view,
            // (R4) Pixel gestures and open/close: `interact.rs`.
            _ => return self.interact(ci, g),
        };
        let view = need_view(rd!(self), view_id, &ptr(gp, "view"))?.clone();
        let bar_series = view_bar_series(rd!(self), &view)
            .filter(|s| s.bars().is_some_and(|b| !b.is_empty()))
            .map(|s| s.id.clone())
            .ok_or_else(|| {
                invalid(
                    format!("view `{view_id}` draws no bar series"),
                    ptr(gp, "view"),
                )
            })?;
        let n = rd!(self)
            .series
            .iter()
            .find(|s| s.id == bar_series)
            .and_then(Series::bars)
            .map_or(0, <[qp_schema::Bar]>::len) as u64;
        if n == 0 {
            return Err(invalid(
                format!("series `{bar_series}` has no bars"),
                ptr(gp, "view"),
            ));
        }
        let last = n - 1;
        // K25: a viewport may reach the future slots (points after the last bar).
        let future = self.future_times(&view).len() as u64;
        let last_slot = last + future;
        let slots = n + future;
        // A saved viewport can outlive data that got shorter (e.g. an undone
        // `append_bars`): clamp it to the current slots before using it.
        let (from, to) = self
            .range_of(&self.conns[ci], &view)
            .unwrap_or((0, last_slot));
        let to = to.min(last_slot);
        let from = from.min(to);
        let past_bar = |field: &str, v: u64| {
            E::new(
                ErrorCode::Invalid,
                format!("bar {v} is past the last bar {last}"),
            )
            .at(ptr(gp, field))
            .expected([format!("0..={last}")])
            .build()
        };
        let past_slot = |field: &str, v: u64| {
            if future == 0 {
                return past_bar(field, v);
            }
            E::new(
                ErrorCode::Invalid,
                format!(
                    "slot {v} is past the last slot {last_slot} (last bar {last} + {future} future slot(s))"
                ),
            )
            .at(ptr(gp, field))
            .expected([format!("0..={last_slot}")])
            .build()
        };
        let (nf, nt) = match g {
            Gesture::Look {
                from_bar, to_bar, ..
            } => {
                if *to_bar > last_slot {
                    return Err(past_slot("to_bar", *to_bar));
                }
                if from_bar > to_bar {
                    return Err(invalid(
                        format!("from_bar {from_bar} is after to_bar {to_bar}"),
                        ptr(gp, "from_bar"),
                    ));
                }
                (*from_bar, *to_bar)
            }
            Gesture::Pan { bars, .. } => {
                let width = to - from;
                let max_from = last_slot.saturating_sub(width);
                let target = i128::from(from) + i128::from(*bars);
                let nf = u64::try_from(target.clamp(0, i128::from(max_from))).unwrap_or(0);
                (nf, nf + width)
            }
            Gesture::Zoom {
                factor, anchor_bar, ..
            } => {
                if !(factor.is_finite() && *factor > 0.0) {
                    return Err(invalid(
                        "factor must be a positive number",
                        ptr(gp, "factor"),
                    ));
                }
                let anchor = anchor_bar.unwrap_or(to);
                if anchor > last_slot {
                    return Err(past_slot("anchor_bar", anchor));
                }
                let width = to - from + 1;
                #[allow(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss
                )]
                let nw = {
                    let w = (width as f64 / factor).round();
                    let w = if w.is_finite() { w.max(2.0) } else { 2.0 };
                    (w.min(slots as f64) as u64).max(1)
                };
                #[allow(clippy::cast_precision_loss)]
                let r = if width > 1 {
                    (anchor as f64 - from as f64) / (width - 1) as f64
                } else {
                    1.0
                };
                #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                let off = (r * (nw - 1) as f64).round() as i128;
                let max_from = i128::from(slots - nw);
                let nf = u64::try_from((i128::from(anchor) - off).clamp(0, max_from)).unwrap_or(0);
                (nf, nf + nw - 1)
            }
            Gesture::Tap { panel, bar, .. } => {
                // A tap answers the bar's values (data plane, §18.3).
                self.gate_read(ci, &bar_series, &ptr(gp, "view"))?;
                if *bar > last {
                    return Err(past_bar("bar", *bar));
                }
                if view.panel(*panel).is_none() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("view `{view_id}` has no panel {panel}"),
                    )
                    .at(ptr(gp, "panel"))
                    .expected(
                        view.panels
                            .iter()
                            .filter_map(|p| p.id)
                            .map(|i| i.to_string()),
                    )
                    .build());
                }
                self.input_counter += 1;
                let principal = self.conns[ci].principal.clone();
                let c = &mut self.conns[ci];
                c.last_input = self.input_counter;
                c.last_at = Some(self.clock);
                self.focus.insert(
                    principal,
                    Target {
                        layer: None,
                        view: Some(view.id.clone()),
                        panel: Some(*panel),
                        series: None,
                        output: None,
                        item: None,
                        from_bar: Some(*bar),
                        to_bar: Some(*bar),
                    },
                );
                let i = usize::try_from(*bar).unwrap_or(0);
                return Ok((
                    QueryResult::At {
                        at: self.at_bar(&bar_series, i),
                    },
                    Vec::new(),
                ));
            }
            _ => unreachable!("pixel gestures return above"),
        };
        let vp = Viewport {
            view: view.id.clone(),
            from_bar: nf,
            to_bar: nt,
        };
        self.input_counter += 1;
        let c = &mut self.conns[ci];
        c.viewports.insert(view.id.clone(), (nf, nt));
        c.current = Some(vp.clone());
        c.last_input = self.input_counter;
        c.last_at = Some(self.clock);
        let principal = c.principal.clone();
        let events = self.viewport_events(&principal, &vp);
        Ok((QueryResult::Viewport { viewport: vp }, events))
    }
}
