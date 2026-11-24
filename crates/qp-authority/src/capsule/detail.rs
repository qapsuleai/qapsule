//! The `capsule` query: one capsule's detail (item, story, facts, ledger, glyph,
//! evidence, comments). Stays with capsule life (RF1 S3.2): the comment validator reads
//! the same detail (`comments.rs`); the reading subsystem asks for it through the view.

use super::*;

impl Session {
    /// Answer to the `capsule` query (run on the asker's projected document); `voice`
    /// and `clock`: the connection's view state (the reading subsystem holds it during
    /// the query, RF1 S3.2).
    pub(crate) fn capsule_query(
        &self,
        ci: usize,
        id: &str,
        path: &str,
        (voice, clock): (Option<&str>, bool),
    ) -> PResult<QueryResult> {
        self.capsule_query_with(ci, id, path, true, (voice, clock))
    }

    #[allow(clippy::too_many_lines)]
    /// [`Session::capsule_query`]; with `with_comments`, the comments the reader may
    /// receive (R5B, §25.4); `voice`: the item's voice surfaces (none: the comment
    /// validator, which reads no voice).
    pub(crate) fn capsule_query_with(
        &self,
        ci: usize,
        id: &str,
        path: &str,
        with_comments: bool,
        (voice, clock): (Option<&str>, bool),
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
        if let Some(v) = voice {
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

    /// One fact's evidence for a reader (K34): the values, or the entitlement they
    /// require and what they rest on.
    pub(super) fn evidence_access(&self, f: &Fact, p: Option<&Proj>) -> EvidenceAccess {
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
}
