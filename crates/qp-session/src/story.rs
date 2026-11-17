//! (R5/M18, K50, K54, K53, K48, K33, K23; `docs/PROTOCOL.md` §23) The instrument's story
//! list: the `stories` query and topic.
//!
//! Every read runs in the reader's world ([`Session::as_reader`], §20.5): a public-view
//! reader lists the public world's stories (the licensed ones exist for it only as the
//! server comments it already gets), a reader without the bar entitlement gets digit-free
//! faces. Rows carry no bars (K23) and no number derived from licensed data: tier, ledger,
//! validity and feed place come from the reader's world.
//!
//! The topic is pushed like the viewer's updates (§22.2): after every input the authority
//! compares, per `stories` subscription, each row's *key* (stage, latest and closing bar,
//! validity, tier, feed place, coin-flip mark, redaction) with what it last sent and sends
//! the rows whose key changed — at most once per reader-world log position. The face texts'
//! relative times ("dün") follow the key's changes, not every bar.

use std::collections::BTreeMap;

use qp_schema::capsule::{Sensitivity, Stage, Story, Validity};
use qp_schema::protocol::ErrorCode;
use qp_schema::protocol::{
    Event, FeedHidden, OutboundLine, QueryResult, Range, Seq, ServerMsg, StoryFace, StoryOutcome,
    StoryRow, Topic, STORIES_DEFAULT_LIMIT, STORIES_MAX_LIMIT,
};
use qp_schema::{Capsule, Document, Series, SeriesData, SeriesSource};

use crate::capsule::Proj;
use crate::error::{ptr, PResult, E};
use crate::session::{out, Session};
use qp_capsule::sentence::StudyMemo;

/// (K53; R5/M19) A host's coin-flip rule (`qp_capsule::feedrule`, `docs/CAPSULE.md`
/// §9.10): whether a story's ledger at birth is near a coin flip, and the K42 exemption.
/// Without one no story is marked.
pub type CoinFlipRule = qp_capsule::feedrule::FeedRule;

/// What a `stories` subscription was last sent: the reader-world log position and each
/// row's key digest.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct StoriesSent {
    gen: Seq,
    rows: BTreeMap<String, u64>,
}

/// The part of a row whose change is pushed.
#[derive(serde::Serialize)]
struct RowKey {
    stage: Stage,
    last_bar: u64,
    validity: Validity,
    tier: u8,
    in_feed: bool,
    hidden: Option<FeedHidden>,
    coin_flip: bool,
    redacted: bool,
}

fn key_digest(k: &RowKey) -> u64 {
    crate::replica::fnv1a64(&serde_json::to_vec(k).unwrap_or_default())
}

/// The bar series of `instrument` in `doc` that stories can be listed on (bars, not
/// studies or points), document order.
fn instrument_series<'d>(doc: &'d Document, instrument: &str) -> Vec<&'d Series> {
    doc.series
        .iter()
        .filter(|s| s.symbol() == Some(instrument))
        .filter(|s| !matches!(s.source, SeriesSource::Study { .. }))
        .filter(|s| !matches!(s.data, Some(SeriesData::Points { .. })))
        .collect()
}

impl Session {
    /// (K53, §23) Sets the host's coin-flip rule (`None`: no story is marked). Applied to
    /// the feed (a marked story reaches a reader only at the `all` sensitivity, K53 rule
    /// 4) and to the story list's mark. Not a protocol message: the authority's
    /// configuration, like the trust keys.
    pub fn set_coin_flip_rule(&mut self, rule: Option<CoinFlipRule>) {
        self.coin_flip = rule;
    }

    /// (R5/M19) Sets the rule for `principal`'s reads over the host's own (`None`: no rule
    /// for that principal). The presentation host uses it for a page's `?feed_rule=`
    /// choice. Host configuration, like [`Session::set_coin_flip_rule`].
    pub fn set_principal_coin_flip_rule(&mut self, principal: &str, rule: Option<CoinFlipRule>) {
        self.coin_flip_for.insert(principal.to_owned(), rule);
    }

