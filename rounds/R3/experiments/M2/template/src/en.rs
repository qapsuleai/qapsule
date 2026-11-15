//! English templates (same frame as Turkish).
//!
//! | Class | Template |
//! |---|---|
//! | Event | `{When}, {subject} {past-tense clause}: {kind noun}.` |
//! | Holding state | `{Subject} has been above/below {thr} for {n} bars and is now {v}. {Noun} is still in place.` |
//! | Ended state | `{When}, {subject} crossed above/below {thr}; it fell back below {when} and is now {v}, no longer above {thr}. {Noun} has ended.` |
//! | Expectation | `If {subject} rises above / falls below / closes above {thr} within {n} bars, {noun} is confirmed.` |
//!
//! Then: stage remark, validity tail (events/expectations), ledger, server mark.

use qp_capsule::Lang;
use qp_schema::capsule::{Direction, EvidenceFamily, MeaningKind};

use crate::num::{fmt, int, Cat};
use crate::{key_cat, Ago, Class, LedgerView, Plan, Remark, Subject, Tendency, Vague, Validity};

const L: Lang = Lang::En;

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => format!("{}{}", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

/// Bare kind noun ("overbought", "upside breakout").
#[must_use]
pub fn noun_bare(kind: MeaningKind, dir: Direction) -> &'static str {
    use Direction::{Down, Neutral, Up};
    use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};
    match (kind, dir) {
        (Continuation, Up) => "uptrend",
        (Continuation, Down) => "downtrend",
        (Continuation, Neutral) => "trend",
        (Reversal, Up) => "upward reversal",
        (Reversal, Down) => "downward reversal",
        (Reversal, Neutral) => "reversal",
        (Breakout, Up) => "upside breakout",
        (Breakout, Down) => "downside breakout",
        (Breakout, Neutral) => "breakout",
        (Squeeze, _) => "squeeze (direction open)",
        (Extreme, Up) => "overbought",
        (Extreme, Down) => "oversold",
        (Extreme, Neutral) => "extreme",
        (Flow, Up) => "money inflow",
        (Flow, Down) => "money outflow",
        (Flow, Neutral) => "unusual money flow",
    }
}

/// Definite noun as a sentence subject ("the overbought state").
fn noun_def(kind: MeaningKind, dir: Direction) -> String {
    match kind {
        MeaningKind::Extreme => format!("the {} state", noun_bare(kind, dir)),
        _ => format!("the {}", noun_bare(kind, dir)),
    }
}

/// Noun after an event clause.
fn noun_event(kind: MeaningKind, dir: Direction) -> String {
    match (kind, dir) {
        (MeaningKind::Continuation, Direction::Up | Direction::Down) => {
            format!("a continuation of the {}", noun_bare(kind, dir))
        }
        (MeaningKind::Squeeze, _) => "a squeeze, direction open".into(),
        (MeaningKind::Extreme | MeaningKind::Flow, _) => noun_bare(kind, dir).into(),
        _ => article(noun_bare(kind, dir)),
    }
}

/// `a` / `an` by the first letter.
fn article(w: &str) -> String {
    if w.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {w}")
    } else {
        format!("a {w}")
    }
}

/// A point in time: `today`, `yesterday`, `3 bars ago`; redacted without digits.
#[must_use]
pub fn when(a: Ago) -> String {
    match a.n {
        0 if a.daily => "today".into(),
        0 => "on this bar".into(),
        1 if a.yesterday => "yesterday".into(),
        1 if a.redacted => "a bar ago".into(),
        1 => "1 bar ago".into(),
        n if !a.redacted => format!("{} bars ago", int(n)),
        _ => match a.vague() {
            Vague::Few if a.daily => "a few days ago",
            Vague::Few => "a few bars ago",
            Vague::Week => "about a week ago",
            Vague::Weeks => "a few weeks ago",
            Vague::Month => "about a month ago",
            Vague::MoreThanMonth => "more than a month ago",
            Vague::While => "a while ago",
            _ => "a long while ago",
        }
        .into(),
    }
}

/// A duration up to now: `since yesterday`, `for 5 bars`.
fn since(a: Ago) -> String {
    match a.n {
        1 if a.yesterday => "since yesterday".into(),
        1 if a.redacted => "for a bar".into(),
        1 => "for 1 bar".into(),
        n if !a.redacted => format!("for {} bars", int(n)),
        _ => match a.vague() {
            Vague::Few if a.daily => "for a few days",
            Vague::Few => "for a few bars",
            Vague::Week => "for about a week",
            Vague::Weeks => "for a few weeks",
            Vague::Month => "for about a month",
            Vague::MoreThanMonth => "for more than a month",
            Vague::While => "for a while",
            _ => "for a long while",
        }
        .into(),
    }
}

