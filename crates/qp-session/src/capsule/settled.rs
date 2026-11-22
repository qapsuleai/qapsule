//! Reader sentences and headlines, capsule disputes and the settled step.

use super::*;

impl Session {
    /// Finding id → the realizer's sentence of every capsule with a story, for a reader
    /// with projection `p` in `lang` (hidden capsules left out).
    /// [`Session::story_headlines`], and the same sentences by capsule. With `reuse`, a
    /// capsule the projection does not touch (its story, facts and redaction are the
    /// entitled reader's) takes the authority's TR headline when that is current
    /// ([`Session::headlines_current`]).
    pub(crate) fn reader_sentences(
        &self,
        p: Option<&Proj>,
        lang: Language,
        reuse: bool,
    ) -> (
        BTreeMap<String, String>,
        Option<std::collections::HashMap<String, String>>,
    ) {
        let base =
            (reuse && lang == Language::Tr && self.headlines_current()).then_some(&self.headlines);
        let mut memo = StudyMemo::default();
        let mut heads = BTreeMap::new();
        let mut by = std::collections::HashMap::new();
        let mut dup = false;
        for c in &self.doc.capsules {
            let Some(story) = c.story.as_ref() else {
                continue;
            };
            if p.is_some_and(|p| p.hidden.contains(&c.id)) {
                continue;
            }
            let touched = p.is_some_and(|p| {
                p.stories.contains_key(&c.id)
                    || story_fact_ids(story)
                        .iter()
                        .any(|i| p.hidden.contains(i) || p.redact.contains(i))
            });
            let text = match base {
                Some(b) if !touched => b.get(&c.finding).cloned(),
                _ => self.reader_text_with(&self.doc, c, p, lang, &mut memo),
            };
            if let Some(t) = text {
                dup |= by.insert(c.id.clone(), t.clone()).is_some();
                heads.insert(c.finding.clone(), t);
            }
        }
        // Two capsules with one id: sentences by capsule would not say whose.
        (heads, (!dup).then_some(by))
    }

    /// Whether [`Session::headlines`] are what [`Session::story_headlines`] gives now
    /// (TR, entitled): written by the last refresh, with no log entry and no change of
    /// the bars they read since, and one capsule per finding.
    pub(super) fn headlines_current(&self) -> bool {
        let st = &self.head_state;
        self.runner.is_some()
            && st.built
            && st.log_len == self.log.len()
            && st.snaps.iter().all(|(s, v)| self.bars_snaps(s) == *v)
    }

    pub(crate) fn story_headlines(
        &self,
        p: Option<&Proj>,
        lang: Language,
    ) -> BTreeMap<String, String> {
        let mut memo = StudyMemo::default();
        self.doc
            .capsules
            .iter()
            .filter(|c| c.story.is_some() && p.is_none_or(|p| !p.hidden.contains(&c.id)))
            .filter_map(|c| {
                self.reader_text_with(&self.doc, c, p, lang, &mut memo)
                    .map(|t| (c.finding.clone(), t))
            })
            .collect()
    }

    /// Refreshes the authority's own headlines (TR, entitled) after the engine wrote.
    ///
    /// (R3/perf) Only the capsules whose sentence input may have changed since the last
    /// refresh are rewritten ([`crate::capidx::HeadState`]): those the log entries since
    /// then put or whose facts they put, and those of a series whose bars (as the
    /// sentences read them, bit for bit) changed. A series put, anything deleted, a
    /// capsule put at an index or two capsules with a story sharing a finding: all of
    /// them, as [`Session::story_headlines`] does.
    pub(crate) fn refresh_headlines(&mut self) {
        if self.runner.is_none() {
            return;
        }
        let mut st = std::mem::take(&mut self.head_state);
        let full = !st.built
            || st.log_len > self.log.len()
            || self.log[st.log_len..].iter().any(|e| {
                e.changes.iter().any(|c| {
                    matches!(
                        c,
                        ChangeOp::PutSeries { .. }
                            | ChangeOp::Delete { .. }
                            | ChangeOp::PutCapsule { index: Some(_), .. }
                    )
                })
            })
            || !self.refresh_some(&mut st);
        if full {
            self.headlines = self.story_headlines(None, Language::Tr);
            st = crate::capidx::HeadState::default();
            let mut ok = true;
            for c in &self.doc.capsules {
                let Some(s) = c.story.as_ref() else { continue };
                ok &= st.register(&c.id, &s.series, &c.finding, story_fact_ids(s));
            }
            if ok {
                let series: Vec<String> = st.by_series.keys().cloned().collect();
                for s in series {
                    let v = self.bars_snaps(&s);
                    st.roots.insert(s.clone(), self.bars_root(&s));
                    st.snaps.insert(s, v);
                }
                st.built = true;
            }
        }
        st.log_len = self.log.len();
        self.head_state = st;
    }

