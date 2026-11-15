//! R3/M2a — **template grammar realizer** (`template`).
//!
//! A sentence is a pure function of [`SentenceInput`]: no LLM, no clock, no randomness.
//! Two steps:
//!
//! 1. [`Plan::new`] reads the input into a language-independent frame: the **time class**
//!    (event / state that holds / state that ended / expectation, from `story.time`,
//!    `still_holds` and the stage), the bars-ago of the lead fact and of the last
//!    transition, the condition (subject, side, threshold key), the current value, the
//!    stage remark, the validity tail, the ledger and the trust/licence marks. Every
//!    number the frame can print comes from an engine slot; in a redacted frame the
//!    accessors return no numbers at all.
//! 2. [`tr`] / [`en`] render the frame with a fixed template family per class. Turkish
//!    suffixes come from real morphology ([`morph`]); numbers are formatted with the
//!    harness rounding rule ([`num`]).
//!
//! Redacted facts (licensed, unentitled; K33) use a separate **no-digit family**: bars-ago
//! becomes "bugün / dün / birkaç bar önce / yaklaşık bir hafta önce…", thresholds become
//! names ("aşırı alım eşiği", "kırılım seviyesi"), the ledger becomes qualitative
//! ("çoğunlukla beklendiği gibi sonuçlandı"). A last guard replaces any sentence that
//! would still carry a digit (it never fires on the tested inputs; see tests).

use std::collections::BTreeMap;

use qp_capsule::{Lang, SentenceInput};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, LedgerStat, MeaningKind, Stage, TimeKind, Trust,
};
use qp_schema::CompareOp;

pub mod en;
pub mod morph;
pub mod num;
pub mod tr;

/// Ledger sample size from which counts are stated plainly; below it the sentence says
/// the record is still thin (and still gives the counts when not redacted).
pub const LEDGER_MIN_N: u32 = 20;

/// Milliseconds in a day (calendar check for "dün / yesterday").
const DAY_MS: i64 = 86_400_000;

/// The realizer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Realizer;

impl qp_capsule::Realizer for Realizer {
    fn name(&self) -> &'static str {
        "template"
    }

    fn sentence(&self, inp: &SentenceInput<'_>, lang: Lang) -> String {
        let p = Plan::new(inp);
        let s = render(&p, lang);
        if p.redacted && s.bytes().any(|b| b.is_ascii_digit()) {
            // Safety net (K33): never let a digit through for a redacted reader.
            return fallback_redacted(&p, lang);
        }
        s
    }
}

/// Renders a plan (no guard).
#[must_use]
pub fn render(p: &Plan<'_>, lang: Lang) -> String {
    match lang {
        Lang::Tr => tr::render(p),
        Lang::En => en::render(p),
    }
}

fn fallback_redacted(p: &Plan<'_>, lang: Lang) -> String {
    let inst = if p.inst.bytes().any(|b| b.is_ascii_digit()) {
        match lang {
            Lang::Tr => "Bu hisse",
            Lang::En => "This stock",
        }
    } else {
        p.inst
    };
    match lang {
        Lang::Tr => format!(
            "{inst} için sunucunun doğruladığı bir {} sinyali var; ayrıntılar veri lisansı gerektiriyor.",
            tr::noun_state(p.kind, p.dir)
        ),
        Lang::En => format!(
            "There is a server-verified {} signal for {inst}; the details require a data license.",
            en::noun_bare(p.kind, p.dir)
        ),
    }
}

/// Time class of the sentence (K26, K27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Past tense + when.
    Event,
    /// A state that holds now: present tense.
    Holding,
    /// A state that ended: past tense + "no longer".
    Ended,
    /// Conditional.
    Expectation,
}

/// Whether the claim still holds (tail of events and expectations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Validity {
    /// Still holds / still open.
    Yes,
    /// No longer holds.
    No,
    /// Not known (no condition is tracked).
    Unknown,
}

/// "How long ago", with what the renderer needs to choose words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ago {
    /// Bars ago.
    pub n: u64,
    /// The series is daily (`.1d`): "bugün / today" for 0.
    pub daily: bool,
    /// Daily and exactly the previous calendar day: "dün / yesterday" for 1.
    pub yesterday: bool,
    /// No digits allowed.
    pub redacted: bool,
}

