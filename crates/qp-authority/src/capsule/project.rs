//! Capsule topic lines and reader projections of documents, changes and log entries.

use super::*;

impl Session {
    /// (Privacy, §25.5) Whether reader `reader` (a projection's principal; empty: the
    /// host) may receive intent `i`: an intent without a watch, the reader's own watch
    /// (an agent: its person's), or any watch for an `owner`. The rule of the document's
    /// projection: it holds for any document, whatever subsystems are registered.
    pub(crate) fn intent_for_reader(&self, i: &qp_schema::Intent, reader: &str) -> bool {
        let Some(w) = &i.watch else { return true };
        reader.is_empty()
            || w.for_ == qp_ops::watch::person_of(&self.plane, reader)
            || self
                .principal(reader)
                .is_some_and(|p| p.role == qp_schema::protocol::Role::Owner)
    }

    // -----------------------------------------------------------------------
    // Notifications
    // -----------------------------------------------------------------------

    /// Topic `capsules` notifications of one engine entry: per subscriber, the story
    /// transitions it may see and that pass its filter; server comments to subscribers
    /// without the flow entitlement. (R3/K1) A public-view reader's transitions are the
    /// public-only engine's (`events_pub`), read in the public world.
    pub(crate) fn capsule_topic_lines(
        &mut self,
        seq: Seq,
        events: &[qp_capsule::StoryEvent],
        events_pub: &[qp_capsule::StoryEvent],
        said: &[ServerComment],
    ) -> Vec<OutboundLine> {
        let mut lines = self.capsule_topic_lines_in(seq, events, said, false);
        // (R3/K1b) Under the public entry's own number (every look makes one).
        let pseq = self.pub_of(seq);
        if let (true, Some(pseq)) = (
            self.other.is_some() && self.conns.iter().any(|c| self.public_view(&c.principal)),
            pseq,
        ) {
            let p = self.in_world(true, |s| {
                s.capsule_topic_lines_in(pseq, events_pub, said, true)
            });
            lines.extend(p);
            lines.sort_by_key(|(ci, _)| *ci);
        }
        lines.into_iter().map(|(_, l)| l).collect()
    }