fn key_name(p: &Plan<'_>, key: &str, subject: &Subject) -> (&'static str, bool) {
    match key {
        "level" if *subject == Subject::Rsi => (
            if p.dir == Direction::Down {
                "the oversold threshold"
            } else {
                "the overbought threshold"
            },
            true,
        ),
        "level" => ("the breakout level", true),
        "slow" | "sma50" => ("the long average", false),
        "fast" | "sma20" => ("the short average", false),
        "price" | "p1_price" | "p2_price" => ("the reversal point", false),
        "threshold" => ("the threshold", false),
        "avg5" => ("the recent average", false),
        "half_width" => ("the half band width", false),
        "vol_avg" | "avg20" => ("the average volume", false),
        "middle" => ("the middle band", false),
        _ => ("the threshold", true),
    }
}

fn thr(p: &Plan<'_>, with_value: bool) -> String {
    let Some(c) = p.cond.as_ref() else {
        return "the threshold".into();
    };
    let (name, bare) = key_name(p, &c.key, &c.subject);
    let cat = key_cat(&c.key, &c.subject);
    match p.thr() {
        Some(v) if bare => fmt(v, cat, L),
        Some(v) if with_value => format!("{name} ({})", fmt(v, cat, L)),
        _ => name.into(),
    }
}

fn subject(p: &Plan<'_>) -> String {
    let i = p.inst;
    match p.cond.as_ref().map(|c| &c.subject) {
        Some(Subject::Close) | None => i.to_owned(),
        Some(Subject::Rsi) => format!("{i}'s RSI"),
        Some(Subject::Bandwidth) => format!("{i}'s band width"),
        Some(Subject::Net) => format!("net institutional flow in {i}"),
        Some(Subject::CloseDev) => format!("{i}'s distance from the middle band"),
        Some(Subject::Other(s)) => format!("{i}'s {}", s.replace('_', " ")),
    }
}

fn num_opt(v: Option<f64>, cat: Cat) -> Option<String> {
    v.map(|x| fmt(x, cat, L))
}

fn paren(parts: &[(&str, Option<String>)]) -> String {
    let xs: Vec<String> = parts
        .iter()
        .filter_map(|(name, v)| v.as_ref().map(|v| format!("{name} {v}")))
        .collect();
    if xs.is_empty() {
        String::new()
    } else {
        format!(" ({})", xs.join(", "))
    }
}

