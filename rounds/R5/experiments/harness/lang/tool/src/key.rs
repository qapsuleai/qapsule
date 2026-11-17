//! The answer key of a situation: what every language must convey, derived from engine
//! data and detector parameters only (never from a language's text). SPEC §2–§3.

use qp_facts::{BandSqueeze, InstitutionalFlow, MaStack, RangeBreakout, RsiExtreme, VolumeSurge};
use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
use serde::{Deserialize, Serialize};

use crate::lang::Lang;
use crate::situ::{direction_name, local_day, stage_name, validity_name, Situation};

/// What a detector's threshold is called and which numbers state it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    /// Stable id of the threshold kind (`range_extreme`, `rsi_level`, …).
    pub id: String,
    /// Plain description (for the SPEC table and the reader key).
    pub plain_tr: String,
    /// Plain description, English.
    pub plain_en: String,
    /// Name pattern (lower-case regex) per language: the threshold is *named* when the
    /// screen text matches it.
    pub name_tr: String,
    /// See `name_tr`.
    pub name_en: String,
    /// Numbers that state the threshold's value: at least one must appear.
    pub values: Vec<f64>,
    /// Detector parameters of which at least one must appear (empty: none required).
    pub params_any: Vec<f64>,
}

/// The key of one situation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Situation id.
    pub id: String,
    /// Instrument symbol (stripped before number checks).
    pub instrument: String,
    /// Plain event category (reader question "ne oldu").
    pub event: String,
    /// `up` / `down` / `neutral`.
    pub direction: String,
    /// Stage now.
    pub stage: String,
    /// `valid` / `no_longer` / `closed`.
    pub validity: String,
    /// Closed: `realized` / `failed` / `expired` / `retracted`.
    pub ending: Option<String>,
    /// Trading days since birth.
    pub born_days: i64,
    /// Trading days since the last transition.
    pub moved_days: i64,
    /// Every acceptable trading-day offset (history transitions, lead, latest, now).
    pub day_offsets: Vec<i64>,
    /// Every acceptable bar offset (same events).
    pub bar_offsets: Vec<u64>,
    /// Local dates of those events `(day, month, year)`.
    pub dates: Vec<(u32, u32, i32)>,
    /// Intraday period.
    pub intraday: bool,
    /// The threshold.
    pub threshold: Threshold,
    /// Ledger at birth: closed cases.
    pub ledger_n: u32,
    /// Ledger at birth: played out.
    pub ledger_realized: u32,
    /// Outcome horizon in bars (what "tuttu" means: within this many bars).
    pub horizon_bars: u32,
    /// No digits for this reader.
    pub redacted: bool,
}

fn civil(days: i64) -> (i32, u32, u32) {
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        i32::try_from(if m <= 2 { y + 1 } else { y }).unwrap_or(0),
        u32::try_from(m).unwrap_or(0),
        u32::try_from(d).unwrap_or(0),
    )
}

/// Plain event category of a kind and direction.
#[must_use]
pub fn event_category(kind: MeaningKind, dir: Direction) -> &'static str {
    match (kind, dir) {
        (MeaningKind::Breakout, Direction::Up) => "breakout_up",
        (MeaningKind::Breakout, Direction::Down) => "breakout_down",
        (MeaningKind::Breakout, Direction::Neutral) => "breakout",
        (MeaningKind::Reversal, Direction::Up) => "reversal_up",
        (MeaningKind::Reversal, Direction::Down) => "reversal_down",
        (MeaningKind::Reversal, Direction::Neutral) => "reversal",
        (MeaningKind::Continuation, Direction::Up) => "trend_up",
        (MeaningKind::Continuation, Direction::Down) => "trend_down",
        (MeaningKind::Continuation, Direction::Neutral) => "trend",
        (MeaningKind::Squeeze, _) => "squeeze",
        (MeaningKind::Extreme, Direction::Up) => "extreme_up",
        (MeaningKind::Extreme, Direction::Down) => "extreme_down",
        (MeaningKind::Extreme, Direction::Neutral) => "extreme",
        (MeaningKind::Flow, Direction::Up) => "flow_in",
        (MeaningKind::Flow, Direction::Down) => "flow_out",
        (MeaningKind::Flow, Direction::Neutral) => "flow",
    }
}

