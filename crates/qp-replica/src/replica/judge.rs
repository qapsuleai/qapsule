//! The display rule: conditions of claims, reconciling, disputes and judging a claim.

use super::*;

impl Replica {
    /// The condition a claim is about.
    fn condition_of(&self, claim: &ClaimRef) -> Option<Condition> {
        let doc = self.document()?;
        match claim {
            ClaimRef::Condition { sub } => self.watches.get(sub).cloned(),
            ClaimRef::Intent { intent } => doc
                .intents
                .iter()
                .find(|i| &i.id == intent)
                .and_then(|i| i.trigger.clone()),
            ClaimRef::Capsule { finding, .. } => doc
                .findings
                .iter()
                .find(|f| &f.id == finding)
                .map(|f| f.invalidation.clone()),
        }
    }

    /// The display rule (§19.4) over every claim the authority asserts.
    pub(super) fn reconcile(&mut self) {
        self.reconcile_on(None);
    }

    /// [`Replica::reconcile`] after a change of bar series `changed` only: claims not
    /// resting on any of them keep their item (their data did not change).
    pub(super) fn reconcile_on(&mut self, changed: Option<&[String]>) {
        let keys: Vec<(ClaimRef, i64)> = self
            .claims
            .iter()
            .filter(|(_, k)| k.state == ClaimState::Asserted)
            .map(|(k, _)| k.clone())
            .collect();
        let old: BTreeMap<(ClaimRef, i64), DisplayItem> = std::mem::take(&mut self.display)
            .into_iter()
            .map(|i| ((i.claim.clone(), i.t), i))
            .collect();
        let mut out = Vec::with_capacity(keys.len());
        let mut changed_any = old.len() != keys.len();
        for key in keys {
            if let (Some(ch), Some(prev)) = (changed, old.get(&key)) {
                let roots = self.roots.get(&key);
                if roots.is_some_and(|r| !r.iter().any(|x| ch.contains(x))) {
                    out.push(prev.clone());
                    continue;
                }
            }
            let ev = self.claims[&key].ev.clone();
            let cond = self.condition_of(&key.0);
            let item = self.judge(&key.0, &ev, cond.as_ref());
            if old.get(&key) != Some(&item) {
                changed_any = true;
            }
            out.push(item);
        }
        let claims = &self.claims;
        self.roots.retain(|k, _| claims.contains_key(k));
        if changed_any {
            self.display_version += 1;
        }
        self.display = out;
    }

    /// Queues a `dispute` (§19.9) for a claim its own data contradicts, once per (claim,
    /// `t`, authority's evidence); it carries the digest of its own bars of the claim's bar
    /// series up to `t`. A rejection re-asserts the same evidence: no new dispute.
    fn send_dispute(&mut self, claim: &ClaimRef, ev: &ClaimEvidence, item: &DisplayItem) {
        let Some(s) = self.s.as_ref() else {
            return;
        };
        let rd = s.reading();
        let Some(bars) = qp_plane::values::find_series(rd, &ev.series).and_then(Series::bars)
        else {
            return;
        };
        let Ok(i) = bars.binary_search_by_key(&ev.t, |b| b.t) else {
            return;
        };
        let digest = qp_schema::data::data_digest(&bars[..=i]);
        let anchor_t = anchor_of(s.document(), &ev.series);
        if !self.disputes_sent.insert((
            claim.clone(),
            ev.t,
            serde_json::to_string(ev).unwrap_or_default(),
        )) {
            return;
        }
        self.dispute_counter += 1;
        self.outbox.push(serde_json::json!({
            "type": "dispute",
            "id": format!("replica:dispute:{}", self.dispute_counter),
            "claim": claim,
            "t": ev.t,
            "values": item.values,
            "anchor_t": anchor_t,
            "digest": digest,
        }));
    }

    /// Disputes sent so far.
    #[must_use]
    pub fn disputes_sent(&self) -> u64 {
        self.dispute_counter
    }

    /// Increases whenever the display state changes.
    #[must_use]
    pub fn display_version(&self) -> u64 {
        self.display_version
    }