/// Bucket of a redacted bars-ago (no digits, no number words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vague {
    /// 0.
    Now,
    /// 1.
    One,
    /// Daily: 2–3 days; other: 2–4 bars.
    Few,
    /// Daily: 4–7 ("about a week").
    Week,
    /// Daily: 8–17 ("a few weeks").
    Weeks,
    /// Daily: 18–26 ("about a month").
    Month,
    /// Daily: > 26 ("more than a month").
    MoreThanMonth,
    /// Other: 5–20 bars ("a while").
    While,
    /// Other: > 20 bars ("a long while").
    LongWhile,
}

impl Ago {
    /// The redacted bucket.
    #[must_use]
    pub fn vague(&self) -> Vague {
        match (self.n, self.daily) {
            (0, _) => Vague::Now,
            (1, _) => Vague::One,
            (2..=3, true) | (2..=4, false) => Vague::Few,
            (4..=7, true) => Vague::Week,
            (8..=17, true) => Vague::Weeks,
            (18..=26, true) => Vague::Month,
            (_, true) => Vague::MoreThanMonth,
            (5..=20, false) => Vague::While,
            (_, false) => Vague::LongWhile,
        }
    }
}

/// The condition subject (what is compared).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// The close: the instrument itself is the subject ("THYAO … kapanırsa").
    Close,
    /// RSI.
    Rsi,
    /// Bollinger band width.
    Bandwidth,
    /// Net institutional flow.
    Net,
    /// Close's distance from the middle band.
    CloseDev,
    /// Anything else (named as is).
    Other(String),
}

impl Subject {
    fn parse(s: &str) -> Self {
        match s {
            "close" | "price" => Self::Close,
            "rsi" | "value" => Self::Rsi,
            "bandwidth" | "bw" => Self::Bandwidth,
            "net" => Self::Net,
            "close_dev" => Self::CloseDev,
            o => Self::Other(o.to_owned()),
        }
    }

    fn cat(&self) -> num::Cat {
        match self {
            Self::Close | Self::CloseDev => num::Cat::Price,
            Self::Rsi => num::Cat::Osc,
            Self::Net => num::Cat::Count,
            Self::Bandwidth | Self::Other(_) => num::Cat::Ratio,
        }
    }
}

/// The condition frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Cond {
    /// Subject.
    pub subject: Subject,
    /// `true` = holds while the subject is above the threshold.
    pub above: bool,
    /// Evidence key of the threshold.
    pub key: String,
    /// Within how many bars (expectation).
    pub within: Option<u32>,
}

/// Stage remark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remark {
    /// Nothing to say (born, or covered by the main clause).
    None,
    /// Still a candidate (open bar).
    Candidate,
    /// Same-family evidence.
    Strengthened,
    /// New evidence family.
    Confirmed,
    /// Opposite fact (or condition broke without an exit clause).
    Weakened,
    /// Outcome hit.
    Realized,
    /// Outcome miss.
    Invalidated,
    /// Horizon reached.
    Expired,
    /// Data corrected.
    Retracted,
}

/// The language-independent frame of one sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan<'a> {
    /// Instrument symbol.
    pub inst: &'a str,
    /// Kind.
    pub kind: MeaningKind,
    /// Direction.
    pub dir: Direction,
    /// Time class.
    pub class: Class,
    /// Bars since the lead fact.
    pub lead_ago: Ago,
    /// Detector id of the lead fact (`breakout.range`).
    pub detector: &'a str,
    /// Lead evidence.
    ev: &'a BTreeMap<String, f64>,
    /// Condition, if any.
    pub cond: Option<Cond>,
    /// Current value of the subject (hidden when redacted).
    current: Option<f64>,
    /// When the state ended, if an exit fact says so.
    pub exit_ago: Option<Ago>,
    /// Stage.
    pub stage: Stage,
    /// Stage remark.
    pub remark: Remark,
    /// Bars since the last transition.
    pub stage_ago: Ago,
    /// Family of the latest fact (for "hacim de teyit etti").
    pub latest_family: Option<EvidenceFamily>,
    /// Independent families.
    families: usize,
    /// Horizon in bars.
    horizon: u32,
    /// Tail for events / expectations.
    pub validity: Validity,
    /// The stage closed the story with "no" (invalidated / expired / retracted).
    pub closed_no: bool,
    /// Ledger at birth.
    ledger: LedgerStat,
    /// Server fact (K38).
    pub server: bool,
    /// Licence entitlement needed (`flow/THYAO`).
    pub entitlement: Option<&'a str>,
    /// No digits.
    pub redacted: bool,
}