    /// (R5/M19) Drops `principal`'s own rule: its reads use the host's again.
    pub fn clear_principal_coin_flip_rule(&mut self, principal: &str) {
        self.coin_flip_for.remove(principal);
    }

    /// The rule `principal`'s reads use.
    pub(crate) fn coin_flip_rule(&self, principal: &str) -> Option<CoinFlipRule> {
        match self.coin_flip_for.get(principal) {
            Some(r) => *r,
            None => self.coin_flip,
        }
    }

    /// Whether `story` is marked "coin flip" (K53) for `principal`.
    pub(crate) fn coin_flip_of(&self, principal: &str, story: &Story) -> bool {
        self.coin_flip_rule(principal)
            .is_some_and(|r| r.coin_flip(story))
    }

    /// (§23, §20.4) The feed place of `story` for `principal` judged at `stage`: whether it
    /// passes the audience filter and, if not, why. The watch list first (empty = every
    /// instrument; positions and active intents' instruments always pass); then the
    /// position/intent override of a negative transition (K42/A22: whatever the level);
    /// then the coin-flip rule (K53: hidden below the `all` sensitivity); then the
    /// sensitivity level. (R5/M19) A rule without the K42 exemption
    /// (`exempt_negative: false`) is judged before the override: a coin-flip story about a
    /// holding is hidden even on a negative transition.
    pub(crate) fn feed_place(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        stage: Stage,
    ) -> (bool, Option<FeedHidden>) {
        let a = Self::audience_of(doc, principal);
        let intents = Self::intents_on(doc);
        let watched = a.watchlist.is_empty()
            || a.watchlist.iter().any(|s| s == &story.instrument)
            || a.positions.contains_key(&story.instrument)
            || intents.iter().any(|s| s == &story.instrument);
        if !watched {
            return (false, Some(FeedHidden::Watchlist));
        }
        let rule = self.coin_flip_rule(principal);
        let coin_flip = rule.is_some_and(|r| r.coin_flip(story));
        let exempt = rule.is_none_or(|r| r.exempt_negative);
        if stage.is_negative()
            && (exempt || !coin_flip)
            && qp_capsule::audience::relevant(&a, story, &intents)
        {
            return (true, None);
        }
        if coin_flip && a.effective() != Sensitivity::All {
            return (false, Some(FeedHidden::CoinFlip));
        }
        if !qp_capsule::admits_at(&a, story, stage, &intents) {
            return (false, Some(FeedHidden::Sensitivity));
        }
        (true, None)
    }

    /// The bar series the story list of `instrument` (and `series`) is on, in the
    /// reader's document. `base`: the JSON pointer of the query or topic.
    fn stories_series(
        &self,
        instrument: &str,
        series: Option<&str>,
        base: &str,
    ) -> PResult<String> {
        let doc = &self.doc;
        let cands = instrument_series(doc, instrument);
        if cands.is_empty() {
            let mut syms: Vec<&str> = doc.series.iter().filter_map(Series::symbol).collect();
            syms.dedup();
            return Err(E::new(
                ErrorCode::NotFound,
                format!("no bar series of instrument `{instrument}`"),
            )
            .at(ptr(base, "instrument"))
            .dym(instrument, syms)
            .build());
        }
        match series {
            Some(s) => cands
                .iter()
                .find(|c| c.id == s)
                .map(|c| c.id.clone())
                .ok_or_else(|| {
                    E::new(
                        ErrorCode::NotFound,
                        format!("`{s}` is not a bar series of `{instrument}`"),
                    )
                    .at(ptr(base, "series"))
                    .dym(s, cands.iter().map(|c| c.id.as_str()))
                    .build()
                }),
            None if cands.len() == 1 => Ok(cands[0].id.clone()),
            None => Err(E::new(
                ErrorCode::Ambiguous,
                format!("`{instrument}` has {} bar series", cands.len()),
            )
            .at(ptr(base, "series"))
            .suggest(cands.iter().map(|c| c.id.clone()))
            .hint("Name one with `series`.")
            .build()),
        }
    }

