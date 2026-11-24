//! Data plane queries (`docs/PROTOCOL.md` §18.3, §18.5): `binding`, `verify_evidence`
//! and the per-asker read gate.

use std::collections::BTreeMap;

use qp_schema::data::{data_digest, BindingState, EvidenceRef, DATA_DIGEST_ALG};
use qp_schema::protocol::{
    BindingInfo, ErrorCode, EvidenceCheck, EvidenceStatus, FactCheck, ValueCheck,
};
use qp_schema::{Fact, SeriesSource};

use crate::rd::Rd;
use qp_ops::expand::need_series;
use qp_plane::error::{PResult, E};
use qp_plane::plane::binding_of;
use qp_plane::values::{find_series, values};

impl Rd<'_> {
    /// `no_entitlement` / `no_data` at `path` if connection `ci`'s principal may not read
    /// the data of `series` now (§18.3).
    pub(crate) fn gate_read(&self, ci: usize, series: &str, path: &str) -> PResult<()> {
        match self
            .plane
            .gate(self.doc, Some(&self.conns[ci].principal), series)
        {
            Some(b) => Err(b.error(series, path)),
            None => Ok(()),
        }
    }

    /// The `binding` query for `asker`: every bound series (document order), or one.
    pub(crate) fn bindings(
        &mut self,
        asker: &str,
        series: Option<&str>,
        path: &str,
    ) -> PResult<Vec<BindingInfo>> {
        let bound: Vec<String> = self
            .doc
            .series
            .iter()
            .filter(|s| matches!(s.source, SeriesSource::Bind { .. }))
            .map(|s| s.id.clone())
            .collect();
        let ids = match series {
            Some(id) => {
                need_series(self.doc, id, path)?;
                if !bound.iter().any(|b| b == id) {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("series `{id}` is not bound (its source is not `bind`)"),
                    )
                    .at(path)
                    .expected(bound.clone())
                    .build());
                }
                vec![id.to_owned()]
            }
            None => bound,
        };
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(binding) = find_series(self.doc, &id).and_then(binding_of) else {
                continue;
            };
            let (state, reason) = self
                .plane
                .state_for(&id, Some(asker))
                .unwrap_or((BindingState::Pending, None));
            let anchor_t = binding
                .anchor_t
                .or_else(|| self.plane.series.get(&id).and_then(|bs| bs.bound.anchor_t));
            let position = if state == BindingState::Ready {
                // (RF1 S3.2) The store's position read without its prefix-digest memo
                // (the plane is the authority's, read through the view): the same digest
                // (`FeedStore::digest_u64` is `data_digest_u64` of all bars).
                self.plane
                    .series
                    .get(&id)
                    .and_then(|bs| bs.bound.store.as_ref())
                    .map(|s| qp_schema::data::DataPosition {
                        feed: s.feed().clone(),
                        bars: s.bars().len() as u64,
                        last_t: s.bars().last().map(|b| b.t),
                        seq: s.seq(),
                        digest: qp_schema::data::data_digest(s.bars()),
                    })
            } else {
                None
            };
            out.push(BindingInfo {
                series: id,
                binding,
                state,
                reason,
                anchor_t,
                position,
            });
        }
        Ok(out)
    }

    /// The `verify_evidence` query for `asker` (§18.5).
    pub(crate) fn verify_evidence(
        &mut self,
        asker: &str,
        item: &str,
        path: &str,
    ) -> PResult<EvidenceCheck> {
        let facts: Vec<Fact> = if let Some(f) = self.doc.facts.iter().find(|f| f.id == item) {
            vec![f.clone()]
        } else if let Some(fnd) = self.doc.findings.iter().find(|f| f.id == item) {
            fnd.facts
                .iter()
                .filter_map(|id| self.doc.facts.iter().find(|f| &f.id == id).cloned())
                .collect()
        } else {
            let ids = self
                .doc
                .facts
                .iter()
                .map(|f| f.id.as_str())
                .chain(self.doc.findings.iter().map(|f| f.id.as_str()));
            return Err(
                E::new(ErrorCode::NotFound, format!("no fact or finding `{item}`"))
                    .at(path)
                    .dym(item, ids)
                    .build(),
            );
        };
        let checks: Vec<FactCheck> = facts.iter().map(|f| self.check_fact(asker, f)).collect();
        Ok(EvidenceCheck {
            item: item.to_owned(),
            verified: checks.iter().all(|c| c.status == EvidenceStatus::Verified),
            facts: checks,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn check_fact(&mut self, asker: &str, f: &Fact) -> FactCheck {
        let mut c = FactCheck {
            fact: f.id.clone(),
            status: EvidenceStatus::Unavailable,
            message: String::new(),
            stored: f.evidence_ref.clone(),
            current: None,
            values: BTreeMap::new(),
        };
        let Some(stored) = f.evidence_ref.clone() else {
            c.status = EvidenceStatus::NoRef;
            c.message = "the fact has no evidence reference (written before RP2)".to_owned();
            return c;
        };
        if let Some(b) = self.plane.gate(self.doc, Some(asker), &stored.series) {
            c.status = if b.state == BindingState::NoEntitlement {
                EvidenceStatus::NoEntitlement
            } else {
                EvidenceStatus::Unavailable
            };
            c.message = format!(
                "bound series `{}` is `{}`: {}",
                b.series,
                qp_plane::plane::state_str(b.state),
                b.reason
            );
            return c;
        }
        let rd = rd!(self);
        let Some(bs) = find_series(rd, &stored.series) else {
            c.message = format!("series `{}` no longer exists", stored.series);
            return c;
        };
        let Some(bars) = bs.bars() else {
            c.message = format!("series `{}` has no bars now", stored.series);
            return c;
        };
        let find = |t: i64| bars.binary_search_by_key(&t, |b| b.t).ok();
        let (Some(i0), Some(i1)) = (find(stored.from_t), find(stored.to_t)) else {
            c.message = format!(
                "the range t={}…{} is no longer held by `{}`",
                stored.from_t, stored.to_t, stored.series
            );
            return c;
        };
        if i0 > i1 {
            c.message = "the stored range is empty".to_owned();
            return c;
        }
        let range = &bars[i0..=i1];
        let current = EvidenceRef {
            series: stored.series.clone(),
            binding: binding_of(bs),
            from_t: stored.from_t,
            to_t: stored.to_t,
            bars: range.len() as u64,
            alg: DATA_DIGEST_ALG.to_owned(),
            digest: data_digest(range),
        };
        // Evidence values of a fact on a study, recomputed at the fact's bar (by time).
        let is_study = find_series(rd, &f.series)
            .is_some_and(|s| matches!(s.source, SeriesSource::Study { .. }));
        let mut value_diff: Option<String> = None;
        if is_study {
            let v = values(rd!(self), self.cache, &f.series).ok();
            let idx = v.as_ref().and_then(|v| v.t.binary_search(&f.t).ok());
            for (key, back) in [("value", 0usize), ("prev", 1usize)] {
                let Some(then) = f.evidence.get(key).copied() else {
                    continue;
                };
                let now = match (&v, idx) {
                    (Some(v), Some(i)) if i >= back => {
                        v.main_values().get(i - back).copied().flatten()
                    }
                    _ => None,
                };
                let same = now.is_some_and(|n| n.to_bits() == then.to_bits());
                if !same && value_diff.is_none() {
                    value_diff = Some(match now {
                        Some(n) => format!("`{key}` {then} → {n}"),
                        None => format!("`{key}` {then} → not computable"),
                    });
                }
                c.values.insert(key.to_owned(), ValueCheck { then, now });
            }
        }
        let (status, message) = if current.bars != stored.bars {
            (
                EvidenceStatus::Changed,
                format!(
                    "bar count in t={}…{}: {} → {}",
                    stored.from_t, stored.to_t, stored.bars, current.bars
                ),
            )
        } else if current.digest != stored.digest {
            let tail = value_diff
                .as_ref()
                .map_or_else(String::new, |d| format!("; {d}"));
            (
                EvidenceStatus::Changed,
                format!(
                    "data digest {} → {}: a bar in t={}…{} changed{tail}",
                    stored.digest, current.digest, stored.from_t, stored.to_t
                ),
            )
        } else if let Some(d) = value_diff {
            (EvidenceStatus::Changed, d)
        } else {
            (
                EvidenceStatus::Verified,
                format!(
                    "same {} bars (digest {}) and same evidence values",
                    current.bars, current.digest
                ),
            )
        };
        c.status = status;
        c.message = message;
        c.current = Some(current);
        c
    }
}