fn event_clause(p: &Plan<'_>) -> String {
    let i = p.inst;
    let w = cap(&when(p.lead_ago));
    let up = p.dir != Direction::Down;
    let noun = noun_event(p.kind, p.dir);
    match p.kind {
        MeaningKind::Breakout => {
            let (verb, obj) = if up {
                ("broke above", "resistance")
            } else {
                ("broke below", "support")
            };
            let at = num_opt(p.val(&["level"]), Cat::Price)
                .map(|l| format!(" at {l}"))
                .unwrap_or_default();
            let cl = paren(&[("close", num_opt(p.val(&["close"]), Cat::Price))]);
            format!("{w}, {i} {verb} {obj}{at}{cl}: {noun}")
        }
        MeaningKind::Extreme => {
            let lvl = num_opt(p.val(&["level"]), Cat::Osc).unwrap_or_else(|| {
                if up {
                    "the overbought threshold".into()
                } else {
                    "the oversold threshold".into()
                }
            });
            let v = num_opt(p.val(&["value", "rsi"]), Cat::Osc)
                .map(|v| format!(" (to {v})"))
                .unwrap_or_default();
            let verb = if up { "rose above" } else { "fell below" };
            format!("{w}, {i}'s RSI {verb} {lvl}{v}: {noun}")
        }
        MeaningKind::Continuation => {
            let side = if up { "an uptrend" } else { "a downtrend" };
            let vals = paren(&[
                ("close", num_opt(p.val(&["close"]), Cat::Price)),
                (
                    "long average",
                    num_opt(p.val(&["slow", "sma50"]), Cat::Price),
                ),
            ]);
            format!("{w}, {i}'s moving averages lined up for {side}{vals}: {noun}")
        }
        MeaningKind::Reversal => {
            let (a, b) = if up {
                ("a new low", "a higher low")
            } else {
                ("a new high", "a lower high")
            };
            let vals = paren(&[
                (
                    "price",
                    num_opt(p.val(&["p2_price", "price", "close"]), Cat::Price),
                ),
                ("RSI", num_opt(p.val(&["p2_rsi", "rsi"]), Cat::Osc)),
            ]);
            format!("{w}, {i} showed {noun} signal: price made {a} while RSI made {b}{vals}")
        }
        MeaningKind::Squeeze => {
            let vals = paren(&[
                (
                    "band width",
                    num_opt(p.val(&["bandwidth", "bw"]), Cat::Ratio),
                ),
                ("threshold", num_opt(p.val(&["threshold"]), Cat::Ratio)),
            ]);
            format!("{w}, {i}'s Bollinger bands narrowed{vals}: {noun}")
        }
        MeaningKind::Flow if p.has("net") => {
            let side = if up { "buying" } else { "selling" };
            let n = num_opt(p.val(&["net"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let a = num_opt(p.val(&["avg5"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            format!(
                "{w}, net institutional {side} in {i}{n} clearly exceeded the recent average{a}: {noun}"
            )
        }
        MeaningKind::Flow if p.has("volume") => {
            let v = num_opt(p.val(&["volume"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let a = num_opt(p.val(&["vol_avg", "avg20"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            format!("{w}, volume in {i}{v} jumped well above its average{a}: {noun}")
        }
        MeaningKind::Flow => format!("{w}, {i} showed {noun}"),
    }
}

fn holding_clause(p: &Plan<'_>) -> String {
    let noun = cap(&noun_def(p.kind, p.dir));
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "{noun} has been in place in {} {}. It is still in place",
            p.inst,
            since(p.lead_ago)
        );
    };
    let side = if c.above { "above" } else { "below" };
    let s = subject(p);
    let t = thr(p, true);
    let at = if c.subject == Subject::Close {
        "at "
    } else {
        ""
    };
    let main = if p.lead_ago.n == 0 {
        let as_of = if p.lead_ago.daily {
            "as of today"
        } else {
            "as of this bar"
        };
        let cur = p
            .current()
            .map(|v| format!(", now {at}{}", fmt(v, c.subject.cat(), L)))
            .unwrap_or_default();
        format!("{} is {side} {t} {as_of}{cur}", cap(&s))
    } else {
        let cur = p
            .current()
            .map(|v| format!(" and is now {at}{}", fmt(v, c.subject.cat(), L)))
            .unwrap_or_default();
        format!("{} has been {side} {t} {}{cur}", cap(&s), since(p.lead_ago))
    };
    format!("{main}. {noun} is still in place")
}

fn ended_clause(p: &Plan<'_>) -> String {
    let w = cap(&when(p.lead_ago));
    let noun = cap(&noun_def(p.kind, p.dir));
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "{w}, {} started in {}; it no longer holds. {noun} has ended",
            noun_def(p.kind, p.dir),
            p.inst
        );
    };
    let (enter, back, side) = if c.above {
        ("crossed above", "fell back below", "above")
    } else {
        ("crossed below", "rose back above", "below")
    };
    let at = if c.subject == Subject::Close {
        "at "
    } else {
        ""
    };
    let cur = p.current().map(|v| fmt(v, c.subject.cat(), L));
    let t_no = thr(p, false);
    let rest = match (p.exit_ago, cur) {
        (Some(a), Some(v)) => {
            format!(
                "it {back} {} and is now {at}{v}, no longer {side} {t_no}",
                when(a)
            )
        }
        (Some(a), None) => format!("it {back} {} and is no longer {side} {t_no}", when(a)),
        (None, Some(v)) => format!("it is now {at}{v}, no longer {side} {t_no}"),
        (None, None) => format!("it is no longer {side} {t_no}"),
    };
    format!(
        "{w}, {} {enter} {}; {rest}. {noun} has ended",
        subject(p),
        thr(p, true)
    )
}

fn expectation_clause(p: &Plan<'_>) -> String {
    let noun = noun_def(p.kind, p.dir);
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "If its condition is met, {noun} in {} would be confirmed",
            p.inst
        );
    };
    let within = match p.within() {
        Some(n) => format!(" within {} bars", int(u64::from(n))),
        None if p.bounded() => " within a few bars".into(),
        None => String::new(),
    };
    let verb = match (&c.subject, c.above) {
        (Subject::Close, true) => "closes above",
        (Subject::Close, false) => "closes below",
        (_, true) => "rises above",
        (_, false) => "falls below",
    };
    format!(
        "If {} {verb} {}{within}, {noun} would be confirmed",
        subject(p),
        thr(p, true)
    )
}

fn family_name(f: EvidenceFamily) -> &'static str {
    match f {
        EvidenceFamily::Price => "Price",
        EvidenceFamily::Volume => "Volume",
        EvidenceFamily::Flow => "Money flow",
        EvidenceFamily::Derivative => "The derivatives market",
        EvidenceFamily::News => "News",
    }
}

fn remark(p: &Plan<'_>) -> Option<String> {
    let s = when(p.stage_ago);
    let no_longer = p.class != Class::Ended;
    let (who, whose) = if p.class == Class::Expectation {
        ("The expectation", "The expectation's")
    } else {
        ("It", "Its")
    };
    Some(match p.remark {
        Remark::None => return None,
        Remark::Candidate => "Still a candidate: the bar has not closed".into(),
        Remark::Strengthened => {
            format!("It strengthened {s} with new evidence from the same family")
        }
        Remark::Confirmed => {
            let fam = p.latest_family.map_or("A new evidence family", family_name);
            let n = p
                .families()
                .map(|n| {
                    format!(
                        "; {} independent evidence families support it",
                        int(n as u64)
                    )
                })
                .unwrap_or_default();
            format!("{fam} confirmed it {s}{n}")
        }
        Remark::Weakened => format!("An opposite signal appeared {s} and weakened it"),
        Remark::Realized if p.class == Class::Expectation => {
            format!("The expectation came true {s}")
        }
        Remark::Realized => format!("The expected outcome came true {s}"),
        Remark::Invalidated if no_longer => {
            format!("{who} was invalidated {s} and is no longer valid")
        }
        Remark::Invalidated => format!("The signal was invalidated {s}"),
        Remark::Expired => {
            let h = p
                .horizon()
                .map(|h| format!("{}-bar ", int(u64::from(h))))
                .unwrap_or_default();
            let tail = if no_longer {
                "; it is no longer valid"
            } else {
                ""
            };
            format!("{whose} {h}life ran out {s} without an outcome{tail}")
        }
        Remark::Retracted => {
            let tail = if no_longer {
                "; it is no longer valid"
            } else {
                ""
            };
            format!(
                "{who} was withdrawn {s} because the data behind it was corrected, so it does not count in the record{tail}"
            )
        }
    })
}

fn tail(p: &Plan<'_>) -> Option<&'static str> {
    if p.closed_no || matches!(p.class, Class::Holding | Class::Ended) {
        return None;
    }
    Some(match (p.class, p.validity) {
        (Class::Expectation, Validity::Yes) => "The setup is still open",
        (Class::Expectation, Validity::Unknown) => "Whether the setup is still open is unknown",
        (_, Validity::Yes) => "It still holds",
        (_, Validity::No) => "It no longer holds",
        (_, Validity::Unknown) => "Whether it still holds is unknown",
    })
}

fn ledger(p: &Plan<'_>) -> String {
    match p.ledger() {
        LedgerView::Empty => "No track record yet".into(),
        LedgerView::Counts(n, r) => format!(
            "Historically, this signal played out in {} of {} cases",
            int(u64::from(r)),
            int(u64::from(n))
        ),
        LedgerView::Thin(n, r) => format!(
            "Its record is still thin: it played out in {} of {} cases",
            int(u64::from(r)),
            int(u64::from(n))
        ),
        LedgerView::ThinQual => "Its record is still thin".into(),
        LedgerView::Qual(Tendency::Mostly) => {
            "Historically, this kind of signal mostly played out".into()
        }
        LedgerView::Qual(Tendency::Even) => {
            "Historically, this kind of signal played out about as often as not".into()
        }
        LedgerView::Qual(Tendency::Rarely) => {
            "Historically, this kind of signal mostly did not play out".into()
        }
    }
}

fn server(p: &Plan<'_>) -> Option<String> {
    if !p.server {
        return None;
    }
    if !p.redacted {
        return Some("Server-verified".into());
    }
    let lic = match p.entitlement {
        Some(e) if e.starts_with("flow") => "an institutional flow data license",
        _ => "a data license",
    };
    Some(format!("Server-verified; the figures require {lic}"))
}

/// Renders a plan in English.
#[must_use]
pub fn render(p: &Plan<'_>) -> String {
    let main = match p.class {
        Class::Event => event_clause(p),
        Class::Holding => holding_clause(p),
        Class::Ended => ended_clause(p),
        Class::Expectation => expectation_clause(p),
    };
    let mut parts = vec![main];
    parts.extend(remark(p));
    parts.extend(tail(p).map(str::to_owned));
    parts.push(ledger(p));
    parts.extend(server(p));
    let mut out = parts.join(". ");
    out.push('.');
    out
}