fn is_daily(series: &str) -> bool {
    let tf = series.rsplit('.').next().unwrap_or("");
    matches!(tf, "1d" | "d" | "D" | "1D" | "day")
}

fn ago(now_bar: u64, now_t: i64, bar: u64, t: i64, daily: bool, redacted: bool) -> Ago {
    let n = now_bar.saturating_sub(bar);
    Ago {
        n,
        daily,
        yesterday: daily && n == 1 && now_t.div_euclid(DAY_MS) - t.div_euclid(DAY_MS) == 1,
        redacted,
    }
}

impl<'a> Plan<'a> {
    /// Reads the input.
    #[must_use]
    pub fn new(inp: &SentenceInput<'a>) -> Self {
        let s = inp.story;
        let m = inp.lead.meaning.as_ref();
        let daily = is_daily(&s.series);
        let red = inp.redacted;
        let lead_ago = ago(inp.now_bar, inp.now_t, inp.lead.bar, inp.lead.t, daily, red);
        let (st_bar, st_t, reason) = s.history.last().map_or((s.last_bar, inp.now_t, ""), |h| {
            (h.bar, h.t, h.reason.as_str())
        });
        let stage_ago = ago(inp.now_bar, inp.now_t, st_bar, st_t, daily, red);
        let cond = m.and_then(|m| m.condition.as_ref()).map(|c| Cond {
            subject: Subject::parse(&c.subject),
            above: matches!(c.op, CompareOp::Gt | CompareOp::Ge | CompareOp::CrossAbove),
            key: c.key.clone(),
            within: c.within_bars,
        });
        let closed_no = matches!(
            s.stage,
            Stage::Invalidated | Stage::Expired | Stage::Retracted
        );
        let class = match s.time {
            TimeKind::Event => Class::Event,
            TimeKind::Expectation => Class::Expectation,
            TimeKind::State if closed_no || inp.still_holds == Some(false) => Class::Ended,
            TimeKind::State if inp.still_holds == Some(true) => Class::Holding,
            TimeKind::State => Class::Event,
        };
        let latest_is_exit = inp.latest.id != inp.lead.id
            && inp
                .latest
                .meaning
                .as_ref()
                .is_some_and(|lm| lm.edge == Edge::Exit);
        let exit_ago = (class == Class::Ended && latest_is_exit).then(|| {
            ago(
                inp.now_bar,
                inp.now_t,
                inp.latest.bar,
                inp.latest.t,
                daily,
                red,
            )
        });
        let remark = match s.stage {
            Stage::Born => Remark::None,
            Stage::Candidate => Remark::Candidate,
            Stage::Strengthened => Remark::Strengthened,
            Stage::Confirmed => Remark::Confirmed,
            // The exit clause of an ended state already says it.
            Stage::Weakened if class == Class::Ended && reason == "state_ended" => Remark::None,
            Stage::Weakened => Remark::Weakened,
            Stage::Realized => Remark::Realized,
            Stage::Invalidated => Remark::Invalidated,
            Stage::Expired => Remark::Expired,
            Stage::Retracted => Remark::Retracted,
        };
        let validity = if closed_no {
            Validity::No
        } else {
            match inp.still_holds {
                Some(true) => Validity::Yes,
                Some(false) => Validity::No,
                None => Validity::Unknown,
            }
        };
        let latest_family = (inp.latest.id != inp.lead.id)
            .then(|| inp.latest.meaning.as_ref().map(|lm| lm.family))
            .flatten();
        Self {
            inst: &s.instrument,
            kind: s.kind,
            dir: s.direction,
            class,
            lead_ago,
            detector: m.map_or("", |m| m.detector.as_str()),
            ev: &inp.lead.evidence,
            cond,
            current: inp.current,
            exit_ago,
            stage: s.stage,
            remark,
            stage_ago,
            latest_family,
            families: s.families.len(),
            horizon: s.horizon_bars,
            validity,
            closed_no,
            ledger: s.ledger_at_birth,
            server: s.trust == Trust::Server || m.is_some_and(|m| m.server),
            entitlement: m.and_then(|m| m.license.entitlement.as_deref()),
            redacted: red,
        }
    }