fn ev(s: &Situation, k: &str) -> Option<f64> {
    s.lead_raw
        .evidence
        .get(k)
        .copied()
        .filter(|v| v.is_finite())
}

fn as_f(x: usize) -> f64 {
    f64::from(u32::try_from(x).unwrap_or(u32::MAX))
}

/// The threshold of a situation's detector, keyed by the detector's parameters and the
/// lead fact's evidence (SPEC §2.6).
#[must_use]
pub fn threshold(s: &Situation) -> Threshold {
    let vals = |ks: &[&str]| {
        ks.iter()
            .filter_map(|k| ev(s, k))
            .map(f64::abs)
            .collect::<Vec<f64>>()
    };
    let t = |id: &str,
             ptr: &str,
             pen: &str,
             ntr: &str,
             nen: &str,
             values: Vec<f64>,
             params: Vec<f64>| Threshold {
        id: id.to_owned(),
        plain_tr: ptr.to_owned(),
        plain_en: pen.to_owned(),
        name_tr: ntr.to_owned(),
        name_en: nen.to_owned(),
        values,
        params_any: params,
    };
    match s.detector.as_str() {
        "breakout.range" => t(
            "range_extreme",
            &format!(
                "son {} periyodun en yükseği / en düşüğü",
                RangeBreakout::WINDOW
            ),
            &format!(
                "the highest / lowest of the last {} periods",
                RangeBreakout::WINDOW
            ),
            r"en yükse|en düşü|zirve|\bdip|diren[cç]|destek",
            r"\bhigh|\blow|\bpeak|\btrough|resistance|support",
            vals(&["level"]),
            vec![as_f(RangeBreakout::WINDOW)],
        ),
        "extreme.rsi" => t(
            "rsi_level",
            &format!("RSI {} / {} çizgisi", RsiExtreme::UPPER, RsiExtreme::LOWER),
            &format!("the RSI {} / {} line", RsiExtreme::UPPER, RsiExtreme::LOWER),
            r"\brs[iı]|göreceli güç",
            r"\brsi|relative strength",
            vals(&["level"]),
            Vec::new(),
        ),
        "continuation.ma_stack" => t(
            "moving_average",
            &format!(
                "{} ve {} periyotluk ortalamalar",
                MaStack::FAST,
                MaStack::SLOW
            ),
            &format!(
                "the {} and {} period averages",
                MaStack::FAST,
                MaStack::SLOW
            ),
            r"ortalama",
            r"average",
            vals(&["sma20", "sma50"]),
            vec![as_f(MaStack::FAST), as_f(MaStack::SLOW)],
        ),
        "reversal.rsi_divergence" => t(
            "divergence_swing",
            "fiyatın önceki dibi/tepesi ile RSI'ın aynı noktadaki değeri",
            "the price's previous low/high and the RSI there",
            r"(\brs[iı]|göreceli güç).*(\bdip|tepe|zirve|en düşü|en yüksek)|(\bdip|tepe|zirve|en düşü|en yüksek).*(\brs[iı]|göreceli güç)",
            r"(\brsi|relative strength).*(\blow|\bhigh|peak|trough)|(\blow|\bhigh|peak|trough).*(\brsi|relative strength)",
            vals(&["p1_price", "p2_price", "p1_rsi", "p2_rsi"]),
            Vec::new(),
        ),
        "squeeze.bandwidth" => t(
            "band",
            &format!(
                "{} periyotluk bandın son {} periyodun en darı olması; bandın alt ve üst sınırı",
                BandSqueeze::PERIOD,
                BandSqueeze::LOOKBACK
            ),
            &format!(
                "the {}-period band at its narrowest of the last {}; its upper and lower edge",
                BandSqueeze::PERIOD,
                BandSqueeze::LOOKBACK
            ),
            r"\bbant",
            r"\bband",
            vals(&["upper", "lower", "middle"]),
            vec![as_f(BandSqueeze::PERIOD), as_f(BandSqueeze::LOOKBACK)],
        ),
        "flow.volume_surge" => t(
            "volume_multiple",
            &format!(
                "son {} periyodun ortalama hacminin {} katı",
                VolumeSurge::WINDOW,
                VolumeSurge::MULT
            ),
            &format!(
                "{} times the average volume of the last {} periods",
                VolumeSurge::MULT,
                VolumeSurge::WINDOW
            ),
            r"hacim",
            r"volume",
            {
                let mut v = vec![VolumeSurge::MULT];
                v.extend(vals(&["ratio"]));
                v
            },
            vec![as_f(VolumeSurge::WINDOW)],
        ),
        "flow.institutional" => t(
            "institutional_multiple",
            &format!(
                "kurumsal net alım/satımın son {} periyot ortalamasının {} katı",
                InstitutionalFlow::WINDOW,
                InstitutionalFlow::MULT
            ),
            &format!(
                "{} times the {}-period average of institutional net flow",
                InstitutionalFlow::MULT,
                InstitutionalFlow::WINDOW
            ),
            r"kurum",
            r"institution",
            {
                let mut v = vec![InstitutionalFlow::MULT];
                v.extend(vals(&["net", "avg5"]));
                v
            },
            vec![as_f(InstitutionalFlow::WINDOW)],
        ),
        other => t(other, other, other, "^$", "^$", Vec::new(), Vec::new()),
    }
}