    /// The bars the sentences of series `s` read: in the reading document and in the
    /// document.
    fn bars_snaps(&self, s: &str) -> (crate::capidx::BarsSnap, crate::capidx::BarsSnap) {
        (
            crate::capidx::snap(bar_series(rd!(self), s).and_then(qp_schema::Series::bars)),
            crate::capidx::snap(bar_series(&self.doc, s).and_then(qp_schema::Series::bars)),
        )
    }

    /// (RP3/M15) The bar series story series `s` reads its bars from, when the reading
    /// document and the document agree (a bound series has bars in the reading document
    /// only; a series change rewrites every headline anyway).
    fn bars_root(&self, s: &str) -> Option<String> {
        let a = bar_series(rd!(self), s).map(|x| x.id.clone());
        let b = bar_series(&self.doc, s).map(|x| x.id.clone());
        a.filter(|a| b.as_ref().is_none_or(|b| b == a))
    }

    /// The incremental step of [`Session::refresh_headlines`]; `false`: not possible
    /// (rewrite all).
    fn refresh_some(&mut self, st: &mut crate::capidx::HeadState) -> bool {
        let mut dirty: BTreeSet<String> = BTreeSet::new();
        // (RP3/M15) Bar series whose bars may have changed since the last refresh: those
        // a data event changed and those a log entry wrote bars of. Only their story
        // series' snapshots are compared (comparing every series' bars after every engine
        // entry made a step of a large universe quadratic in its size).
        let mut bars_changed = std::mem::take(&mut st.dirty);
        let all = std::mem::take(&mut st.dirty_all);
        for e in &self.log[st.log_len..] {
            for c in &e.changes {
                match c {
                    ChangeOp::PutCapsule { capsule, .. } => {
                        dirty.insert(capsule.id.clone());
                    }
                    ChangeOp::PutFact { fact, .. } => {
                        if let Some(s) = st.by_fact.get(&fact.id) {
                            dirty.extend(s.iter().cloned());
                        }
                    }
                    ChangeOp::AppendBars { series, .. }
                    | ChangeOp::TruncateBars { series, .. }
                    | ChangeOp::SetLastBar { series, .. } => {
                        bars_changed.insert(series.clone());
                    }
                    _ => {}
                }
            }
        }
        let series: Vec<String> = st
            .snaps
            .keys()
            .filter(|s| {
                all || bars_changed.contains(*s)
                    || st
                        .roots
                        .get(*s)
                        .is_none_or(|r| r.as_ref().is_none_or(|r| bars_changed.contains(r)))
            })
            .cloned()
            .collect();
        for s in series {
            let now = self.bars_snaps(&s);
            if st.snaps.get(&s) != Some(&now) {
                if let Some(c) = st.by_series.get(&s) {
                    dirty.extend(c.iter().cloned());
                }
                st.snaps.insert(s, now);
            }
        }
        let mut heads = std::mem::take(&mut self.headlines);
        let mut memo = StudyMemo::default();
        let mut ok = true;
        for id in &dirty {
            if let Some(f) = st.unregister(id) {
                heads.remove(&f);
            }
            let Some(c) = self.doc_capsule(id) else {
                continue;
            };
            let Some(s) = c.story.as_ref() else { continue };
            if !st.register(&c.id, &s.series, &c.finding, story_fact_ids(s)) {
                ok = false;
                break;
            }
            if !st.snaps.contains_key(&s.series) {
                let v = self.bars_snaps(&s.series);
                st.roots.insert(s.series.clone(), self.bars_root(&s.series));
                st.snaps.insert(s.series.clone(), v);
            }
            if let Some(t) = self.reader_text_with(&self.doc, c, None, Language::Tr, &mut memo) {
                heads.insert(c.finding.clone(), t);
            }
        }
        self.headlines = heads;
        ok
    }

    // -----------------------------------------------------------------------
    // Disputes and settled data (§19.9 + §20)
    // -----------------------------------------------------------------------

    /// (R3/K1) Whether capsule `cap` is the running engine's: a story on a watched
    /// series (by the capsule index).
    pub(crate) fn runs_capsule(&self, cap: &str) -> bool {
        let Some(r) = self.runner.as_ref() else {
            return false;
        };
        self.doc_capsule(cap)
            .and_then(|c| c.story.as_ref())
            .is_some_and(|s| r.watched_of(&s.series).is_some())
    }

    /// `true` when `capsule` is a story of a watched series.
    pub(crate) fn engine_capsule(&self, capsule: &str) -> Option<String> {
        let r = self.runner.as_ref()?;
        let c = self.doc.capsules.iter().find(|c| c.id == capsule)?;
        let s = c.story.as_ref()?;
        r.watched_of(&s.series).map(|w| w.series.clone())
    }

