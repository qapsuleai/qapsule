//! Reader access and text: entitlements, projections, facts and capsules by id, reader
//! sentences, voices and headline parts.

use super::*;

impl Session {
    /// `true` when `principal` may read the licensed flow of `instrument` (or there is
    /// none).
    pub(crate) fn flow_ok(&self, principal: &str, instrument: &str) -> bool {
        let Some(r) = self.runner.as_ref() else {
            return true;
        };
        match r.flow_feed(instrument) {
            Some(k) => self.plane.entitled(principal, &k),
            None => true,
        }
    }

    /// `true` when `principal` may read the bars of series `id` (inline: always).
    pub(crate) fn bars_ok(&self, principal: &str, id: &str) -> bool {
        let doc = rd!(self);
        let Some(bs) = bar_series(doc, id).or_else(|| find_series(doc, id)) else {
            return true;
        };
        match bs.binding() {
            Some(b) => self.plane.entitled(principal, &b.feed_key()),
            None => true,
        }
    }

    /// What `principal` may receive of the engine's items; `None` when everything.
    ///
    /// (R3/perf) Kept per reader and capsule ([`qp_plane::capidx::ReaderProj`]): brought up to
    /// date from the log entries since the last call (the capsules they put, and those
    /// naming the facts they put), rebuilt when a series is put, anything is deleted or an
    /// entitlement answer it was made with changed. Inside [`Session::as_reader`] (a
    /// projected document) and when its per-capsule rule does not hold, it is
    /// [`Session::proj_scratch`].
    pub(crate) fn proj_for(&self, principal: &str) -> Option<Rc<Proj>> {
        self.runner.as_ref()?;
        if self.proj.is_some() {
            return self.proj_scratch(principal).map(Rc::new);
        }
        let mut all = self.projs.borrow_mut();
        let rp = all
            .entry(principal.to_owned())
            .or_insert_with(|| qp_plane::capidx::ReaderProj::new(principal));
        let fresh = rp.built
            && !rp.degenerate
            && rp.log_len <= self.log.len()
            && !qp_plane::capidx::proj_breaks(&self.log[rp.log_len..])
            && self.entitlements_hold(principal, rp);
        if !fresh {
            *rp = qp_plane::capidx::ReaderProj::new(principal);
            for c in &self.doc.capsules {
                if c.story.is_none() {
                    continue;
                }
                if rp.contains(&c.id) {
                    rp.degenerate = true;
                    break;
                }
                let x = self.contrib(principal, c, rp);
                rp.add(&c.id, x);
            }
            rp.built = true;
        } else if rp.log_len < self.log.len() {
            let mut caps: BTreeSet<String> = BTreeSet::new();
            for e in &self.log[rp.log_len..] {
                for ch in &e.changes {
                    match ch {
                        ChangeOp::PutCapsule { capsule, .. } => {
                            caps.insert(capsule.id.clone());
                        }
                        ChangeOp::PutFact { fact, .. } => {
                            caps.extend(rp.capsules_of_fact(&fact.id));
                        }
                        _ => {}
                    }
                }
            }
            for id in caps {
                rp.remove(&id);
                if let Some(c) = self.doc_capsule(&id).filter(|c| c.story.is_some()) {
                    let x = self.contrib(principal, c, rp);
                    rp.add(&id, x);
                }
            }
        }
        rp.log_len = self.log.len();
        if rp.degenerate {
            drop(all);
            return self.proj_scratch(principal).map(Rc::new);
        }
        rp.result()
    }

    /// Whether the entitlement answers `rp` was made with still hold.
    fn entitlements_hold(&self, principal: &str, rp: &qp_plane::capidx::ReaderProj) -> bool {
        rp.flow
            .values()
            .chain(rp.bars.values())
            .all(|(k, a)| k.as_ref().is_none_or(|k| self.plane.entitled(principal, k)) == *a)
    }

    /// [`Session::flow_ok`], recorded in `rp`.
    fn flow_ok_in(
        &self,
        principal: &str,
        instrument: &str,
        rp: &mut qp_plane::capidx::ReaderProj,
    ) -> bool {
        if let Some((_, a)) = rp.flow.get(instrument) {
            return *a;
        }
        let k = self.runner.as_ref().and_then(|r| r.flow_feed(instrument));
        let a = k.as_ref().is_none_or(|k| self.plane.entitled(principal, k));
        rp.flow.insert(instrument.to_owned(), (k, a));
        a
    }

