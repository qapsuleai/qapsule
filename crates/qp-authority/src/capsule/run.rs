//! Running the capsule engine at settle points, capsule items and audience filters.

use super::*;

impl Session {
    // -----------------------------------------------------------------------
    // Runner: final bars → engine entries
    // -----------------------------------------------------------------------

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    /// Feeds every bar that became final on a watched series to the detectors and the
    /// engine; one engine entry per series that advanced; returns its notifications
    /// (`change`, `finding_state`, topic `capsules`).
    pub(crate) fn run_capsules(&mut self) -> Vec<OutboundLine> {
        let Some(mut runner) = self.runner.take() else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        // (R3/perf) The watched series' positions in the reading document, per structure
        // generation (only a structure change moves or renames series).
        if runner.pos_gen != self.struct_gen {
            let rd = rd!(self);
            for w in &mut runner.watched {
                w.pos = rd.series.iter().position(|s| s.id == w.series);
            }
            runner.pos_gen = self.struct_gen;
        }
        let mut wrote = false;
        for wi in 0..runner.watched.len() {
            let rd = rd!(self);
            let w = &runner.watched[wi];
            let Some(bs) = w
                .pos
                .and_then(|i| rd.series.get(i))
                .filter(|s| s.id == w.series)
                .or_else(|| find_series(rd, &w.series))
            else {
                continue;
            };
            let Some(all) = bs.bars() else {
                continue;
            };
            // `final_count(rd, &bs.id, …)`: `bs` is the series found under that id.
            let fin = final_count_of(Some(bs), all.len());
            if fin <= w.done {
                continue;
            }
            let bars: &[Bar] = &all[..fin];
            let meta = bs;
            let w = &mut runner.watched[wi];
            let flow = w.flow.as_ref().map(|(seed, _)| flow_points(bars, *seed));
            let from = w.done;
            let mut events = Vec::new();
            let mut events_pub = Vec::new();
            let mut made: BTreeMap<String, Fact> = BTreeMap::new();
            let mut said: Vec<ServerComment> = Vec::new();
            for i in from..fin {
                let fp = flow.as_ref().and_then(|f| f.get(i));
                let mut facts = Vec::new();
                for det in &mut w.dets {
                    for d in det.push(&bars[i], fp) {
                        let mut f = qp_facts::to_fact(
                            det.as_ref(),
                            &d,
                            &w.symbol,
                            &w.series,
                            i as u64,
                            bars[i].t,
                            qp_capsule::pipeline::ENGINE_VERSION,
                        );
                        f.evidence_ref = Some(qp_plane::plane::evidence_ref(meta, bars, i));
                        facts.push(f);
                    }
                }
                let evs = runner
                    .engine
                    .on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &facts);
                // (R3/K1) The public-only engine: the same bar, public facts only.
                if let Some(pe) = runner.pub_engine.as_mut() {
                    let evp = if facts.iter().any(is_licensed) {
                        let public: Vec<Fact> =
                            facts.iter().filter(|f| !is_licensed(f)).cloned().collect();
                        pe.on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &public)
                    } else {
                        pe.on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &facts)
                    };
                    events_pub.extend(evp);
                }
                if let (Some(m), Some(p)) = (w.muhur.as_mut(), fp) {
                    let server: Vec<Fact> =
                        facts.iter().filter(|f| is_licensed(f)).cloned().collect();
                    let obs = FlowObservation {
                        t: bars[i].t,
                        bar: i as u64,
                        instrument: w.symbol.clone(),
                        values: p.values.clone(),
                    };
                    for q in m.observe(&obs, &server) {
                        said.push(ServerComment {
                            instrument: q.instrument,
                            series: w.series.clone(),
                            bar: q.bar,
                            t: q.t,
                            codes: q.codes,
                            tr: q.tr,
                            en: q.en,
                            trust: q.trust,
                            attachments: q.attachments,
                        });
                    }
                }
                for f in facts {
                    made.insert(f.id.clone(), f);
                }
                events.extend(evs);
            }
            w.done = fin;
            let series = w.series.clone();
            let mut changes = self.capsule_changes(&runner.engine, &events, &made, fin - 1);
            // (RF1 S3.1) The look's subsystem changes (watches, §25.5), in the same entry.
            changes.extend(self.look_phase(&events, false));
            if let Some(pe) = runner.pub_engine.as_ref() {
                let changes = self.in_world(true, |s| {
                    let mut c = s.capsule_changes(pe, &events_pub, &made, fin - 1);
                    c.extend(s.look_phase(&events_pub, true));
                    c
                });
                self.pending_pub = Some(crate::public::PubPending::of(changes));
            }
            let summary = if fin - from == 1 {
                format!("capsules {series} bar {from}")
            } else {
                format!("capsules {series} bars {from}..={}", fin - 1)
            };
            let Some(seq) = self.engine_entry_with(changes, summary, None) else {
                self.pending_pub = None;
                continue;
            };
            for c in &said {
                runner.comments.push((seq, c.clone()));
            }
            self.runner = Some(runner);
            wrote = true;
            lines.extend(self.entry_events(seq));
            lines.extend(self.capsule_topic_lines(seq, &events, &events_pub, &said));
            lines.extend(self.written_phase(seq));
            runner = self.runner.take().expect("runner put back");
        }
        self.runner = Some(runner);
        // (R3/perf) The headlines once, after the last engine entry (nothing in between
        // reads them; the document they are written from is the last entry's).
        if wrote {
            self.refresh_headlines();
        }
        lines
    }

    #[allow(clippy::too_many_lines)]
    /// The changes of one look: facts that caused transitions, then per story its
    /// finding and capsule (last snapshot), then ledger rows of the closes.
    pub(super) fn capsule_changes(
        &self,
        engine: &Engine,
        events: &[qp_capsule::StoryEvent],
        made: &BTreeMap<String, Fact>,
        now: usize,
    ) -> Vec<ChangeOp> {
        let mut changes = Vec::new();
        let mut put_facts: BTreeSet<String> = BTreeSet::new();
        let mut order: Vec<&str> = Vec::new();
        let mut last: BTreeMap<&str, &qp_capsule::StoryEvent> = BTreeMap::new();
        for e in events {
            for f in &e.transition.facts {
                if put_facts.contains(f) || self.doc_fact(f).is_some() {
                    continue;
                }
                if let Some(fact) = made.get(f) {
                    put_facts.insert(f.clone());
                    changes.push(ChangeOp::PutFact {
                        fact: fact.clone(),
                        index: None,
                    });
                }
            }
            if !last.contains_key(e.capsule.as_str()) {
                order.push(&e.capsule);
            }
            last.insert(&e.capsule, e);
        }
        let rd = rd!(self);
        let mut memo = StudyMemo::default();
        for id in order {
            let e = last[id];
            let story = &e.story;
            let ids = story_fact_ids(story);
            let facts: Vec<&Fact> = ids
                .iter()
                .filter_map(|i| made.get(i).or_else(|| self.doc_fact(i)))
                .collect();
            let bars = bar_series(rd, &story.series)
                .and_then(qp_schema::Series::bars)
                .unwrap_or_default();
            let now = (now as u64).max(story.last_bar);
            let (tr, _) = sentence_pair_memo(
                story,
                &facts,
                bars,
                now,
                false,
                &mut memo,
                Some((&e.capsule, &self.sentences)),
                Some(Lang::Tr),
            )
            .unwrap_or_else(|| (e.finding.clone(), e.finding.clone()));
            let at = e.transition.t;
            let prov = Provenance {
                author: Author::Engine {
                    name: ENGINE_NAME.to_owned(),
                    version: qp_capsule::pipeline::ENGINE_VERSION.to_owned(),
                },
                at,
                based_on: vec![story.series.clone()],
                approved_by: None,
            };
            let existing = self.doc_capsule(&e.capsule);
            changes.push(ChangeOp::PutFinding {
                finding: Finding {
                    id: e.finding.clone(),
                    claim: tr.clone(),
                    facts: ids,
                    confidence: story.strength.clamp(0.0, 1.0),
                    invalidation: Condition::Text {
                        text: if engine.rule_names().1 == "rp4" {
                            OUTCOME_TEXT_RP4
                        } else {
                            OUTCOME_TEXT
                        }
                        .to_owned(),
                    },
                    provenance: prov.clone(),
                },
                index: None,
            });
            let state = match existing.map(|c| c.state) {
                Some(s @ (CapsuleState::Approved | CapsuleState::Rejected)) => s,
                _ => state_of(story.stage),
            };
            changes.push(ChangeOp::PutCapsule {
                capsule: Capsule {
                    id: e.capsule.clone(),
                    finding: e.finding.clone(),
                    state,
                    revision: u32::try_from(story.history.len().saturating_sub(1))
                        .unwrap_or(u32::MAX),
                    headline: tr,
                    glyphs: Vec::new(),
                    view: existing.and_then(|c| c.view.clone()),
                    depth: Depth::Capsule,
                    expires_at: None,
                    story: Some(story.clone()),
                    provenance: prov,
                },
                index: None,
            });
        }
        for e in events {
            if matches!(
                e.transition.stage,
                Stage::Realized | Stage::Invalidated | Stage::Expired
            ) {
                if let Some(r) = engine
                    .ledger_records()
                    .iter()
                    .rev()
                    .find(|r| r.entry.capsule.as_deref() == Some(e.capsule.as_str()))
                {
                    changes.push(ChangeOp::PushLedger {
                        entry: r.entry.clone(),
                    });
                }
            }
        }
        changes
    }

    // -----------------------------------------------------------------------
    // Items for a reader
    // -----------------------------------------------------------------------

    /// One capsule as listed for a reader (`p`: its projection; `doc`: the document it
    /// sees).
    pub(crate) fn capsule_item(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
    ) -> Option<CapsuleItem> {
        self.capsule_item_with(doc, c, p, lang, principal, &mut StudyMemo::default())
    }

    /// [`Session::capsule_item`] with the study values of one pass (`memo`).
    pub(crate) fn capsule_item_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleItem> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let (sentence, redacted) = self.reader_sentence_with(doc, c, p, lang, memo)?;
        let validity = self.reader_validity(doc, c, p, memo)?;
        let last = story.history.last()?;
        Some(CapsuleItem {
            id: c.id.clone(),
            finding: c.finding.clone(),
            instrument: story.instrument.clone(),
            series: story.series.clone(),
            kind: story.kind,
            direction: story.direction,
            time: story.time,
            stage: story.stage,
            state: c.state,
            tier: story.tier,
            importance: story.importance,
            confirmations: u8::try_from(story.families.len().clamp(1, 5)).unwrap_or(5),
            trust: story.trust,
            born_bar: story.born_bar,
            last_bar: story.last_bar,
            t: last.t,
            ledger_at_birth: story.ledger_at_birth,
            sentence,
            validity,
            relevant: self.relevant(doc, principal, &story.instrument),
            redacted,
            coin_flip: self.coin_flip_of(principal, story),
            voice: None,
        })
    }

    /// The audience of `principal` (the `habituated` default when it set none). (R5B,
    /// §25.7) An agent that works for a person and set no audience of its own reads with
    /// the person's.
    pub(crate) fn audience_of(&self, doc: &Document, principal: &str) -> Audience {
        let own = doc.audiences.get(principal).or_else(|| {
            self.plane
                .delegates
                .get(principal)
                .and_then(|person| doc.audiences.get(person))
        });
        own.cloned().unwrap_or(Audience {
            group: UserGroup::Habituated,
            sensitivity: None,
            watchlist: Vec::new(),
            positions: BTreeMap::new(),
        })
    }

    /// Instruments of the active intents' triggers (symbols of the series they read);
    /// (R5B, §25.5) and of `principal`'s own watches (an agent: its person's) — another
    /// person's watch never counts.
    pub(crate) fn intents_on(&self, doc: &Document, principal: &str) -> Vec<String> {
        let mut out = Vec::new();
        let person = qp_ops::watch::person_of(&self.plane, principal);
        for i in doc.intents.iter().filter(|i| i.active) {
            if let Some(w) = &i.watch {
                if w.for_ == person {
                    if let Some(sym) = &w.match_.instrument {
                        if !out.iter().any(|x| x == sym) {
                            out.push(sym.clone());
                        }
                    }
                }
                continue;
            }
            let Some(t) = &i.trigger else { continue };
            for (_, s) in qp_plane::values::condition_series(t) {
                if let Some(sym) = bar_series(doc, s).and_then(qp_schema::Series::symbol) {
                    if !out.iter().any(|x| x == sym) {
                        out.push(sym.to_owned());
                    }
                }
            }
        }
        out
    }

    /// About `principal`'s position or one of the document's active intents.
    pub(crate) fn relevant(&self, doc: &Document, principal: &str, instrument: &str) -> bool {
        let a = self.audience_of(doc, principal);
        a.positions.get(instrument).is_some_and(|q| *q != 0.0)
            || self
                .intents_on(doc, principal)
                .iter()
                .any(|s| s == instrument)
    }

    /// Server comments `principal` gets: those on instruments whose licensed flow it may
    /// not read.
    pub(crate) fn comments_for(&self, principal: &str) -> Vec<ServerComment> {
        let Some(r) = self.runner.as_ref() else {
            return Vec::new();
        };
        r.comments
            .iter()
            .filter(|(_, c)| !self.flow_ok(principal, &c.instrument))
            .map(|(_, c)| c.clone())
            .collect()
    }

    /// Whether `story` passes the filters of the `capsules` query `q` (all but
    /// `latest` and `limit`).
    pub(crate) fn capsule_passes(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        q: &qp_schema::protocol::Query,
    ) -> bool {
        let qp_schema::protocol::Query::Capsules {
            instrument,
            stages,
            open,
            meaning,
            since_bar,
            since_t,
            born_since_bar,
            born_since_t,
            audience,
            ..
        } = q
        else {
            return false;
        };
        !(instrument.as_ref().is_some_and(|i| i != &story.instrument)
            || (!stages.is_empty() && !stages.contains(&story.stage))
            || open.is_some_and(|o| o == story.stage.is_closed())
            || meaning.is_some_and(|k| k != story.kind)
            || since_bar.is_some_and(|b| story.last_bar < b)
            || since_t.is_some_and(|t| story.history.last().is_none_or(|l| l.t < t))
            || born_since_bar.is_some_and(|b| story.born_bar < b)
            || born_since_t.is_some_and(|t| story.history.first().is_none_or(|h| h.t < t))
            || (*audience && !self.admits(doc, principal, story, story.stage)))
    }

    /// (R5/M19, K53) Whether `story` passes every filter of the `capsules` query `q` but
    /// the audience, and the audience keeps it out by the coin-flip rule alone.
    pub(crate) fn coin_flip_hidden_by(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        q: &qp_schema::protocol::Query,
    ) -> bool {
        let qp_schema::protocol::Query::Capsules {
            audience: true,
            instrument,
            stages,
            open,
            meaning,
            since_bar,
            since_t,
            born_since_bar,
            born_since_t,
            latest,
            limit,
        } = q
        else {
            return false;
        };
        if self.coin_flip_rule(principal).is_none() {
            return false;
        }
        let rest = qp_schema::protocol::Query::Capsules {
            audience: false,
            instrument: instrument.clone(),
            stages: stages.clone(),
            open: *open,
            meaning: *meaning,
            since_bar: *since_bar,
            since_t: *since_t,
            born_since_bar: *born_since_bar,
            born_since_t: *born_since_t,
            latest: *latest,
            limit: *limit,
        };
        self.capsule_passes(doc, principal, story, &rest)
            && self.feed_place(doc, principal, story, story.stage).1
                == Some(qp_schema::protocol::FeedHidden::CoinFlip)
    }

    /// (R4/K42-FEED) Whether `it`'s latest transition is at the latest final bar of its
    /// series: what became known in the current bar period (for a daily series, the
    /// current session).
    pub(crate) fn is_current(&self, it: &CapsuleItem) -> bool {
        [rd!(self), &self.doc]
            .into_iter()
            .find_map(|d| {
                let bs = bar_series(d, &it.series)?;
                let n = bs.bars()?.len();
                (n > 0).then(|| final_count(d, &bs.id, n).max(1) - 1)
            })
            .is_some_and(|last| usize::try_from(it.last_bar).is_ok_and(|b| b >= last))
    }

    /// The ledger rows with a key, retracted capsules left out: the engine's when the
    /// authority runs it (license-independent, K33: the same for every reader, also
    /// inside [`Session::as_reader`]), else the document's.
    pub(crate) fn ledger_now(&self) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
        match self.runner.as_ref() {
            Some(r) => {
                // (R3/K1) In the public world, the public-only engine's.
                let engine = match (self.in_public, r.pub_engine.as_ref()) {
                    (true, Some(e)) => e,
                    _ => &r.engine,
                };
                // An engine's ledger: no capsule of its own is retracted (RF1 S3.2: no
                // `Document` built for it).
                ledger_rows_of(&[], &engine.ledger())
            }
            None => ledger_rows(&self.doc),
        }
    }

    /// (R4/S1, §20.2, ergonomics C11) What a capsule selector `sel` may also have matched
    /// that `principal` cannot see: when the reader lacks the licensed flow of the
    /// instrument (the selector's, else the resolved capsule's) and the selector does not
    /// rule flow out, the server comments the reader already gets (`capsules`) that are
    /// newer than the resolved capsule's latest transition, with their entitlement. No new
    /// existence oracle: only what the comments told this reader already.
    pub(crate) fn withheld_for(
        &self,
        principal: &str,
        sel: &qp_schema::protocol::CapsuleSel,
        item: &CapsuleItem,
    ) -> Option<qp_schema::protocol::Withheld> {
        if sel
            .meaning
            .is_some_and(|m| m != qp_schema::capsule::MeaningKind::Flow)
        {
            return None;
        }
        let instrument = sel.instrument.as_deref().unwrap_or(&item.instrument);
        let comments: Vec<ServerComment> = self
            .comments_for(principal)
            .into_iter()
            .filter(|c| c.instrument == instrument && c.t > item.t)
            .collect();
        if comments.is_empty() {
            return None;
        }
        let mut requires: Vec<String> = comments
            .iter()
            .filter_map(|c| c.attachments.get("entitlement").cloned())
            .collect();
        requires.sort();
        requires.dedup();
        Some(qp_schema::protocol::Withheld { requires, comments })
    }
}