    /// A `dispute` on an engine capsule: the claim is "the story's facts hold on the
    /// settled bars". Settled bars from the anchor to `t` are asked; the answer re-runs
    /// the detectors on the corrected data (see [`Session::capsule_settled`]).
    pub(crate) fn capsule_dispute(
        &mut self,
        ci: usize,
        id: &str,
        capsule: &str,
        t: i64,
        digest: Option<String>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        // (R3/K1) The capsule as the disputer's world has it.
        let public = self.public_view(&principal);
        let (finding, visible, series, story) = self.in_world(public, |s| {
            let c = s.doc.capsules.iter().find(|c| c.id == capsule);
            let finding = c.map(|c| c.finding.clone()).unwrap_or_default();
            let story = c.and_then(|c| c.story.clone());
            let visible = s
                .proj_for(&principal)
                .is_none_or(|p| !p.hidden.contains(capsule));
            (finding, visible, s.engine_capsule(capsule), story)
        });
        let claim = ClaimRef::Capsule {
            capsule: capsule.to_owned(),
            finding,
        };
        let reply = |s: &Self, v: DisputeVerdict, ev: Option<ClaimEvidence>| {
            out(
                &conn,
                ServerMsg::Result {
                    id: id.to_owned(),
                    seq: s.seq_to(&principal, s.seq()),
                    now: s.clock(),
                    data: QueryResult::Dispute {
                        verdict: v,
                        evidence: ev,
                    },
                    resolved: BTreeMap::new(),
                },
            )
        };
        let (Some(series), Some(story), true) = (series, story, visible) else {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        };
        if !self.bars_ok(&principal, &series) {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        }
        let seen = (conn.clone(), claim.clone(), t, digest.unwrap_or_default());
        if self.settle.seen_contains(&seen) {
            return vec![reply(self, DisputeVerdict::Duplicate, None)];
        }
        if story.stage == Stage::Retracted || t < story.history.first().map_or(i64::MAX, |h| h.t) {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        }
        self.settle
            .seen_insert(seen.clone(), DisputeVerdict::Pending);
        if let Some(r) = self.runner.as_mut() {
            r.disputes.push(CapDispute {
                conn: conn.clone(),
                capsule: capsule.to_owned(),
                series: series.clone(),
                t,
                seen: seen.clone(),
                public,
            });
        }
        let feed = self.plane.series.get(&series).map(|b| b.binding.feed_key());
        let from_t = self
            .plane
            .bars(&series)
            .and_then(<[Bar]>::first)
            .map_or(t, |b| b.t);
        if let Some(feed) = feed {
            self.settle.push_request(crate::dispute::SettleRequest {
                series: series.clone(),
                feed,
                from_t,
                to_t: t,
            });
        }
        let lines = self.serve_test_source();
        let verdict = self
            .settle
            .seen_get(&seen)
            .unwrap_or(DisputeVerdict::Pending);
        let ev = (verdict == DisputeVerdict::Rejected).then(|| self.capsule_evidence(&series, t));
        let mut v = vec![reply(self, verdict, ev)];
        v.extend(lines);
        v
    }

    fn capsule_evidence(&self, series: &str, t: i64) -> ClaimEvidence {
        let doc = rd!(self);
        ClaimEvidence {
            series: series.to_owned(),
            t,
            values: Vec::new(),
            maturity: Maturity::Final,
            clean: true,
            anchor_t: crate::claims::anchor_of(doc, series),
            feeds: find_series(doc, series)
                .and_then(qp_schema::Series::binding)
                .map(|b| vec![b.feed_key()])
                .unwrap_or_default(),
        }
    }

