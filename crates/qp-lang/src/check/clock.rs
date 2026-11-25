//! Timeframes and clocks (PR §3.2): canonical spelling (`PROTOCOL.md` §18.2), the allowed
//! higher timeframes of a base, and the clock of a node from its inputs.

use qp_calendar::Interval;

use super::{Base, Cx};
use crate::checked::{Clock, Tf};
use crate::message::Msg;
use crate::reason::Reason;

/// The canonical spelling of an interval: `Nm` (N not a multiple of 60), `Nh`, `1d`,
/// `1w`, `1M`.
pub(crate) fn canonical(iv: Interval) -> String {
    match iv {
        Interval::Minutes(n) if n % 60 == 0 => format!("{}h", n / 60),
        Interval::Minutes(n) => format!("{n}m"),
        Interval::Day => "1d".to_owned(),
        Interval::Week => "1w".to_owned(),
        Interval::Month => "1M".to_owned(),
    }
}

/// A canonical interval string as a [`Tf`]; `Err(Some(canon))` for a valid but
/// non-canonical spelling (`60m`), `Err(None)` for an unknown one.
pub(crate) fn parse_tf(s: &str) -> Result<Tf, Option<String>> {
    let iv = Interval::parse(s).map_err(|_| None)?;
    let name = canonical(iv);
    if name == s {
        Ok(Tf { name, interval: iv })
    } else {
        Err(Some(name))
    }
}

/// Whether `tf` is an allowed higher timeframe of `base` (PR §3.2): larger, and dividing
/// evenly for intraday ones; `1d`, `1w`, `1M` over any intraday base; `1w`, `1M` over `1d`;
/// none over `1w` or `1M`.
pub(crate) fn allowed_above(base: Interval, tf: Interval) -> bool {
    match (base, tf) {
        (Interval::Minutes(n), Interval::Minutes(m)) => m > n && m % n == 0,
        (Interval::Minutes(_), _) => true,
        (Interval::Day, Interval::Week | Interval::Month) => true,
        _ => false,
    }
}

/// Reads an optional `tf` value (already a string) at `at`: `Ok(Clock::Base)` when absent.
pub(super) fn tf_clock(cx: &mut Cx<'_>, tf: Option<&str>, at: &str) -> Option<Clock> {
    let Some(s) = tf else {
        return Some(Clock::Base);
    };
    let tf = match parse_tf(s) {
        Ok(tf) => tf,
        Err(Some(c)) => {
            let m = Msg::NonCanonicalTf(s.to_owned(), c.clone());
            cx.d.at(Reason::UnknownTf, at, &m).dym(vec![c]);
            return None;
        }
        Err(None) => {
            cx.d.at(Reason::UnknownTf, at, &Msg::UnknownTf(s.to_owned()))
                .expected(["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w", "1M"]);
            return None;
        }
    };
    match &cx.base {
        Base::Unbound => Some(Clock::Tf(tf)),
        Base::Bound(None) => {
            cx.d.at(Reason::UnknownTf, at, &Msg::TfUnbound(s.to_owned()));
            None
        }
        Base::Bound(Some(b)) if allowed_above(b.interval, tf.interval) => Some(Clock::Tf(tf)),
        Base::Bound(Some(b)) => {
            let m = Msg::TfNotAbove(s.to_owned(), b.name.clone());
            let f = cx.d.at(Reason::UnknownTf, at, &m);
            if b.name == tf.name {
                f.hint(&Msg::HintOmitTf);
            }
            None
        }
    }
}

/// The common clock of inputs (clockless ones join any clock); `Err((i, a, b))` when
/// input `i`'s clock `b` differs from an earlier `a`. `history` nodes over clockless
/// inputs only run on the base clock (PR §3.2: a constant feeding a state node is `base`).
pub(super) fn merge(clocks: &[&Clock], history: bool) -> Result<Clock, (usize, String, String)> {
    let mut found: Option<&Clock> = None;
    for (i, c) in clocks.iter().enumerate() {
        if matches!(c, Clock::Free) {
            continue;
        }
        match found {
            None => found = Some(c),
            Some(f) if f == *c => {}
            Some(f) => return Err((i, f.name().to_owned(), c.name().to_owned())),
        }
    }
    Ok(match found {
        Some(c) => c.clone(),
        None if history => Clock::Base,
        None => Clock::Free,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spelling_and_order() {
        assert_eq!(parse_tf("4h").map(|t| t.name), Ok("4h".to_owned()));
        assert_eq!(parse_tf("60m").map(|t| t.name), Err(Some("1h".to_owned())));
        assert_eq!(parse_tf("3x").map(|t| t.name), Err(None));
        let m = Interval::Minutes;
        assert!(allowed_above(m(60), m(240)) && !allowed_above(m(60), m(90)));
        assert!(!allowed_above(m(60), m(60)) && !allowed_above(m(60), m(30)));
        assert!(allowed_above(m(5), Interval::Day) && allowed_above(Interval::Day, Interval::Week));
        assert!(!allowed_above(Interval::Week, Interval::Month));
        assert!(!allowed_above(Interval::Day, m(240)));
    }
}