    #[allow(clippy::too_many_lines)]
    fn judge(
        &mut self,
        claim: &ClaimRef,
        ev: &ClaimEvidence,
        cond: Option<&Condition>,
    ) -> DisplayItem {
        let mut item = DisplayItem {
            claim: claim.clone(),
            series: ev.series.clone(),
            t: ev.t,
            state: Shown::Pending,
            label: None,
            values: Vec::new(),
            reason: None,
        };
        let pending = |mut i: DisplayItem, why: &'static str| {
            i.reason = Some(why);
            i
        };
        let Some(cond) = cond else {
            return pending(item, "unknown_condition");
        };
        let Some(s) = self.s.as_mut() else {
            return pending(item, "no_document");
        };
        let (doc, rd, plane, cache) = s.eval_parts();
        // The bar series of every operand must be resolved from the document's anchor,
        // and the authority's values computed from the same anchor.
        let mut bss: Vec<String> = Vec::new();
        for v in &ev.values {
            let bs = bar_series(rd, &v.series)
                .map(|x| x.id.clone())
                .or_else(|| qp_plane::plane::bar_root(doc, &v.series).map(|x| x.id.clone()))
                .unwrap_or_else(|| v.series.clone());
            bss.push(bs);
        }
        let mut roots = bss.clone();
        roots.push(ev.series.clone());
        roots.sort();
        roots.dedup();
        self.roots.insert((claim.clone(), ev.t), roots);
        for bs in &bss {
            if let Some((state, _)) = plane.state_for(bs, None) {
                match state {
                    BindingState::Ready => {}
                    BindingState::NoEntitlement => return pending(item, "no_entitlement"),
                    BindingState::Stale => return pending(item, "stale"),
                    _ => return pending(item, "no_data"),
                }
                let doc_anchor = anchor_of(doc, bs);
                let own = plane.series.get(bs).and_then(|b| b.bound.anchor_t);
                if doc_anchor.is_none() || own != doc_anchor {
                    return pending(item, "anchor");
                }
            }
        }
        if anchor_of(doc, &ev.series) != ev.anchor_t {
            return pending(item, "anchor");
        }
        let Some((holds, own)) = eval_at(rd, cache, cond, Some(&ev.values)) else {
            return pending(item, "not_covered");
        };
        item.values = own;
        let own_closed = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| closed(rd, bs, v.t));
        let own_settled = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| self.settled.get(bs).is_some_and(|t| v.t <= *t));
        let own_clean = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| self.unclean.get(bs).is_none_or(|t| v.t < *t));
        let auth_final = ev.maturity == Maturity::Final;
        let trust = ev.clean || own_clean;
        if holds {
            if auth_final && own_closed && trust {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Final);
                return item;
            }
            if auth_final && own_settled && trust {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Provisional);
                return item;
            }
            if !auth_final && !own_closed && self.opts.live {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Provisional);
                return item;
            }
            let why = if !trust {
                "unclean"
            } else if auth_final {
                "not_closed"
            } else {
                "awaiting_final"
            };
            return pending(item, why);
        }
        // Its own data does not hold the claim.
        if auth_final && own_settled && !own_closed && own_clean {
            // The bar came from a settled fetch while its own feed lags (e.g. an outage):
            // the bars before it may miss corrections the feed never delivered. Ask for
            // the settled range once (§19.5 gap repair); never disputed before closing.
            let key = (ev.series.clone(), ev.t);
            if self.opts.refetch && self.refetch_asked.insert(key.clone()) {
                self.refetch_new.push(key);
            }
            return pending(item, "not_held");
        }
        if auth_final && own_closed && own_clean {
            let key = (ev.series.clone(), ev.t);
            if !self.opts.refetch || self.refetched.contains(&key) {
                item.state = Shown::Disputed;
                item.reason = Some("sources_differ");
                if self.opts.dispute {
                    self.send_dispute(claim, ev, &item);
                }
                return item;
            }
            if self.refetch_asked.insert(key.clone()) {
                self.refetch_new.push(key);
            }
            return pending(item, "refetch");
        }
        pending(item, "not_held")
    }
}
