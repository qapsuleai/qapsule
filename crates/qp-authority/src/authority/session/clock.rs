//! Host clock, market status and ticks.

use super::*;

impl Session {
    /// (R4, §21) The host's own clock input, without a connection: what a `clock` message
    /// on the host's connection does (same checks, same outputs). The replay host gives
    /// the virtual time with it before each step's data events.
    ///
    /// # Errors
    /// The clock would go back (`invalid`).
    pub fn host_clock(&mut self, now: i64) -> Result<Vec<OutboundLine>, ProtocolError> {
        self.advance_clock(now)
    }

    /// (R6/T3, §24) The host's market status input: the exchange time, whether the market
    /// is open, each instrument's session phase, delayed/stale. Not the clock (that is
    /// still [`Session::host_clock`]); it changes no document and enters no log. When it
    /// differs from the last one, every `viewer` subscription is told (`market` in its
    /// event, and the capsules whose sentences now speak of the market); the `capsules`
    /// answers and events carry the market note from then on (`docs/LANGUAGE.md`).
    pub fn set_market_status(
        &mut self,
        status: qp_schema::market::MarketStatus,
    ) -> Vec<OutboundLine> {
        if self.market.as_ref() == Some(&status) {
            return Vec::new();
        }
        self.market = Some(status);
        self.viewer_flush()
    }

    /// (R6/T3, §24, §20.5) Whether `principal` may see the bars of instrument `symbol`: a
    /// bound series of it whose every feed the reader is entitled to.
    pub(crate) fn symbol_entitled(&self, principal: &str, symbol: &str) -> bool {
        self.doc
            .series
            .iter()
            .filter(|s| s.instrument.as_ref().is_some_and(|i| i.symbol == symbol))
            .any(|s| {
                crate::claims::feeds_of(&self.doc, &[s.id.as_str()]).is_some_and(|ks| {
                    !ks.is_empty() && ks.iter().all(|k| self.plane.entitled(principal, k))
                })
            })
    }

    /// (R6/T3, §24, §20.5) The market status as reader `principal` gets it: the phases of
    /// the instruments whose bars it may see, without the feed's numeric codes (`code`
    /// 0, not sent); open/closed, holiday, delayed and stale for everyone.
    #[must_use]
    pub fn market_for(&self, principal: &str) -> Option<qp_schema::market::MarketStatus> {
        let m = self.market.as_ref()?;
        let mut v = m.clone();
        v.phases = m
            .phases
            .iter()
            .filter(|(sym, _)| self.symbol_entitled(principal, sym))
            .map(|(sym, p)| {
                (
                    sym.clone(),
                    qp_schema::market::PhaseCode {
                        code: 0,
                        key: p.key.clone(),
                    },
                )
            })
            .collect();
        Some(v)
    }

    /// (R6/T3) The market status the host last gave.
    #[must_use]
    pub fn market_status(&self) -> Option<&qp_schema::market::MarketStatus> {
        self.market.as_ref()
    }

    /// (R6/T3, §24) Writes the market note into a capsule item for its reader: the R4
    /// sentence (`text`, `tr`, `en`) ends with the note's sentence; a voice's headline with
    /// its clause, its detail with its sentence, and its text face is rebuilt. Nothing
    /// without a market status or when the instrument trades continuously on fresh data.
    pub(crate) fn market_item(
        &self,
        it: &mut qp_schema::protocol::CapsuleItem,
        lang: qp_schema::protocol::Language,
        principal: &str,
    ) {
        use qp_capsule::market::{note_of, with_clause, with_sentence};
        use qp_schema::protocol::Language;
        let Some(m) = self.market.as_ref() else {
            return;
        };
        // (§24, §20.5) An instrument's phase speaks only to a reader entitled to its bars;
        // the others hear the market's open/closed and stale only.
        let note = if !m.phases.contains_key(&it.instrument)
            || self.symbol_entitled(principal, &it.instrument)
        {
            note_of(m, &it.instrument)
        } else {
            let general = qp_schema::market::MarketStatus {
                phases: BTreeMap::new(),
                ..m.clone()
            };
            note_of(&general, &it.instrument)
        };
        let Some(note) = note else {
            return;
        };
        let red = it.redacted;
        it.sentence.tr = with_sentence(&it.sentence.tr, note, Language::Tr, red);
        it.sentence.en = with_sentence(&it.sentence.en, note, Language::En, red);
        it.sentence.text = match lang {
            Language::Tr => it.sentence.tr.clone(),
            Language::En => it.sentence.en.clone(),
        };
        if let Some(v) = it.voice.as_mut() {
            v.headline = with_clause(&v.headline, note, lang);
            v.detail = with_sentence(&v.detail, note, lang, red);
            v.face = qp_voice::text_face(&qp_voice::Surfaces {
                headline: v.headline.clone(),
                detail: v.detail.clone(),
                ledger: v.ledger.clone(),
                glossary: Vec::new(),
            });
        }
    }

    pub(super) fn tick(&mut self, ci: usize, now: i64) -> PResult<Vec<OutboundLine>> {
        if self.conns[ci].role != Role::Host {
            return Err(E::new(ErrorCode::Forbidden, "only the host sends `clock`")
                .hint("The authority's time comes from its host connection.")
                .build());
        }
        self.advance_clock(now)
    }

    fn advance_clock(&mut self, now: i64) -> PResult<Vec<OutboundLine>> {
        if now < self.clock {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("clock went back from {} to {now}", self.clock),
            )
            .at("/now")
            .expected([format!(">= {}", self.clock)])
            .build());
        }
        self.clock = now;
        let due: Vec<Proposal> = self
            .doc
            .proposals
            .iter()
            .filter(|p| p.state == ProposalState::Pending && p.expires_at.is_some_and(|e| now >= e))
            .cloned()
            .collect();
        let mut lines = Vec::new();
        for mut p in due {
            let seq = self.seq() + 1;
            p.state = ProposalState::Expired;
            p.decided_seq = Some(seq);
            let pid = p.id.clone();
            if let Some(s) = self.engine_entry(
                vec![ChangeOp::PutProposal {
                    proposal: p,
                    index: None,
                }],
                format!("expire {pid}"),
            ) {
                lines.extend(self.settle(s));
            }
        }
        Ok(lines)
    }
}
