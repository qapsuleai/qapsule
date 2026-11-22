//! Capsule, evidence and ledger queries.

use super::*;

impl Session {
    #[allow(clippy::too_many_lines)]
    /// Answer to the `capsules` query (run on the asker's projected document).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capsules_query(
        &self,
        ci: usize,
        q: &qp_schema::protocol::Query,
    ) -> PResult<QueryResult> {
        let qp_schema::protocol::Query::Capsules {
            instrument,
            since_bar,
            since_t,
            latest,
            audience,
            limit,
            ..
        } = q
        else {
            unreachable!("capsules query");
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let voice = self.conns[ci].look.voice.clone();
        let clock = self.conns[ci].look.clock;
        let p = self.proj.as_deref();
        let doc = &self.doc;
        let mut items: Vec<CapsuleItem> = Vec::new();
        let mut memo = StudyMemo::default();
        // (R4) `latest`: the most recently born match only (birth time, bar, id).
        let mut newest: Option<(i64, u64, &str)> = None;
        if *latest {
            for c in &doc.capsules {
                let Some(story) = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref()) else {
                    continue;
                };
                if !self.capsule_passes(doc, &principal, story, q) {
                    continue;
                }
                let key = (
                    story.history.first().map_or(i64::MIN, |h| h.t),
                    story.born_bar,
                    c.id.as_str(),
                );
                if newest.is_none_or(|n| key > n) {
                    newest = Some(key);
                }
            }
        }
        let mut coin_flip_hidden = 0u32;
        let mut series_at: std::collections::HashMap<&str, Option<&qp_schema::Series>> =
            std::collections::HashMap::new();
        for c in &doc.capsules {
            if *latest && newest.is_none_or(|n| n.2 != c.id) {
                continue;
            }
            let Some(story) = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref()) else {
                continue;
            };
            if !self.capsule_passes(doc, &principal, story, q) {
                if !*latest && self.coin_flip_hidden_by(doc, &principal, story, q) {
                    coin_flip_hidden += 1;
                }
                continue;
            }
            let compute = |memo: &mut StudyMemo| {
                let mut it = self.capsule_item_with(doc, c, p, lang, &principal, memo)?;
                // (R5/m17-int) The connection's voice speaks every listed capsule.
                if let Some(v) = voice.as_deref() {
                    it.voice = self.reader_voice(doc, c, p, lang, v, clock, memo);
                }
                Some(it)
            };
            // (RP4/T6) An entitled reader's item is written again only when its input
            // changed (`ItemMemo`); the reader's own fields are applied every time.
            let base = if p.is_none() {
                let series = series_at
                    .entry(story.series.as_str())
                    .or_insert_with(|| self.item_series(doc, &story.series));
                let fact_ids = story_fact_ids(story);
                let facts: Vec<Option<&Fact>> =
                    fact_ids.iter().map(|i| self.fact_by_id(i)).collect();
                let key = crate::capidx::ItemKeyRef {
                    capsule: c,
                    facts: &facts,
                    series: *series,
                    lang,
                    voice: voice.as_deref(),
                    clock,
                    cal: self.headline_cal.as_ref(),
                    tz: doc.display_tz.as_deref(),
                };
                let got = self.item_memo.borrow_mut().get(&key);
                match got {
                    Some(v) => v.map(|mut it| {
                        it.relevant = self.relevant(doc, &principal, &story.instrument);
                        it.coin_flip = self.coin_flip_of(&principal, story);
                        it
                    }),
                    None => {
                        let v = compute(&mut memo);
                        self.item_memo.borrow_mut().put(&key, v.clone());
                        v
                    }
                }
            } else {
                compute(&mut memo)
            };
            if let Some(mut it) = base {
                // (R6/T3, §24) The market note, when the host gave a market status.
                self.market_item(&mut it, lang, &principal);
                items.push(it);
            }
        }
        if p.is_none() {
            self.item_memo.borrow_mut().prune(
                doc.capsules.iter().map(|c| c.id.as_str()),
                doc.capsules.len(),
            );
        }
        items.sort_by(|a, b| {
            b.importance
                .total_cmp(&a.importance)
                .then(b.t.cmp(&a.t))
                .then(a.id.cmp(&b.id))
        });
        if let Some(l) = limit {
            let l = *l as usize;
            if *audience && items.len() > l {
                // (R4/K42-FEED) `limit` does not cut a current negative transition of the
                // asker's position/intent story (K42/A22: it reaches the person whatever
                // the level; the page size must not undo that). Kept: those first, then
                // the rest by rank; the answer stays in rank order and at most `limit`.
                let keep: Vec<bool> = items
                    .iter()
                    .map(|it| it.relevant && it.stage.is_negative() && self.is_current(it))
                    .collect();
                let mut room = l.saturating_sub(keep.iter().filter(|k| **k).count());
                let mut take = Vec::with_capacity(l);
                for (it, k) in items.into_iter().zip(keep) {
                    if k || room > 0 {
                        if !k {
                            room -= 1;
                        }
                        take.push(it);
                    }
                }
                items = take;
            }
            items.truncate(l);
        }
        let comments = self
            .comments_for(&principal)
            .into_iter()
            .filter(|c| instrument.as_ref().is_none_or(|i| i == &c.instrument))
            .filter(|c| since_bar.is_none_or(|b| c.bar >= b))
            .filter(|c| since_t.is_none_or(|t| c.t >= t))
            .collect();
        Ok(QueryResult::Capsules {
            capsules: items,
            comments,
            coin_flip_hidden,
        })
    }

    /// Answer to the `capsule` query (run on the asker's projected document).
    pub(crate) fn capsule_query(&self, ci: usize, id: &str, path: &str) -> PResult<QueryResult> {
        self.capsule_query_with(ci, id, path, true)
    }

    #[allow(clippy::too_many_lines)]
    /// [`Session::capsule_query`]; with `with_comments`, the comments the reader may
    /// receive (R5B, §25.4).
    pub(crate) fn capsule_query_with(
        &self,
        ci: usize,
        id: &str,
        path: &str,
        with_comments: bool,
    ) -> PResult<QueryResult> {
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let p = self.proj.as_deref();
        let doc = &self.doc;
        let Some(c) = doc
            .capsules
            .iter()
            .filter(|c| c.story.is_some())
            .find(|c| c.id == id || c.finding == id)
        else {
            return Err(E::new(
                ErrorCode::NotFound,
                format!("no capsule with a story `{id}`"),
            )
            .at(ptr(path, "capsule"))
            .dym(
                id,
                doc.capsules
                    .iter()
                    .filter(|c| c.story.is_some())
                    .map(|c| c.id.as_str()),
            )
            .build());
        };
        let mut item = self
            .capsule_item(doc, c, p, lang, &principal)
            .ok_or_else(|| {
                E::new(ErrorCode::NoData, "the capsule's bars are not available").build()
            })?;
        if let Some(v) = self.conns[ci].look.voice.as_deref() {
            let clock = self.conns[ci].look.clock;
            item.voice = self.reader_voice(doc, c, p, lang, v, clock, &mut StudyMemo::default());
        }
        self.market_item(&mut item, lang, &principal);
        let story = p
            .and_then(|p| p.stories.get(&c.id))
            .or(c.story.as_ref())
            .cloned()
            .unwrap_or_else(|| unreachable!("story checked"));
        let facts: Vec<Fact> = story_fact_ids(&story)
            .iter()
            .filter_map(|i| doc.facts.iter().find(|f| &f.id == i).cloned())
            .collect();
        let rd = rd!(self);
        let (now, bars) = Self::now_bar(rd, &story).unwrap_or((story.last_bar, &[]));
        let now_t = bars
            .get(usize::try_from(now).unwrap_or(0) + 1)
            .or_else(|| bars.get(usize::try_from(now).unwrap_or(0)))
            .map_or(i64::MAX, |b| b.t);
        let ledger = self
            .ledger_now()
            .into_iter()
            .filter(|(k, ..)| k == &story.ledger_key)
            .fold(
                qp_schema::capsule::LedgerStat::default(),
                |mut s, (_, _, t, real)| {
                    if t < now_t {
                        s.n += 1;
                        s.realized += u32::from(real);
                    }
                    s
                },
            );
        // (K42/A28) The glyph carries the item's validity: it says what the sentence says.
        let glyph = qp_capsule::glyph_spec_at(
            &story,
            now,
            qp_grammar::depth::GLYPH_MIN_LEDGER_N,
            item.validity,
        );
        let evidence = facts.iter().map(|f| self.evidence_access(f, p)).collect();
        Ok(QueryResult::CapsuleDetail {
            detail: Box::new(CapsuleDetail {
                item,
                story,
                facts,
                ledger,
                glyph,
                evidence,
                withheld: None,
                comments: if with_comments {
                    self.capsule_comments(ci, &c.id)
                } else {
                    Vec::new()
                },
            }),
        })
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

    /// One fact's evidence for a reader (K34): the values, or the entitlement they
    /// require and what they rest on.
    fn evidence_access(&self, f: &Fact, p: Option<&Proj>) -> EvidenceAccess {
        let m = f.meaning.as_ref();
        let licensed = m.is_some_and(|m| m.license.class == LicenseClass::Licensed);
        let open = p.is_none_or(|p| !p.redact.contains(&f.id));
        let doc = rd!(self);
        let bs = bar_series(doc, &f.series);
        let feed = bs.and_then(qp_schema::Series::binding);
        let symbol = m.map_or_else(|| f.series.clone(), |m| m.instrument.clone());
        let mut rests_on = match &feed {
            Some(b) => format!(
                "bars {} {} (feed {})",
                b.symbol,
                b.interval,
                b.feed.as_deref().unwrap_or("default")
            ),
            None => format!(
                "bars of series {}",
                bs.map_or(f.series.as_str(), |s| s.id.as_str())
            ),
        };
        let requires = if open {
            None
        } else if licensed {
            m.and_then(|m| m.license.entitlement.clone())
        } else {
            feed.as_ref().map(|b| match &b.feed {
                Some(fd) => format!("{fd}/{}", b.symbol),
                None => b.symbol.clone(),
            })
        };
        if licensed {
            let det = m.map_or("", |m| m.detector.as_str());
            rests_on = format!("licensed flow of {symbol} ({det}); {rests_on}");
        }
        EvidenceAccess {
            fact: f.id.clone(),
            family: m.map_or(EvidenceFamily::Price, |m| m.family),
            license: m.map_or(LicenseClass::Public, |m| m.license.class),
            open,
            values: if open {
                f.evidence.clone()
            } else {
                BTreeMap::new()
            },
            evidence_ref: if open { f.evidence_ref.clone() } else { None },
            requires,
            rests_on,
        }
    }

    /// The ledger rows with a key, retracted capsules left out: the engine's when the
    /// authority runs it (license-independent, K33: the same for every reader, also
    /// inside [`Session::as_reader`]), else the document's.
    fn ledger_now(&self) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
        match self.runner.as_ref() {
            Some(r) => {
                // (R3/K1) In the public world, the public-only engine's.
                let engine = match (self.in_public, r.pub_engine.as_ref()) {
                    (true, Some(e)) => e,
                    _ => &r.engine,
                };
                let doc = Document {
                    ledger: qp_schema::Ledger {
                        entries: engine.ledger(),
                    },
                    ..Document {
                        schema_version: String::new(),
                        id: String::new(),
                        title: None,
                        display_tz: None,
                        series: Vec::new(),
                        views: Vec::new(),
                        facts: Vec::new(),
                        findings: Vec::new(),
                        capsules: Vec::new(),
                        intents: Vec::new(),
                        ledger: qp_schema::Ledger::default(),
                        narratives: Vec::new(),
                        proposals: Vec::new(),
                        audiences: BTreeMap::new(),
                        comments: Vec::new(),
                    }
                };
                ledger_rows(&doc)
            }
            None => ledger_rows(&self.doc),
        }
    }

    /// Answer to the `ledger` query: rows of the ledger with a key, the retracted
    /// capsules left out.
    pub(crate) fn ledger_query(
        &self,
        key: Option<&str>,
        meaning: Option<qp_schema::capsule::MeaningKind>,
        detector: Option<&str>,
        direction: Option<qp_schema::capsule::Direction>,
        by_kind: bool,
    ) -> QueryResult {
        let mut rows: BTreeMap<String, LedgerRow> = BTreeMap::new();
        let mut total = qp_schema::capsule::LedgerStat::default();
        for (k, kind, _, real) in self.ledger_now() {
            let parsed = qp_capsule::parse_ledger_key(&k);
            let det = parsed.map(|(_, _, d, _)| d.to_owned());
            let dir = parsed.and_then(|(_, d, _, _)| d);
            if key.is_some_and(|x| x != k)
                || meaning.is_some_and(|m| m != kind)
                || detector.is_some_and(|d| det.as_deref() != Some(d))
                || direction.is_some_and(|d| dir != Some(d))
            {
                continue;
            }
            let rk = if by_kind {
                kind.name().to_owned()
            } else {
                k.clone()
            };
            let row = rows.entry(rk.clone()).or_insert(LedgerRow {
                key: rk,
                kind,
                direction: if by_kind { direction } else { dir },
                detector: if by_kind { None } else { det },
                n: 0,
                realized: 0,
            });
            row.n += 1;
            row.realized += u32::from(real);
            total.n += 1;
            total.realized += u32::from(real);
        }
        QueryResult::Ledger {
            rows: rows.into_values().collect(),
            total,
        }
    }
}