    #[allow(clippy::too_many_lines)]
    /// [`Session::capsule_topic_lines`] for the connections of the world the fields hold
    /// (`public`), each line with its connection index.
    fn capsule_topic_lines_in(
        &self,
        seq: Seq,
        events: &[qp_capsule::StoryEvent],
        said: &[ServerComment],
        public: bool,
    ) -> Vec<(usize, OutboundLine)> {
        let mut lines = Vec::new();
        for (ci, c) in self.conns.iter().enumerate() {
            if self.other.is_some() && self.public_view(&c.principal) != public {
                continue;
            }
            let subs: Vec<(&str, Option<&String>, bool)> = c
                .subs
                .iter()
                .filter_map(|s| match &s.topic {
                    Topic::Capsules { instrument, all } => {
                        Some((s.id.as_str(), instrument.as_ref(), *all))
                    }
                    _ => None,
                })
                .collect();
            let stories = c
                .subs
                .iter()
                .any(|s| matches!(s.topic, Topic::Stories { .. }));
            if subs.is_empty() && !stories {
                continue;
            }
            let p = self.proj_for(&c.principal);
            let doc = &self.doc;
            let lang = self.lang_of(ci);
            let mut memo = StudyMemo::default();
            for e in events.iter().filter(|_| !subs.is_empty()) {
                if p.as_ref().is_some_and(|p| {
                    p.hidden.contains(&e.capsule)
                        || (!e.transition.facts.is_empty()
                            && e.transition.facts.iter().all(|f| p.hidden.contains(f)))
                }) {
                    continue;
                }
                let Some(cap) = self.doc_capsule(&e.capsule) else {
                    continue;
                };
                let Some(mut item) =
                    self.capsule_item_with(doc, cap, p.as_deref(), lang, &c.principal, &mut memo)
                else {
                    continue;
                };
                if let Some(v) = c.look.voice.as_deref() {
                    let clock = c.look.clock;
                    item.voice =
                        self.reader_voice(doc, cap, p.as_deref(), lang, v, clock, &mut memo);
                }
                self.market_item(&mut item, lang, &c.principal);
                for (sub, inst, all) in &subs {
                    if inst.is_some_and(|i| i != &item.instrument) {
                        continue;
                    }
                    if !all {
                        let story = p
                            .as_ref()
                            .and_then(|p| p.stories.get(&cap.id))
                            .or(cap.story.as_ref());
                        if !story
                            .is_some_and(|s| self.admits(doc, &c.principal, s, e.transition.stage))
                        {
                            continue;
                        }
                    }
                    lines.push((
                        ci,
                        out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: (*sub).to_owned(),
                                event: Event::Capsule {
                                    capsule: Box::new(item.clone()),
                                    transition: e.transition.clone(),
                                    seq,
                                },
                            },
                        ),
                    ));
                }
            }
            for sc in said {
                if self.flow_ok(&c.principal, &sc.instrument) {
                    continue;
                }
                for (sub, inst, all) in &subs {
                    if inst.is_some_and(|i| i != &sc.instrument) {
                        continue;
                    }
                    if !all {
                        let a = self.audience_of(doc, &c.principal);
                        let watched = a.watchlist.is_empty()
                            || a.watchlist.contains(&sc.instrument)
                            || a.positions.contains_key(&sc.instrument);
                        if !watched {
                            continue;
                        }
                    }
                    lines.push((
                        ci,
                        out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: (*sub).to_owned(),
                                event: Event::ServerComment {
                                    comment: sc.clone(),
                                    seq,
                                },
                            },
                        ),
                    ));
                }
            }
            // (R5/M18, §23) The `stories` topic: server comments on its instrument.
            for sc in said {
                if self.flow_ok(&c.principal, &sc.instrument) {
                    continue;
                }
                for s in &c.subs {
                    let Topic::Stories { instrument, series } = &s.topic else {
                        continue;
                    };
                    if instrument != &sc.instrument
                        || series.as_ref().is_some_and(|x| x != &sc.series)
                    {
                        continue;
                    }
                    lines.push((
                        ci,
                        out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::ServerComment {
                                    comment: sc.clone(),
                                    seq,
                                },
                            },
                        ),
                    ));
                }
            }
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Projection of documents and changes
    // -----------------------------------------------------------------------

    /// `doc` as `p`'s principal may receive it.
    ///
    /// `pre`: the reader's sentences by capsule ([`Session::reader_sentences`]), when
    /// already written.
    pub(crate) fn project_doc(
        &self,
        doc: &Document,
        p: &Proj,
        lang: Language,
        pre: Option<&std::collections::HashMap<String, String>>,
    ) -> Document {
        let mut d = doc.clone();
        if !p.principal.is_empty() {
            // (R5B, §25.7) An agent that works for a person keeps the person's audience
            // (its holdings and watchlist make capsules relevant to it); nobody else's.
            let person = qp_ops::watch::person_of(&self.plane, &p.principal);
            d.audiences
                .retain(|k, _| k == &p.principal || k.as_str() == person);
        }
        d.facts.retain(|f| !p.hidden.contains(&f.id));
        for f in &mut d.facts {
            if p.redact.contains(&f.id) {
                f.evidence.clear();
                f.evidence_ref = None;
            }
        }
        d.findings.retain(|f| !p.hidden.contains(&f.id));
        for f in &mut d.findings {
            f.facts.retain(|x| !p.hidden.contains(x));
        }
        d.capsules.retain(|c| !p.hidden.contains(&c.id));
        // (R5B, §25.5) Another person's watches stay out.
        d.intents
            .retain(|i| self.intent_for_reader(i, &p.principal));
        // (R5B, §25.4) Comments with a hidden cite or with numbers stay out.
        d.comments.retain(|c| self.projected_comment_ok(c, p));
        let views_kept: Vec<qp_schema::View> = d
            .views
            .iter()
            .filter(|v| !p.mentions_hidden(*v))
            .cloned()
            .collect();
        d.views = views_kept;
        let view_ids: BTreeSet<String> = d.views.iter().map(|v| v.id.clone()).collect();
        for c in &mut d.capsules {
            if let Some(s) = p.stories.get(&c.id) {
                c.story = Some(s.clone());
            }
            if c.view.as_ref().is_some_and(|v| !view_ids.contains(v)) {
                c.view = None;
            }
        }
        // Sentences written for the reader where the stored one does not fit it.
        let mut memo = StudyMemo::default();
        let rewrites: Vec<(usize, String)> = d
            .capsules
            .iter()
            .enumerate()
            .filter(|(_, c)| p.rewrite.contains(&c.id))
            .filter_map(|(i, c)| match pre {
                Some(m) => m.get(&c.id).map(|t| (i, t.clone())),
                None => self
                    .reader_sentence_with(&d, c, Some(p), lang, &mut memo)
                    .map(|(s, _)| (i, s.text)),
            })
            .collect();
        // (R4/T6) The first finding of each id, looked up once (a linear search per
        // rewritten capsule was most of a public reader's first `hello`).
        let mut finding_at: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(d.findings.len());
        for (k, f) in d.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(k);
        }
        let targets: Vec<(usize, Option<usize>)> = rewrites
            .iter()
            .map(|(i, _)| (*i, finding_at.get(d.capsules[*i].finding.as_str()).copied()))
            .collect();
        for ((i, k), (_, text)) in targets.into_iter().zip(rewrites) {
            d.capsules[i].headline.clone_from(&text);
            if let Some(k) = k {
                d.findings[k].claim = text;
            }
        }
        d.ledger.entries.retain(|e| {
            !p.hidden.contains(&e.finding)
                && e.capsule.as_ref().is_none_or(|c| !p.hidden.contains(c))
        });
        d.narratives.retain(|n| !p.mentions_hidden(n));
        d.proposals.retain(|x| !p.mentions_hidden(x));
        d
    }

    #[allow(clippy::too_many_lines)]
    /// `changes` as `p`'s principal may receive them.
    pub(crate) fn project_changes(
        &self,
        changes: &[ChangeOp],
        p: &Proj,
        lang: Language,
    ) -> Vec<ChangeOp> {
        let mut outv = Vec::new();
        for c in changes {
            match c {
                ChangeOp::PutFact { fact, index } => {
                    if p.hidden.contains(&fact.id) {
                        continue;
                    }
                    let mut f = fact.clone();
                    if p.redact.contains(&f.id) {
                        f.evidence.clear();
                        f.evidence_ref = None;
                    }
                    outv.push(ChangeOp::PutFact {
                        fact: f,
                        index: *index,
                    });
                }
                ChangeOp::PutFinding { finding, index } => {
                    if p.hidden.contains(&finding.id) {
                        continue;
                    }
                    let mut f = finding.clone();
                    f.facts.retain(|x| !p.hidden.contains(x));
                    if let Some(cap) = self.doc_capsule_of(&f.id) {
                        if p.rewrite.contains(&cap.id) {
                            if let Some((s, _)) =
                                self.reader_sentence(&self.doc, cap, Some(p), lang)
                            {
                                f.claim = s.text;
                            }
                        }
                    }
                    outv.push(ChangeOp::PutFinding {
                        finding: f,
                        index: *index,
                    });
                }
                ChangeOp::PutCapsule { capsule, index } => {
                    if p.hidden.contains(&capsule.id) {
                        continue;
                    }
                    let mut cap = capsule.clone();
                    if let Some(st) = cap.story.as_ref() {
                        let ids = story_fact_ids(st);
                        let hidden: BTreeSet<String> = ids
                            .iter()
                            .filter(|i| p.hidden.contains(*i))
                            .cloned()
                            .collect();
                        if !hidden.is_empty() {
                            let facts: Vec<Fact> = ids
                                .iter()
                                .filter_map(|i| self.doc_fact(i).cloned())
                                .collect();
                            cap.story = Some(project_story(st, &hidden, &facts));
                        }
                    }
                    if p.rewrite.contains(&cap.id) {
                        if let Some((s, _)) = self.reader_sentence(&self.doc, &cap, Some(p), lang) {
                            cap.headline = s.text;
                        }
                    }
                    outv.push(ChangeOp::PutCapsule {
                        capsule: cap,
                        index: *index,
                    });
                }
                ChangeOp::SetCapsuleState { capsule, .. } if p.hidden.contains(capsule) => {}
                ChangeOp::PutAudience { principal, .. }
                    if !p.principal.is_empty() && principal != &p.principal => {}
                ChangeOp::Delete { id } if p.hidden.contains(id) => {}
                ChangeOp::PutComment { comment, .. } if !self.projected_comment_ok(comment, p) => {}
                ChangeOp::PutIntent { intent, .. }
                    if !self.intent_for_reader(intent, &p.principal) => {}
                ChangeOp::PushLedger { entry }
                    if p.hidden.contains(&entry.finding)
                        || entry.capsule.as_ref().is_some_and(|c| p.hidden.contains(c)) => {}
                ChangeOp::PutView { .. }
                | ChangeOp::InsertLayer { .. }
                | ChangeOp::PutLayer { .. }
                | ChangeOp::SetEmphasis { .. }
                | ChangeOp::PutNarrative { .. }
                | ChangeOp::PutProposal { .. }
                    if p.mentions_hidden(c) => {}
                other => outv.push(other.clone()),
            }
        }
        outv
    }

    /// A log entry as `p`'s principal may receive it: changes and inverse projected,
    /// request ops and a summary that name a hidden item removed.
    pub(crate) fn project_entry(&self, e: &LogEntry, p: &Proj, lang: Language) -> LogEntry {
        let mut x = e.clone();
        x.changes = self.project_changes(&e.changes, p, lang);
        x.inverse = self.project_changes(&e.inverse, p, lang);
        x.request.retain(|op| !p.mentions_hidden(op));
        // (R5B, §25.5) Another person's private requests (watches: `qp-watch`) stay out.
        let author_person = qp_ops::watch::person_of(&self.plane, &e.author).to_owned();
        let reader_ok = p.principal.is_empty()
            || qp_ops::watch::person_of(&self.plane, &p.principal) == author_person
            || self
                .principal(&p.principal)
                .is_some_and(|q| q.role == qp_schema::protocol::Role::Owner);
        x.request
            .retain(|op| reader_ok || !self.registry.op(op).is_some_and(|h| h.reader_private()));
        // (R5B, §25.4; audit B1, fail-closed) A restricted reader (hidden, redacted or
        // rewritten items) never gets an `add_comment` request: the stored comment, which
        // passes its own check (`projected_comment_ok`), is what it may read.
        let restricted = !(p.hidden.is_empty() && p.redact.is_empty() && p.rewrite.is_empty());
        x.request
            .retain(|op| !(restricted && matches!(op, qp_schema::protocol::Op::AddComment { .. })));
        // Another person's capsule preferences (positions, watch list, level) never go.
        let own = |w: &Option<String>| {
            p.principal.is_empty() || w.as_deref().unwrap_or(e.author.as_str()) == p.principal
        };
        let before = x.request.len();
        x.request.retain(|op| match op {
            qp_schema::protocol::Op::SetAudience { principal, .. }
            | qp_schema::protocol::Op::SetSensitivity { principal, .. } => own(principal),
            _ => true,
        });
        let foreign = !p.principal.is_empty()
            && e.changes.iter().any(
                |c| matches!(c, ChangeOp::PutAudience { principal, .. } if principal != &p.principal),
            );
        if foreign || x.request.len() != before {
            x.summary = "set_audience".to_owned();
        }
        if p.hidden
            .iter()
            .any(|h| h.len() <= x.summary.len() && x.summary.contains(h.as_str()))
        {
            x.summary = "…".to_owned();
        }
        if let Some(c) = x.cause.as_ref() {
            if p.hidden.iter().any(|h| c.contains(h.as_str())) {
                x.cause = None;
            }
        }
        x
    }

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    /// Runs `f` with the document (and the reading document) as `principal` may receive
    /// them (R3, §20): every read path, selector and error suggestion then only sees
    /// what the principal may see. Nothing written inside `f` may touch the document.
    ///
    /// (R3/perf) What it gives a reader (headlines, projected documents) is kept per
    /// reader and language ([`qp_plane::capidx::ReaderView`]) and given again while the log
    /// position, the projection and the reading document's bars are the same.
    pub(crate) fn as_reader<R>(
        &mut self,
        principal: &str,
        lang: Language,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        // (R3/K1) A public-view reader reads the public world. Its headlines are brought
        // up to date here, when a reader needs them (not after every engine entry).
        let public = self.public_view(principal);
        if public != self.in_public && self.other.is_some() {
            return self.in_world(public, |s| s.as_reader(principal, lang, f));
        }
        if self.in_public && self.proj.is_none() && !self.headlines_current() {
            self.refresh_headlines();
        }
        let p = self.reader_proj(principal);
        let lines = self.runner.is_some() || lang == Language::En;
        if p.is_none() && !lines {
            return f(self);
        }
        let nested = self.proj.is_some();
        let cached = if nested {
            None
        } else {
            self.views
                .iter()
                .position(|v| v.principal == principal && v.lang == lang)
                .map(|i| self.views.remove(i))
                .filter(|v| self.view_valid(v, p.as_ref(), lines))
        };
        let mut view = if let Some(v) = cached {
            v
        } else {
            let (heads, pre) = if lines {
                self.reader_sentences(p.as_deref(), lang, !nested)
            } else {
                (BTreeMap::new(), None)
            };
            let (doc, rdp) = match p.as_deref() {
                Some(p) => (
                    Some(self.project_doc(&self.doc, p, lang, pre.as_ref())),
                    self.rd
                        .as_ref()
                        .map(|r| self.project_doc(r, p, lang, pre.as_ref())),
                ),
                None => (None, None),
            };
            qp_plane::capidx::ReaderView {
                principal: principal.to_owned(),
                lang,
                log_len: self.log.len(),
                lines,
                proj: p.clone(),
                rd: if nested {
                    None
                } else {
                    qp_plane::capidx::rd_snap(self.rd.as_ref())
                },
                heads: Some(heads),
                doc,
                rdp,
            }
        };
        let heads = view.heads.take().unwrap_or_default();
        let saved_heads = std::mem::replace(&mut self.headlines, heads);
        let r = if let Some(p) = p {
            let doc = view.doc.take().expect("a projection has its document");
            let rd = view.rdp.take();
            let mut real_doc = std::mem::replace(&mut self.doc, doc);
            let real_rd = std::mem::replace(&mut self.rd, rd);
            let vr = self.view_reads.take();
            let li = self.live_index.take();
            let saved = self.proj.replace(p);
            // The real facts, moved (not copied) for the sentences.
            let real_facts =
                std::mem::replace(&mut self.real_facts, std::mem::take(&mut real_doc.facts));
            let r = f(self);
            real_doc.facts = std::mem::replace(&mut self.real_facts, real_facts);
            self.proj = saved;
            view.doc = Some(std::mem::replace(&mut self.doc, real_doc));
            view.rdp = std::mem::replace(&mut self.rd, real_rd);
            self.view_reads = vr;
            self.live_index = li;
            r
        } else {
            f(self)
        };
        view.heads = Some(std::mem::replace(&mut self.headlines, saved_heads));
        if !nested {
            self.views.push(view);
            if self.views.len() > qp_plane::capidx::VIEWS {
                self.views.remove(0);
            }
        }
        r
    }

    /// Whether `v` is what [`Session::as_reader`] would make now for projection `p`.
    fn view_valid(
        &self,
        v: &qp_plane::capidx::ReaderView,
        p: Option<&Rc<Proj>>,
        lines: bool,
    ) -> bool {
        v.log_len == self.log.len()
            && v.lines == lines
            && v.heads.is_some()
            && match (v.proj.as_ref(), p) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b) || **a == **b,
                _ => false,
            }
            && (p.is_none() || (v.doc.is_some() && v.rdp.is_some() == self.rd.is_some()))
            && qp_plane::capidx::rd_same(&v.rd, self.rd.as_ref())
    }
}
