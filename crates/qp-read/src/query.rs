//! Queries (§5.4), input gestures and presence (§4.6, §7).

mod view;

use std::collections::BTreeMap;

use qp_schema::protocol::{
    AtBar, AttentionState, Candidate, Collection, Description, ErrorCode, Gesture, Op,
    OutboundLine, PanelInfo, Query, QueryResult, Range, Reduce, Reduced, ScreenInfo, SeriesValues,
    Target, Topic, Viewport, PROTOCOL_VERSION,
};
use qp_schema::{CapsuleState, Document, Mark, Series, SeriesSource, View};
use serde_json::Value;

use crate::rd::{Conn, Rd};
use qp_ops::apply::{all_ids, find_collection};
use qp_ops::expand::{need_finding, need_series, need_view, view_bar_series};
use qp_ops::present::{present, profile_for};
use qp_ops::provider::DocProvider;
use qp_ops::bounds::resolve_range;
use qp_ops::select::{finding_about, finding_bar};
use qp_plane::error::{invalid, ptr, PResult, E};
use qp_plane::values::values;

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
        Collection::Comments => doc.comments.iter().map(item_json).collect(),
        Collection::Proposals => doc.proposals.iter().map(item_json).collect(),
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

impl Rd<'_> {
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
                        thresholds: qp_ops::canon::thresholds(),
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
                        principals: self.principals.to_vec(),
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
                        *ev = self.v.evidence_for(asker, ev);
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
                        focus: self.st.focus.get(principal).cloned(),
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
            Query::Preview {
                proposal,
                width,
                height,
            } => self.preview(ci, proposal, (*width, *height)),
            Query::Stories {
                instrument,
                series,
                range,
                limit,
                after,
            } => self.stories_query(
                ci,
                instrument,
                series.as_deref(),
                range.as_ref(),
                (*limit, after.as_deref()),
                qp,
            ),
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
}