    /// [`Session::bars_ok`], recorded in `rp`.
    fn bars_ok_in(&self, principal: &str, id: &str, rp: &mut qp_plane::capidx::ReaderProj) -> bool {
        if let Some((_, a)) = rp.bars.get(id) {
            return *a;
        }
        let doc = rd!(self);
        let k = bar_series(doc, id)
            .or_else(|| find_series(doc, id))
            .and_then(qp_schema::Series::binding)
            .map(|b| b.feed_key());
        let a = k.as_ref().is_none_or(|k| self.plane.entitled(principal, k));
        rp.bars.insert(id.to_owned(), (k, a));
        a
    }

    /// What capsule `c` (with a story) adds to `principal`'s projection: the steps of
    /// [`Session::proj_scratch`] for one capsule.
    fn contrib(
        &self,
        principal: &str,
        c: &Capsule,
        rp: &mut qp_plane::capidx::ReaderProj,
    ) -> qp_plane::capidx::Contrib {
        let mut x = qp_plane::capidx::Contrib::default();
        let Some(story) = c.story.as_ref() else {
            return x;
        };
        let ids = story_fact_ids(story);
        x.shown = ids
            .iter()
            .filter(|i| {
                self.doc_fact(i).is_some_and(|f| {
                    !is_licensed(f)
                        || self.flow_ok_in(
                            principal,
                            f.meaning.as_ref().map_or("", |m| m.instrument.as_str()),
                            rp,
                        )
                })
            })
            .cloned()
            .collect();
        x.facts.clone_from(&ids);
        let Some(lead) = ids.first().and_then(|i| self.doc_fact(i)) else {
            return x;
        };
        let flow_ok = self.flow_ok_in(principal, &story.instrument, rp);
        if is_licensed(lead) && !flow_ok {
            x.full = true;
            x.hidden.push(c.id.clone());
            x.hidden.push(c.finding.clone());
            x.hidden.extend(ids.iter().cloned());
            return x;
        }
        let bars_ok = self.bars_ok_in(principal, &story.series, rp);
        let mut hidden_here = BTreeSet::new();
        for i in &ids {
            if let Some(f) = self.doc_fact(i) {
                if is_licensed(f) && !flow_ok {
                    hidden_here.insert(i.clone());
                } else if !bars_ok && f.meaning.is_some() {
                    x.redact.push(i.clone());
                }
            }
        }
        if !bars_ok {
            x.rewrite = true;
        }
        if !hidden_here.is_empty() {
            let story_facts: Vec<Fact> = ids
                .iter()
                .filter_map(|i| self.doc_fact(i).cloned())
                .collect();
            x.story = Some(project_story(story, &hidden_here, &story_facts));
            x.rewrite = true;
            x.hidden.extend(hidden_here);
        }
        x
    }