    /// Bar times of bar series `series` (the reading document's) and the index of its last
    /// final bar (`None`: no bars).
    fn series_times(&self, series: &str) -> (Vec<i64>, Option<u64>) {
        let rd = rd!(self);
        let Some(bs) = crate::values::bar_series(rd, series)
            .or_else(|| crate::values::bar_series(&self.doc, series))
        else {
            return (Vec::new(), None);
        };
        let bars = bs.bars().unwrap_or(&[]);
        let t: Vec<i64> = bars.iter().map(|b| b.t).collect();
        let last = (!bars.is_empty()).then(|| {
            let doc = if rd.series.iter().any(|s| s.id == bs.id) {
                rd
            } else {
                &self.doc
            };
            u64::try_from(crate::capsule::final_count(doc, &bs.id, bars.len()).max(1) - 1)
                .unwrap_or(0)
        });
        (t, last)
    }

    /// The stories (with their capsules) of bar series `series` in the reader's document,
    /// by birth: time, bar, id.
    fn series_stories(&self, series: &str) -> Vec<(&Capsule, &Story)> {
        let p = self.proj.as_deref();
        let mut v: Vec<(&Capsule, &Story)> = self
            .doc
            .capsules
            .iter()
            .filter_map(|c| {
                let s = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
                (s.series == series).then_some((c, s))
            })
            .collect();
        v.sort_by(|(ca, a), (cb, b)| {
            let ta = a.history.first().map_or(i64::MIN, |h| h.t);
            let tb = b.history.first().map_or(i64::MIN, |h| h.t);
            ta.cmp(&tb)
                .then(a.born_bar.cmp(&b.born_bar))
                .then(ca.id.cmp(&cb.id))
        });
        v
    }

