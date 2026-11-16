//! Building blocks of a short headline (R4; for the capsule face, K43/M8).
//!
//! The R3 sentence ([`super::Template`]) is a report: bar counts ("6 bar önce"), internal
//! terms ("aynı aileden"), several clauses. It stays as it is (the R3 transcripts and the
//! detail depth use it). A face headline is built by its design (M8) from the pieces
//! here, each a pure function of engine data, in Turkish and English:
//!
//! | Piece | TR | EN |
//! |-------|----|----|
//! | [`Parts::event`] | yukarı kırılım | upside breakout |
//! | [`Parts::when`] (birth, [`super::elapsed`]) | 6 işlem günü önce, dün 15:00'te | 6 trading days ago |
//! | [`Parts::moved`] (last transition) | bugün | today |
//! | [`Parts::validity_phrase`] | hâlâ geçerli / artık geçerli değil / gerçekleşti | still holds / no longer holds / played out |
//! | [`Parts::direction_word`] | yukarı / aşağı / yönsüz | up / down / no direction |
//! | [`Parts::ledger_short`] | 7'de 3 tuttu / az örnek | 3 of 7 played out / few cases |
//!
//! No piece says "bar", "aile" or "teyit" (K43). A redacted reader (K33) gets no digit in
//! any piece. [`Parts::validity`] is the same value as the capsule item's and the glyph's
//! (K42/A28).

use qp_calendar::ExchangeCalendar;
use qp_schema::capsule::{Direction, LedgerStat, MeaningKind, Stage, Story, Validity};

use super::elapsed::{elapsed, phrase, phrase_vague, Elapsed, Period};
use super::morph::Word;
use super::{en, tr, LEDGER_MIN_N};
use crate::Lang;

/// How a closed story ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Gerçekleşti.
    Realized,
    /// Tutmadı (invalidated).
    Failed,
    /// Süresi doldu.
    Expired,
    /// Geri çekildi (the data was wrong).
    Retracted,
}

impl Ending {
    /// The ending of a closed stage; `None` for an open one.
    #[must_use]
    pub fn of(stage: Stage) -> Option<Self> {
        match stage {
            Stage::Realized => Some(Self::Realized),
            Stage::Invalidated => Some(Self::Failed),
            Stage::Expired => Some(Self::Expired),
            Stage::Retracted => Some(Self::Retracted),
            _ => None,
        }
    }
}

/// The pieces of a headline for one story at one "now".
#[derive(Debug, Clone, PartialEq)]
pub struct Parts {
    /// Instrument symbol.
    pub instrument: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Stage now.
    pub stage: Stage,
    /// When it was born (the birth bar's time to now).
    pub born: Elapsed,
    /// When it last moved (the last transition's bar time to now).
    pub last: Elapsed,
    /// Validity now (K42/A28).
    pub validity: Validity,
    /// How it ended, when closed.
    pub ending: Option<Ending>,
    /// Ledger at birth.
    pub ledger: LedgerStat,
    /// No digits for this reader (K33).
    pub redacted: bool,
}

/// The headline pieces of `story` as of bar time `now_t` on `cal` for bars of `period`.
/// `validity` comes from [`crate::sentence::validity_now`] (the same value as the item's
/// and the glyph's); `redacted` from the reader's entitlement.
#[must_use]
pub fn parts(
    story: &Story,
    now_t: i64,
    validity: Validity,
    cal: &ExchangeCalendar,
    period: Period,
    redacted: bool,
) -> Parts {
    let born_t = story.history.first().map_or(now_t, |h| h.t);
    let last_t = story.history.last().map_or(now_t, |h| h.t);
    Parts {
        instrument: story.instrument.clone(),
        kind: story.kind,
        direction: story.direction,
        stage: story.stage,
        born: elapsed(cal, period, born_t, now_t),
        last: elapsed(cal, period, last_t, now_t),
        validity,
        ending: Ending::of(story.stage),
        ledger: story.ledger_at_birth,
        redacted,
    }
}