    /// An evidence value of the lead fact (first key present), never when redacted.
    #[must_use]
    pub fn val(&self, keys: &[&str]) -> Option<f64> {
        if self.redacted {
            return None;
        }
        keys.iter()
            .find_map(|k| self.ev.get(*k).copied())
            .filter(|v| v.is_finite())
    }

    /// Whether the lead evidence has a key (no value is read).
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.ev.contains_key(key)
    }

    /// The threshold value, never when redacted.
    #[must_use]
    pub fn thr(&self) -> Option<f64> {
        let c = self.cond.as_ref()?;
        self.val(&[c.key.as_str()])
    }

    /// The current value, never when redacted.
    #[must_use]
    pub fn current(&self) -> Option<f64> {
        if self.redacted {
            None
        } else {
            self.current.filter(|v| v.is_finite())
        }
    }

    /// Independent families when there are several and numbers may be shown.
    #[must_use]
    pub fn families(&self) -> Option<usize> {
        (!self.redacted && self.families >= 2).then_some(self.families)
    }

    /// Horizon in bars, never when redacted.
    #[must_use]
    pub fn horizon(&self) -> Option<u32> {
        (!self.redacted && self.horizon > 0).then_some(self.horizon)
    }

    /// Within-bars of the condition, never when redacted.
    #[must_use]
    pub fn within(&self) -> Option<u32> {
        if self.redacted {
            None
        } else {
            self.cond.as_ref().and_then(|c| c.within)
        }
    }

    /// The condition has a within-bars bound (no number read).
    #[must_use]
    pub fn bounded(&self) -> bool {
        self.cond.as_ref().is_some_and(|c| c.within.is_some())
    }

    /// The ledger, for the ledger phrase.
    #[must_use]
    pub fn ledger(&self) -> LedgerView {
        let l = self.ledger;
        if l.n == 0 {
            return LedgerView::Empty;
        }
        if self.redacted {
            if l.n < LEDGER_MIN_N {
                return LedgerView::ThinQual;
            }
            let p = f64::from(l.realized) / f64::from(l.n);
            return LedgerView::Qual(if p >= 0.6 {
                Tendency::Mostly
            } else if p >= 0.4 {
                Tendency::Even
            } else {
                Tendency::Rarely
            });
        }
        if l.n < LEDGER_MIN_N {
            LedgerView::Thin(l.n, l.realized)
        } else {
            LedgerView::Counts(l.n, l.realized)
        }
    }

    /// Value category of the threshold key.
    #[must_use]
    pub fn thr_cat(&self) -> num::Cat {
        let Some(c) = self.cond.as_ref() else {
            return num::Cat::Ratio;
        };
        key_cat(&c.key, &c.subject)
    }
}

/// Value category of an evidence key.
#[must_use]
pub fn key_cat(key: &str, subject: &Subject) -> num::Cat {
    match key {
        "level" if *subject == Subject::Rsi => num::Cat::Osc,
        "value" | "prev" | "rsi" | "prev_rsi" | "p1_rsi" | "p2_rsi" => num::Cat::Osc,
        "level" | "close" | "price" | "fast" | "slow" | "sma20" | "sma50" | "sma20_prev5"
        | "atr" | "p1_price" | "p2_price" | "upper" | "lower" | "middle" | "half_width" => {
            num::Cat::Price
        }
        "net" | "avg5" | "volume" | "vol_avg" | "avg20" => num::Cat::Count,
        _ => num::Cat::Ratio,
    }
}

/// What the ledger phrase may say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerView {
    /// No closed capsule yet.
    Empty,
    /// `n < LEDGER_MIN_N`: counts with a "still thin" caveat.
    Thin(u32, u32),
    /// `n ≥ LEDGER_MIN_N`: "in r of n cases".
    Counts(u32, u32),
    /// Redacted, `n < LEDGER_MIN_N`: "still thin", no numbers.
    ThinQual,
    /// Redacted, `n ≥ LEDGER_MIN_N`: qualitative rate.
    Qual(Tendency),
}

/// Qualitative hit rate (redacted): ≥ 60 %, 40–60 %, < 40 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tendency {
    /// Mostly played out.
    Mostly,
    /// About as often as not.
    Even,
    /// Mostly did not.
    Rarely,
}

#[cfg(test)]
mod tests;