    /// The key of `c`'s row for the reader (`None`: no bars to judge it by, as for the
    /// `capsules` answer).
    fn row_key(
        &self,
        c: &Capsule,
        story: &Story,
        p: Option<&Proj>,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<RowKey> {
        let doc = &self.doc;
        let validity = self.reader_validity(doc, c, p, memo)?;
        let (in_feed, hidden) = self.feed_place(doc, principal, story, story.stage);
        let redacted = p.is_some_and(|p| {
            crate::capsule::story_fact_ids(story)
                .iter()
                .filter(|i| !p.hidden.contains(*i))
                .any(|i| p.redact.contains(i))
        });
        Some(RowKey {
            stage: story.stage,
            last_bar: story.last_bar,
            validity,
            tier: story.tier,
            in_feed,
            hidden,
            coin_flip: self.coin_flip_of(principal, story),
            redacted,
        })
    }

    /// `c`'s row for the reader.
    fn story_row(
        &self,
        c: &Capsule,
        story: &Story,
        p: Option<&Proj>,
        lang: qp_schema::protocol::Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<StoryRow> {
        let doc = &self.doc;
        let parts = self.reader_parts(doc, c, p, lang, memo)?;
        let (in_feed, hidden) = self.feed_place(doc, principal, story, story.stage);
        let first = story.history.first()?;
        let last = story.history.last()?;
        let closed = story.stage.is_closed();
        Some(StoryRow {
            id: c.id.clone(),
            finding: c.finding.clone(),
            series: story.series.clone(),
            kind: story.kind,
            direction: story.direction,
            tier: story.tier,
            born_bar: story.born_bar,
            born_t: first.t,
            last_bar: story.last_bar,
            last_t: last.t,
            closed_bar: closed.then_some(story.last_bar),
            closed_t: closed.then_some(last.t),
            stage: story.stage,
            outcome: StoryOutcome::of(story.stage),
            validity: parts.validity,
            in_feed,
            hidden,
            coin_flip: self.coin_flip_of(principal, story),
            redacted: parts.redacted,
            face: StoryFace {
                event: parts.event,
                direction_word: parts.direction_word,
                when: parts.when,
                moved: parts.moved,
                validity_phrase: parts.validity_phrase,
                ledger_short: parts.ledger_short,
                few_cases: parts.few_cases,
                ledger_n: if parts.redacted { None } else { parts.ledger_n },
            },
        })
    }

    /// Answer to the `stories` query (run in the asker's world).
    pub(crate) fn stories_query(
        &self,
        ci: usize,
        instrument: &str,
        series: Option<&str>,
        range: Option<&Range>,
        (limit, after): (Option<u32>, Option<&str>),
        qp: &str,
    ) -> PResult<QueryResult> {
        if limit.is_some_and(|l| l == 0 || l > STORIES_MAX_LIMIT) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("`limit` must be 1..={STORIES_MAX_LIMIT}"),
            )
            .at(ptr(qp, "limit"))
            .expected([format!("1..={STORIES_MAX_LIMIT}")])
            .build());
        }
        let sid = self.stories_series(instrument, series, qp)?;
        let (times, last_final) = self.series_times(&sid);
        let (from, to) = match range {
            None => (0, u64::MAX),
            Some(r) => {
                let (a, b) = crate::query::resolve_range(Some(r), &times, qp)?;
                (a as u64, b as u64)
            }
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let p = self.proj.as_deref();
        let in_range: Vec<(&Capsule, &Story)> = self
            .series_stories(&sid)
            .into_iter()
            .filter(|(_, s)| {
                let end = if s.stage.is_closed() {
                    s.last_bar
                } else {
                    last_final.unwrap_or(s.last_bar).max(s.last_bar)
                };
                s.born_bar <= to && end >= from
            })
            .collect();
        let start = match after {
            None => 0,
            Some(a) => {
                in_range
                    .iter()
                    .position(|(c, _)| c.id == a)
                    .ok_or_else(|| {
                        E::new(
                        ErrorCode::NotFound,
                        format!("`{a}` is not a story of this list"),
                    )
                    .at(ptr(qp, "after"))
                    .hint("Send the previous answer's `next`, with the same instrument and range.")
                    .build()
                    })?
                    + 1
            }
        };
        let limit = limit.unwrap_or(STORIES_DEFAULT_LIMIT) as usize;
        let mut memo = StudyMemo::default();
        let mut rows = Vec::new();
        let mut next = None;
        for (c, s) in in_range.iter().skip(start) {
            if rows.len() == limit {
                next = rows.last().map(|r: &StoryRow| r.id.clone());
                break;
            }
            if let Some(r) = self.story_row(c, s, p, lang, &principal, &mut memo) {
                rows.push(r);
            }
        }
        let (lo_t, hi_t) = (
            usize::try_from(from)
                .ok()
                .and_then(|i| times.get(i))
                .copied()
                .unwrap_or(i64::MIN),
            usize::try_from(to)
                .ok()
                .and_then(|i| times.get(i))
                .copied()
                .unwrap_or(i64::MAX),
        );
        let comments = self
            .comments_for(&principal)
            .into_iter()
            .filter(|c| c.instrument == instrument && c.series == sid)
            .filter(|c| c.bar >= from && c.bar <= to && c.t >= lo_t && c.t <= hi_t)
            .collect();
        Ok(QueryResult::Stories {
            instrument: instrument.to_owned(),
            series: sid,
            stories: rows,
            total: u32::try_from(in_range.len()).unwrap_or(u32::MAX),
            next,
            comments,
        })
    }

    /// Checks a `stories` subscription (run in the subscriber's world).
    pub(crate) fn stories_check(&self, instrument: &str, series: Option<&str>) -> PResult<()> {
        self.stories_series(instrument, series, "/topic")
            .map(|_| ())
    }