    /// The reference computation of [`Session::proj_for`], over the document as it is.
    pub(crate) fn proj_scratch(&self, principal: &str) -> Option<Proj> {
        self.runner.as_ref()?;
        let doc = &self.doc;
        let mut p = Proj {
            principal: principal.to_owned(),
            ..Proj::default()
        };
        let facts: BTreeMap<&str, &Fact> = doc.facts.iter().map(|f| (f.id.as_str(), f)).collect();
        for c in &doc.capsules {
            let Some(story) = c.story.as_ref() else {
                continue;
            };
            let ids = story_fact_ids(story);
            let Some(lead) = ids.first().and_then(|i| facts.get(i.as_str())) else {
                continue;
            };
            let flow_ok = self.flow_ok(principal, &story.instrument);
            if is_licensed(lead) && !flow_ok {
                p.hidden.insert(c.id.clone());
                p.hidden.insert(c.finding.clone());
                for i in &ids {
                    p.hidden.insert(i.clone());
                }
                continue;
            }
            let bars_ok = self.bars_ok(principal, &story.series);
            let mut hidden_here = BTreeSet::new();
            for i in &ids {
                if let Some(f) = facts.get(i.as_str()) {
                    if is_licensed(f) && !flow_ok {
                        hidden_here.insert(i.clone());
                    } else if !bars_ok && f.meaning.is_some() {
                        p.redact.insert(i.clone());
                    }
                }
            }
            if !bars_ok {
                p.rewrite.insert(c.id.clone());
            }
            if !hidden_here.is_empty() {
                let story_facts: Vec<Fact> = ids
                    .iter()
                    .filter_map(|i| facts.get(i.as_str()).map(|f| (*f).clone()))
                    .collect();
                p.stories.insert(
                    c.id.clone(),
                    project_story(story, &hidden_here, &story_facts),
                );
                p.rewrite.insert(c.id.clone());
                p.hidden.extend(hidden_here);
            }
        }
        // A fact hidden in one story but shown in another stays shown there.
        let shown: BTreeSet<String> = doc
            .capsules
            .iter()
            .filter(|c| !p.hidden.contains(&c.id))
            .filter_map(|c| c.story.as_ref())
            .flat_map(story_fact_ids)
            .filter(|i| {
                facts.get(i.as_str()).is_some_and(|f| {
                    !is_licensed(f)
                        || self.flow_ok(
                            principal,
                            f.meaning.as_ref().map_or("", |m| m.instrument.as_str()),
                        )
                })
            })
            .collect();
        p.hidden.retain(|h| !shown.contains(h));
        if p.hidden.is_empty() && p.redact.is_empty() {
            return None;
        }
        Some(p)
    }

    /// The document's facts with their values (inside [`Session::as_reader`] the
    /// document is projected; sentences are still computed from the real facts and
    /// redacted afterwards).
    pub(crate) fn all_facts(&self) -> &[Fact] {
        if self.proj.is_some() {
            &self.real_facts
        } else {
            &self.doc.facts
        }
    }

    /// What `principal` may receive: [`Session::proj_for`], or — when another person's
    /// capsule preferences exist (or ever existed) — a projection that only removes
    /// those (positions and watch lists are personal: a reader gets its own audience
    /// only, in every document, change and log entry, owners included). `None`: the
    /// stored items go as they are.
    pub(crate) fn reader_proj(&self, principal: &str) -> Option<Rc<Proj>> {
        if let Some(p) = self.proj_for(principal) {
            if self.is_host(principal) {
                let mut p = (*p).clone();
                p.principal = String::new();
                return Some(Rc::new(p));
            }
            return Some(p);
        }
        if self.is_host(principal) {
            return None;
        }
        self.audience_seen.iter().any(|k| k != principal).then(|| {
            Rc::new(Proj {
                principal: principal.to_owned(),
                ..Proj::default()
            })
        })
    }

    /// Fact `id` among [`Session::all_facts`] (the first with that id).
    pub(crate) fn fact_by_id(&self, id: &str) -> Option<&Fact> {
        let facts = self.all_facts();
        let mut ix = self.fact_ix.borrow_mut();
        ix.refresh(&self.log, facts);
        ix.get(facts, id).map(|i| &facts[i])
    }

    /// Fact `id` of `self.doc` (inside [`Session::as_reader`]: the projected one).
    pub(crate) fn doc_fact(&self, id: &str) -> Option<&Fact> {
        if self.proj.is_some() {
            return self.doc.facts.iter().find(|f| f.id == id);
        }
        self.fact_by_id(id)
    }

    /// Capsule `id` of `self.doc`.
    pub(crate) fn doc_capsule(&self, id: &str) -> Option<&Capsule> {
        let caps = &self.doc.capsules;
        if self.proj.is_some() {
            return caps.iter().find(|c| c.id == id);
        }
        let mut ix = self.cap_ix.borrow_mut();
        ix.refresh(&self.log, caps);
        ix.get(caps, id).map(|i| &caps[i])
    }

    /// The first capsule of finding `f` in `self.doc`.
    pub(crate) fn doc_capsule_of(&self, f: &str) -> Option<&Capsule> {
        let caps = &self.doc.capsules;
        if self.proj.is_some() {
            return caps.iter().find(|c| c.finding == f);
        }
        let mut ix = self.cap_ix.borrow_mut();
        ix.refresh(&self.log, caps);
        ix.by_finding(caps, f).map(|i| &caps[i])
    }