/// The key of a situation.
#[must_use]
pub fn key(s: &Situation) -> Key {
    let st = &s.story;
    let now_rank = s.rank_of(s.now_bar).unwrap_or(0);
    let born_bar = st.history.first().map_or(st.born_bar, |h| h.bar);
    let last_bar = st.history.last().map_or(st.last_bar, |h| h.bar);
    let off = |b: u64| now_rank - s.rank_of(b).unwrap_or(now_rank);
    let mut bars: Vec<u64> = st.history.iter().map(|h| h.bar).collect();
    bars.extend([s.lead.bar, s.latest.bar, s.now_bar]);
    bars.sort_unstable();
    bars.dedup();
    let mut day_offsets: Vec<i64> = bars.iter().map(|b| off(*b)).collect();
    day_offsets.sort_unstable();
    day_offsets.dedup();
    let mut bar_offsets: Vec<u64> = bars.iter().map(|b| s.now_bar.saturating_sub(*b)).collect();
    bar_offsets.sort_unstable();
    bar_offsets.dedup();
    let mut times: Vec<i64> = st.history.iter().map(|h| h.t).collect();
    times.extend([s.lead.t, s.latest.t, s.now_t]);
    let mut dates: Vec<(u32, u32, i32)> = times
        .iter()
        .map(|t| {
            let (y, m, d) = civil(local_day(*t));
            (d, m, y)
        })
        .collect();
    dates.sort_unstable();
    dates.dedup();
    let ending = match st.stage {
        Stage::Realized => Some("realized"),
        Stage::Invalidated => Some("failed"),
        Stage::Expired => Some("expired"),
        Stage::Retracted => Some("retracted"),
        _ => None,
    };
    Key {
        id: s.id.clone(),
        instrument: st.instrument.clone(),
        event: event_category(st.kind, st.direction).to_owned(),
        direction: direction_name(st.direction).to_owned(),
        stage: stage_name(st.stage).to_owned(),
        validity: validity_name(s.validity).to_owned(),
        ending: ending.map(str::to_owned),
        born_days: off(born_bar),
        moved_days: off(last_bar),
        day_offsets,
        bar_offsets,
        dates,
        intraday: s.period != "1day",
        threshold: threshold(s),
        ledger_n: st.ledger_at_birth.n,
        ledger_realized: st.ledger_at_birth.realized,
        horizon_bars: st.horizon_bars,
        redacted: s.redacted,
    }
}

impl Key {
    /// The threshold's name pattern in `lang`.
    #[must_use]
    pub fn name_pattern(&self, lang: Lang) -> &str {
        match lang {
            Lang::Tr => &self.threshold.name_tr,
            Lang::En => &self.threshold.name_en,
        }
    }

    /// Validity class with the ending when closed (`valid`, `no_longer`,
    /// `closed:realized`, …).
    #[must_use]
    pub fn validity_class(&self) -> String {
        match (&self.validity[..], &self.ending) {
            ("closed", Some(e)) => format!("closed:{e}"),
            (v, _) => v.to_owned(),
        }
    }
}

/// `Validity` from its name.
#[must_use]
pub fn validity_of(name: &str) -> Validity {
    match name {
        "no_longer" => Validity::NoLonger,
        "closed" => Validity::Closed,
        _ => Validity::Valid,
    }
}

#[cfg(test)]
mod tests {
    use super::civil;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_000), (2024, 10, 4));
    }
}