    /// The keys of every row of the `stories` subscription `topic` for connection `ci`
    /// (in the subscriber's world).
    fn stories_keys(&self, ci: usize, topic: &Topic) -> BTreeMap<String, u64> {
        let Topic::Stories { instrument, series } = topic else {
            return BTreeMap::new();
        };
        let Ok(sid) = self.stories_series(instrument, series.as_deref(), "/topic") else {
            return BTreeMap::new();
        };
        let principal = &self.conns[ci].principal;
        let p = self.proj.as_deref();
        let mut memo = StudyMemo::default();
        self.series_stories(&sid)
            .into_iter()
            .filter_map(|(c, s)| {
                let k = self.row_key(c, s, p, principal, &mut memo)?;
                Some((c.id.clone(), key_digest(&k)))
            })
            .collect()
    }

    /// The `stories` subscription `sub` of connection `ci` was made: remembers its rows
    /// as sent (the state at subscription is the `stories` answer).
    pub(crate) fn stories_start(&mut self, ci: usize, sub: &str) {
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let gen = self.seq_to(&principal, self.seq());
        self.as_reader(&principal, lang, |s| {
            let Some(i) = s.conns[ci].subs.iter().position(|x| x.id == sub) else {
                return;
            };
            let rows = s.stories_keys(ci, &s.conns[ci].subs[i].topic.clone());
            s.conns[ci].subs[i].stories = Some(StoriesSent { gen, rows });
        });
    }

    /// Whether any connection holds a `stories` subscription.
    pub(crate) fn has_stories(&self) -> bool {
        self.conns
            .iter()
            .any(|c| c.subs.iter().any(|s| s.stories.is_some()))
    }

    /// (§23) The `story` events of every `stories` subscription whose reader-world log
    /// position moved since it was last told: the rows whose key changed, by birth.
    pub(crate) fn stories_flush(&mut self) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for ci in 0..self.conns.len() {
            let principal = self.conns[ci].principal.clone();
            let gen = self.seq_to(&principal, self.seq());
            let due = self.conns[ci]
                .subs
                .iter()
                .any(|s| s.stories.as_ref().is_some_and(|x| x.gen != gen));
            if !due {
                continue;
            }
            let lang = self.lang_of(ci);
            let l = self.as_reader(&principal, lang, |s| s.stories_lines(ci, gen));
            lines.extend(l);
        }
        lines
    }

    /// [`Session::stories_flush`] for connection `ci`, in its world.
    fn stories_lines(&mut self, ci: usize, gen: Seq) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let conn = self.conns[ci].id.clone();
        for i in 0..self.conns[ci].subs.len() {
            let Some(sent) = self.conns[ci].subs[i].stories.clone() else {
                continue;
            };
            if sent.gen == gen {
                continue;
            }
            let topic = self.conns[ci].subs[i].topic.clone();
            let sub = self.conns[ci].subs[i].id.clone();
            let keys = self.stories_keys(ci, &topic);
            let Topic::Stories { instrument, series } = &topic else {
                continue;
            };
            if let Ok(sid) = self.stories_series(instrument, series.as_deref(), "/topic") {
                let p = self.proj.as_deref();
                let mut memo = StudyMemo::default();
                for (c, s) in self.series_stories(&sid) {
                    let Some(k) = keys.get(&c.id) else { continue };
                    if sent.rows.get(&c.id) == Some(k) {
                        continue;
                    }
                    let Some(row) = self.story_row(c, s, p, lang, &principal, &mut memo) else {
                        continue;
                    };
                    lines.push(out(
                        &conn,
                        ServerMsg::Notify {
                            sub: sub.clone(),
                            event: Event::Story {
                                story: Box::new(row),
                                seq: gen,
                            },
                        },
                    ));
                }
            }
            self.conns[ci].subs[i].stories = Some(StoriesSent { gen, rows: keys });
        }
        lines
    }
}