    /// The host (the authority's operator, which holds the store anyway) receives every
    /// person's capsule preferences; nobody else, owners included, gets another's.
    fn is_host(&self, principal: &str) -> bool {
        self.principal(principal)
            .is_some_and(|p| p.role == qp_schema::protocol::Role::Host)
    }

    /// The language of connection `conn`'s capsule sentences.
    pub(crate) fn lang_of(&self, ci: usize) -> Language {
        self.conns[ci].surface.lang.unwrap_or_default()
    }

    /// Index of the last final bar of the story's series in `doc` (the sentences' "now").
    pub(super) fn now_bar<'d>(doc: &'d Document, story: &Story) -> Option<(u64, &'d [Bar])> {
        let bs = bar_series(doc, &story.series)?;
        let bars = bs.bars()?;
        let n = bars.len();
        if n == 0 {
            return None;
        }
        let f = final_count(doc, &bs.id, n).max(1);
        let now = u64::try_from(f - 1).unwrap_or(0).max(story.last_bar);
        Some((now.min(u64::try_from(n - 1).unwrap_or(0)), bars))
    }

    /// The sentences of capsule `c` (with story) for a reader with projection `p`
    /// (`None`: entitled to everything), over the facts of `doc`.
    pub(crate) fn reader_sentence(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
    ) -> Option<(CapsuleSentence, bool)> {
        self.reader_sentence_with(doc, c, p, lang, &mut StudyMemo::default())
    }

    /// [`Session::reader_sentence`] with the study values of one pass over unchanged
    /// bars (`memo`) and the session's sentence memo.
    pub(crate) fn reader_sentence_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<(CapsuleSentence, bool)> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        let (tr, en) = sentence_pair_memo(
            story,
            &facts,
            bars,
            now,
            redacted,
            memo,
            Some((&c.id, &self.sentences)),
            None,
        )?;
        let text = match lang {
            Language::Tr => tr.clone(),
            Language::En => en.clone(),
        };
        Some((CapsuleSentence { text, tr, en }, redacted))
    }

    /// The text of [`Session::reader_sentence_with`] only (one language written).
    pub(crate) fn reader_text_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<String> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        let (tr, en) = sentence_pair_memo(
            story,
            &facts,
            bars,
            now,
            redacted,
            memo,
            Some((&c.id, &self.sentences)),
            Some(match lang {
                Language::Tr => Lang::Tr,
                Language::En => Lang::En,
            }),
        )?;
        Some(match lang {
            Language::Tr => tr,
            Language::En => en,
        })
    }

    /// (R4, K42/A28) Validity of capsule `c` for a reader with projection `p`, as of the
    /// same bar and over the same facts as [`Session::reader_sentence_with`].
    pub(crate) fn reader_validity(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        memo: &mut StudyMemo,
    ) -> Option<qp_schema::capsule::Validity> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        Some(story_validity(story, &facts, bars, now, memo))
    }

    /// (R4/H-FP) The headline pieces of capsule `c` for a reader with projection `p` in
    /// `lang` ([`crate::headparts`]): the same story, facts, "now", validity and
    /// redaction as [`Session::reader_text_with`] writes the reader's sentence with.
    pub(crate) fn reader_parts(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<qp_grammar::HeadlineParts> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars, bs) =
            Self::now_series(rd!(self), story).or_else(|| Self::now_series(doc, story))?;
        let validity = story_validity(story, &facts, bars, now, memo);
        qp_plane::headparts::story_parts(
            story,
            (bs, bars, now),
            validity,
            redacted,
            lang,
            self.headline_cal.as_ref(),
        )
    }

    /// (R5/m17-int, `docs/LANGUAGE.md`) The surfaces of capsule `c` in voice `voice` for a
    /// reader with projection `p` in `lang`: the same story, facts, "now", validity and
    /// redaction as [`Session::reader_parts`] (the voice's pieces are those parts, A33).
    /// `None` under `r4`, or without the story's bars or a known period.
    #[allow(clippy::too_many_arguments)] // (RP4/T7) the connection's `clock`
    pub(crate) fn reader_voice(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        voice: &str,
        clock: bool,
        memo: &mut StudyMemo,
    ) -> Option<qp_schema::protocol::CapsuleVoice> {
        if !qp_voice::is_voice(voice) || voice == qp_voice::DEFAULT_VOICE {
            return None;
        }
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars, bs) =
            Self::now_series(rd!(self), story).or_else(|| Self::now_series(doc, story))?;
        let validity = story_validity(story, &facts, bars, now, memo);
        qp_plane::headparts::story_voice(
            voice,
            story,
            &facts,
            (bs, bars, now),
            validity,
            redacted,
            lang,
            self.headline_cal.as_ref(),
            clock.then_some(doc.display_tz.as_deref()),
            memo,
        )
    }

    /// (RP4/T7) The time stamp of story row `row` for a reader in `lang` (the story chart's
    /// rows): its event's time (birth, or close once closed) as of the series' last bar,
    /// in the document's display zone (else the exchange's); `None` for a redacted row
    /// (K33) or without the series' bars.
    #[must_use]
    pub fn row_stamp(&self, row: &qp_schema::protocol::StoryRow, lang: Language) -> Option<String> {
        if row.redacted {
            return None;
        }
        let (doc, bs) = [rd!(self), &self.doc]
            .into_iter()
            .find_map(|d| bar_series(d, &row.series).map(|bs| (d, bs)))?;
        qp_plane::headparts::event_stamp(
            row.closed_t.unwrap_or(row.born_t),
            bs,
            bs.bars()?,
            lang,
            self.headline_cal.as_ref(),
            doc.display_tz.as_deref(),
        )
    }

    /// (RP4/T6) The `capsules` item memo's lookups so far: (reused, written).
    #[must_use]
    pub fn item_memo_counts(&self) -> (u64, u64) {
        let m = self.item_memo.borrow();
        (m.hits, m.misses)
    }

    /// (RP4/T6) Forgets every memoized `capsules` item (the next answers are written from
    /// scratch): the "from scratch" side of the incremental = from scratch tests.
    pub fn forget_item_memo(&self) {
        self.item_memo.borrow_mut().clear();
    }

    /// [`Session::now_bar`] with the bar series it read.
    fn now_series<'d>(
        doc: &'d Document,
        story: &Story,
    ) -> Option<(u64, &'d [Bar], &'d qp_schema::Series)> {
        let (now, bars) = Self::now_bar(doc, story)?;
        Some((now, bars, bar_series(doc, &story.series)?))
    }

    /// (R4/H-FP) The headline pieces of the capsule of `item` (the authority's `capsules`
    /// answer for this reader) in `lang`, over this session's own story and bars: what the
    /// shell's replica gives a face design's headline template. Validity and redaction are
    /// the item's (the authority's, K42/A28, K33); "now" is the last final bar of the
    /// story's series, as for the sentence. `None` without the capsule's story or bars
    /// (an unentitled reader's replica has no bars; its face is text anyway).
    #[must_use]
    pub fn item_headline_parts(
        &self,
        item: &qp_schema::protocol::CapsuleItem,
        lang: Language,
    ) -> Option<qp_grammar::HeadlineParts> {
        let caps = &rd!(self).capsules;
        let c = caps
            .iter()
            .find(|c| c.id == item.id)
            .or_else(|| caps.iter().find(|c| c.finding == item.finding))
            .or_else(|| self.doc.capsules.iter().find(|c| c.id == item.id))?;
        let story = c.story.as_ref()?;
        let (now, bars, bs) =
            Self::now_series(rd!(self), story).or_else(|| Self::now_series(&self.doc, story))?;
        qp_plane::headparts::story_parts(
            story,
            (bs, bars, now),
            item.validity,
            item.redacted,
            lang,
            self.headline_cal.as_ref(),
        )
    }

    /// (R4/H-FP) The exchange calendar of headline pieces for bar series whose instrument
    /// names none (`None`: the built-in `XIST`); see [`crate::headparts`]. A replay host
    /// sets [`qp_plane::headparts::replay_calendar`] of its source.
    pub fn set_headline_calendar(&mut self, cal: Option<qp_schema::Calendar>) {
        self.headline_cal = cal;
    }
}