    /// After a settled answer on `series` (`lo..=hi`; `corrected`: bars changed): the
    /// detectors re-run on the authority's (corrected) final bars; every story with a fact
    /// the corrected data no longer gives (or gives with other evidence values) is
    /// **retracted** (K37: `data_corrected`, or `dispute_upheld` when disputed; never in
    /// the ledger). The detectors continue from the corrected data. Facts the corrected
    /// data would newly give are not asserted (history is not rewritten). Open capsule
    /// disputes the answer covers are decided. Returns the notifications.
    pub(crate) fn capsule_settled(
        &mut self,
        series: &str,
        lo: i64,
        hi: i64,
        corrected: bool,
    ) -> Vec<OutboundLine> {
        let Some(mut runner) = self.runner.take() else {
            return Vec::new();
        };
        let Some(wi) = runner.watched.iter().position(|w| w.series == series) else {
            self.runner = Some(runner);
            return Vec::new();
        };
        let mut lines = Vec::new();
        let mut retract: Vec<(String, u64, i64, &'static str)> = Vec::new();
        let mut retract_pub: Vec<(String, u64, i64, &'static str)> = Vec::new();
        if corrected {
            let rd = rd!(self);
            let bars: Vec<Bar> = find_series(rd, series)
                .and_then(qp_schema::Series::bars)
                .map(<[Bar]>::to_vec)
                .unwrap_or_default();
            let w = &mut runner.watched[wi];
            let done = w.done.min(bars.len());
            let flow = w
                .flow
                .as_ref()
                .map(|(seed, _)| flow_points(&bars[..done], *seed));
            let mut dets = detectors_of(runner.engine_name);
            let mut again: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
            for (i, b) in bars[..done].iter().enumerate() {
                let fp = flow.as_ref().and_then(|f| f.get(i));
                for det in &mut dets {
                    for d in det.push(b, fp) {
                        let f = qp_facts::to_fact(
                            det.as_ref(),
                            &d,
                            &w.symbol,
                            &w.series,
                            i as u64,
                            b.t,
                            qp_capsule::pipeline::ENGINE_VERSION,
                        );
                        let bits = f
                            .evidence
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_bits()))
                            .collect();
                        again.insert(f.id, bits);
                    }
                }
            }
            w.dets = dets;
            let broken = |engine: &Engine, public: bool| {
                broken_stories(
                    engine,
                    series,
                    &again,
                    &bars,
                    done,
                    runner.disputes.iter().filter(|d| !public || d.public),
                )
            };
            retract = broken(&runner.engine, false);
            if let Some(pe) = runner.pub_engine.as_ref() {
                retract_pub = broken(pe, true);
            }
        }
        let mut events = Vec::new();
        for (id, bar, t, why) in retract {
            if let Some(ev) = runner.engine.retract(&id, bar, t, why) {
                events.push(ev);
            }
        }
        // (R3/K1) The public-only engine's stories, retracted on the same corrected data.
        let mut events_pub = Vec::new();
        if let Some(pe) = runner.pub_engine.as_mut() {
            for (id, bar, t, why) in retract_pub {
                if let Some(ev) = pe.retract(&id, bar, t, why) {
                    events_pub.push(ev);
                }
            }
        }
        // Decide the open disputes the answer covers.
        let open = std::mem::take(&mut runner.disputes);
        for d in open {
            if d.series != series || d.t < lo || d.t > hi {
                runner.disputes.push(d);
                continue;
            }
            // (R3/K1) A public-view reader's dispute is decided on the public-only engine.
            let (evs, engine) = match (d.public, runner.pub_engine.as_ref()) {
                (true, Some(pe)) => (&events_pub, pe),
                _ => (&events, &runner.engine),
            };
            let retracted = evs.iter().any(|e| e.capsule == d.capsule)
                || engine
                    .closed_stories()
                    .iter()
                    .any(|s| s.id == d.capsule && s.story.stage == Stage::Retracted);
            let verdict = if retracted {
                DisputeVerdict::Upheld
            } else {
                DisputeVerdict::Rejected
            };
            self.settle.seen_insert(d.seen.clone(), verdict);
            let _ = &d.conn;
        }
        // (R3/K1) With a public world, whether an entry is written may not depend on the
        // licensed data (which stories of the authority's engine broke): one entry per
        // corrected answer, each world with its own retractions.
        let public_world = runner.pub_engine.is_some();
        if events.is_empty() && !(public_world && corrected) {
            self.runner = Some(runner);
            return lines;
        }
        let summary = |evs: &[qp_capsule::StoryEvent]| {
            if evs.is_empty() {
                format!("settled {series}")
            } else {
                let ids: Vec<&str> = evs.iter().map(|e| e.capsule.as_str()).collect();
                format!("retract {}", ids.join(", "))
            }
        };
        let changes = self.capsule_changes(&runner.engine, &events, &BTreeMap::new(), 0);
        if let Some(pe) = runner.pub_engine.as_ref() {
            let changes = self.in_world(true, |s| {
                s.capsule_changes(pe, &events_pub, &BTreeMap::new(), 0)
            });
            self.pending_pub = Some(crate::public::PubNext::Entry(crate::public::PubPending {
                changes,
                summary: Some(summary(&events_pub)),
                evidence: None,
                undoes: None,
                request: None,
            }));
        }
        if let Some(seq) = self.engine_entry_with(changes, summary(&events), None) {
            self.runner = Some(runner);
            self.refresh_headlines();
            lines.extend(self.entry_events(seq));
            lines.extend(self.capsule_topic_lines(seq, &events, &events_pub, &[]));
            return lines;
        }
        self.pending_pub = None;
        self.runner = Some(runner);
        lines
    }
}