impl Parts {
    /// What happened, as a short noun phrase ("yukarı kırılım", "upside breakout").
    #[must_use]
    pub fn event(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Tr => tr::noun_state(self.kind, self.direction),
            Lang::En => en::noun_bare(self.kind, self.direction),
        }
    }

    fn el(&self, e: &Elapsed, lang: Lang) -> String {
        if self.redacted {
            phrase_vague(e, lang)
        } else {
            phrase(e, lang)
        }
    }

    /// When it was born, in everyday words ("6 işlem günü önce").
    #[must_use]
    pub fn when(&self, lang: Lang) -> String {
        self.el(&self.born, lang)
    }

    /// When it last moved ("bugün").
    #[must_use]
    pub fn moved(&self, lang: Lang) -> String {
        self.el(&self.last, lang)
    }

    /// The direction as one word.
    #[must_use]
    pub fn direction_word(&self, lang: Lang) -> &'static str {
        match (self.direction, lang) {
            (Direction::Up, Lang::Tr) => "yukarı",
            (Direction::Down, Lang::Tr) => "aşağı",
            (Direction::Neutral, Lang::Tr) => "yönsüz",
            (Direction::Up, Lang::En) => "up",
            (Direction::Down, Lang::En) => "down",
            (Direction::Neutral, Lang::En) => "no direction",
        }
    }

    /// Whether it still holds, in words. Closed: how it ended.
    #[must_use]
    pub fn validity_phrase(&self, lang: Lang) -> &'static str {
        match (self.validity, self.ending, lang) {
            (Validity::Valid, _, Lang::Tr) => "hâlâ geçerli",
            (Validity::Valid, _, Lang::En) => "still holds",
            (Validity::NoLonger, _, Lang::Tr) => "artık geçerli değil",
            (Validity::NoLonger, _, Lang::En) => "no longer holds",
            (Validity::Closed, Some(Ending::Realized), Lang::Tr) => "gerçekleşti",
            (Validity::Closed, Some(Ending::Realized), Lang::En) => "played out",
            (Validity::Closed, Some(Ending::Failed), Lang::Tr) => "tutmadı",
            (Validity::Closed, Some(Ending::Failed), Lang::En) => "did not play out",
            (Validity::Closed, Some(Ending::Expired), Lang::Tr) => "süresi doldu",
            (Validity::Closed, Some(Ending::Expired), Lang::En) => "ran out of time",
            (Validity::Closed, Some(Ending::Retracted), Lang::Tr) => "geri çekildi",
            (Validity::Closed, Some(Ending::Retracted), Lang::En) => "withdrawn",
            (Validity::Closed, None, Lang::Tr) => "kapandı",
            (Validity::Closed, None, Lang::En) => "closed",
        }
    }

    /// The ledger, shortened but honest: counts (TR `7'de 3 tuttu`, EN `3 of 7 played
    /// out`) with "az örnek / few cases" below the sample threshold; qualitative when
    /// redacted; "karnesi yok / no record yet" when empty.
    #[must_use]
    pub fn ledger_short(&self, lang: Lang) -> String {
        let l = self.ledger;
        if l.n == 0 {
            return match lang {
                Lang::Tr => "karnesi yok".into(),
                Lang::En => "no record yet".into(),
            };
        }
        let thin = l.n < LEDGER_MIN_N;
        if self.redacted {
            return match (thin, lang) {
                (true, Lang::Tr) => "az örnek".into(),
                (true, Lang::En) => "few cases".into(),
                (false, Lang::Tr) if 10 * l.realized >= 6 * l.n => "çoğunlukla tuttu".into(),
                (false, Lang::En) if 10 * l.realized >= 6 * l.n => "mostly played out".into(),
                (false, Lang::Tr) if 10 * l.realized >= 4 * l.n => "yarı yarıya tuttu".into(),
                (false, Lang::En) if 10 * l.realized >= 4 * l.n => {
                    "played out about half the time".into()
                }
                (false, Lang::Tr) => "çoğunlukla tutmadı".into(),
                (false, Lang::En) => "mostly did not play out".into(),
            };
        }
        let base = match lang {
            Lang::Tr => format!(
                "{} {} tuttu",
                Word::number(&l.n.to_string()).loc(),
                l.realized
            ),
            Lang::En => format!("{} of {} played out", l.realized, l.n),
        };
        match (thin, lang) {
            (true, Lang::Tr) => format!("{base} (az örnek)"),
            (true, Lang::En) => format!("{base} (few cases)"),
            _ => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_calendar::{builtin, days_from_civil};
    use qp_schema::capsule::{EvidenceFamily, TimeKind, Transition, Trust};

    fn ist(m: u32, d: u32) -> i64 {
        (days_from_civil(2026, m, d) * 86_400 + 7 * 3600) * 1000
    }

    fn story(stage: Stage, born: i64, last: i64, lab: LedgerStat) -> Story {
        let tr = |stage, t| Transition {
            stage,
            bar: 0,
            t,
            facts: Vec::new(),
            reason: String::new(),
        };
        Story {
            instrument: "SYN03".into(),
            theme: "breakout-up".into(),
            kind: MeaningKind::Breakout,
            direction: Direction::Up,
            time: TimeKind::Event,
            stage,
            families: vec![EvidenceFamily::Price],
            strength: 0.5,
            importance: 1.0,
            tier: 1,
            series: "px".into(),
            born_bar: 0,
            last_bar: 0,
            horizon_bars: 20,
            ledger_key: "breakout/up/breakout.range@1".into(),
            ledger_at_birth: lab,
            trust: Trust::Replica,
            history: vec![tr(Stage::Born, born), tr(stage, last)],
            condition: None,
            resolution: None,
        }
    }

    #[test]
    fn pieces_in_both_languages() {
        let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
        let s = story(
            Stage::Strengthened,
            ist(4, 30),
            ist(5, 4),
            LedgerStat { n: 7, realized: 3 },
        );
        let p = parts(
            &s,
            ist(5, 6),
            Validity::NoLonger,
            &cal,
            Period::Daily,
            false,
        );
        assert_eq!(p.event(Lang::Tr), "yukarı kırılım");
        assert_eq!(p.event(Lang::En), "upside breakout");
        // 30 Apr → 6 May: 1 May holiday and the weekend skipped → 3 trading days.
        assert_eq!(p.when(Lang::Tr), "3 işlem günü önce");
        assert_eq!(p.when(Lang::En), "3 trading days ago");
        assert_eq!(p.validity_phrase(Lang::Tr), "artık geçerli değil");
        assert_eq!(p.validity_phrase(Lang::En), "no longer holds");
        assert_eq!(p.ledger_short(Lang::Tr), "7'de 3 tuttu (az örnek)");
        assert_eq!(p.ledger_short(Lang::En), "3 of 7 played out (few cases)");
        for l in [Lang::Tr, Lang::En] {
            for w in [p.when(l), p.moved(l), p.ledger_short(l)] {
                let lw = w.to_lowercase();
                assert!(!lw.contains("bar") && !lw.contains("aile") && !lw.contains("teyit"));
            }
        }
        let r = parts(&s, ist(5, 6), Validity::NoLonger, &cal, Period::Daily, true);
        for l in [Lang::Tr, Lang::En] {
            for w in [r.when(l), r.moved(l), r.ledger_short(l)] {
                assert!(!w.bytes().any(|b| b.is_ascii_digit()), "{w}");
            }
        }
    }

    #[test]
    fn closed_endings() {
        let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
        let s = story(
            Stage::Invalidated,
            ist(5, 4),
            ist(5, 5),
            LedgerStat::default(),
        );
        let p = parts(&s, ist(5, 5), Validity::Closed, &cal, Period::Daily, false);
        assert_eq!(p.validity_phrase(Lang::Tr), "tutmadı");
        assert_eq!(p.moved(Lang::Tr), "bugün");
        assert_eq!(p.ledger_short(Lang::En), "no record yet");
        assert_eq!(Ending::of(Stage::Retracted), Some(Ending::Retracted));
        assert_eq!(Ending::of(Stage::Weakened), None);
    }
}
