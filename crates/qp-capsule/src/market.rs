//! (R6/T3) The market status in the capsule language (`docs/LANGUAGE.md` "Piyasa durumu",
//! `docs/PROTOCOL.md` §24): what a capsule says about its instrument's session phase or
//! stale data, and the one-line status the shell shows.
//!
//! Pure functions of the [`MarketStatus`] the host gave: no clock, no network. With no
//! status nothing here is called and every sentence stays as it was (byte-equal).
//!
//! Rules:
//! - A capsule speaks of the market only when its instrument is **not** in continuous
//!   trading (its phase, else the market's `open: false`) or the data is stale.
//! - The phase's numeric code never reaches the reader; only its class in words.
//! - Stale data is never shown as live: the capsule says so, and when (`HH:MM`, exchange
//!   time zone). A redacted reader (K33) gets the same without the digits.

use qp_schema::market::{MarketStatus, PhaseClass};
use qp_schema::protocol::Language;

/// The exchange's time zone (the time in "last data HH:MM").
pub const EXCHANGE_TZ: &str = "Europe/Istanbul";

/// Minutes of the delayed feed (BIST delayed data: 15 minutes).
pub const DELAY_MINUTES: u32 = 15;

/// What a capsule says about the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketNote {
    /// Its instrument is not in continuous trading (never [`PhaseClass::Continuous`]).
    Phase(PhaseClass),
    /// The live data is stale (connection down); `last` is the last data's exchange time.
    Stale {
        /// Exchange time of the last data (UTC ms).
        last: i64,
    },
}

/// The note of a capsule on instrument `symbol` under `status`, if it has one: stale data
/// first, then the instrument's own phase (none in continuous trading), then a closed
/// market (`open: false`) for an instrument without a phase.
#[must_use]
pub fn note_of(status: &MarketStatus, symbol: &str) -> Option<MarketNote> {
    if status.stale {
        return Some(MarketNote::Stale {
            last: status.exchange_time,
        });
    }
    match status.phase_of(symbol) {
        Some(PhaseClass::Continuous) => None,
        Some(c) => Some(MarketNote::Phase(c)),
        None if status.open == Some(false) => Some(MarketNote::Phase(PhaseClass::Closed)),
        None => None,
    }
}

/// `HH:MM` of `t` in the exchange's time zone.
#[must_use]
pub fn hhmm(t: i64) -> String {
    let off = qp_calendar::Zone::get(EXCHANGE_TZ).map_or(0, |z| z.offset_at(t.div_euclid(1000)));
    let local_s = t.div_euclid(1000) + i64::from(off);
    let m = local_s.div_euclid(60).rem_euclid(24 * 60);
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// The short clause a headline ends with (after " · "), in `lang`.
#[must_use]
pub fn clause(note: MarketNote, lang: Language) -> &'static str {
    let tr = lang == Language::Tr;
    match note {
        MarketNote::Stale { .. } => {
            if tr {
                "veri bayat"
            } else {
                "data stale"
            }
        }
        MarketNote::Phase(c) => phase_clause(c, lang),
    }
}

/// The short words of phase class `c` (`seans kapalı`, `kapanış seansında`, …).
#[must_use]
pub fn phase_clause(c: PhaseClass, lang: Language) -> &'static str {
    match (c, lang) {
        (PhaseClass::Closed, Language::Tr) => "seans kapalı",
        (PhaseClass::Closed, Language::En) => "session closed",
        (PhaseClass::PreOpen, Language::Tr) => "açılış emir toplamada",
        (PhaseClass::PreOpen, Language::En) => "in the opening auction",
        (PhaseClass::Continuous, Language::Tr) => "sürekli işlemde",
        (PhaseClass::Continuous, Language::En) => "in continuous trading",
        (PhaseClass::ClosingAuction, Language::Tr) => "kapanış seansında",
        (PhaseClass::ClosingAuction, Language::En) => "in the closing auction",
        (PhaseClass::ClosingPriceTrades, Language::Tr) => "kapanış fiyatından işlemde",
        (PhaseClass::ClosingPriceTrades, Language::En) => "trading at the closing price",
        (PhaseClass::Halted, Language::Tr) => "işlem durduruldu",
        (PhaseClass::Halted, Language::En) => "trading halted",
        (PhaseClass::Other, Language::Tr) => "sürekli işlem dışında",
        (PhaseClass::Other, Language::En) => "outside continuous trading",
    }
}

/// The sentence a detail (and an R4 sentence) ends with, in `lang`. `redacted`: no digits
/// (K33; the stale time is left out).
#[must_use]
pub fn sentence(note: MarketNote, lang: Language, redacted: bool) -> String {
    let tr = lang == Language::Tr;
    match note {
        MarketNote::Stale { last } => match (tr, redacted) {
            (true, false) => format!(
                "Bağlantı yok, veri bayat (son veri {}); yazılanlar o ana göre.",
                hhmm(last)
            ),
            (true, true) => "Bağlantı yok, veri bayat; yazılanlar son veriye göre.".to_owned(),
            (false, false) => format!(
                "No connection, data stale (last data {}); this is as of then.",
                hhmm(last)
            ),
            (false, true) => "No connection, data stale; this is as of the last data.".to_owned(),
        },
        MarketNote::Phase(c) => match (c, tr) {
            (PhaseClass::Closed, true) => "Seans kapalı; son seansın verisiyle.",
            (PhaseClass::Closed, false) => "The session is closed; as of the last session.",
            (PhaseClass::PreOpen, true) => {
                "Açılış emir toplamada; fiyat seans açılınca kesinleşir."
            }
            (PhaseClass::PreOpen, false) => {
                "In the opening auction; the price settles when trading opens."
            }
            (PhaseClass::ClosingAuction, true) => {
                "Kapanış seansında; kapanış fiyatı emir toplamayla belirleniyor."
            }
            (PhaseClass::ClosingAuction, false) => {
                "In the closing auction; the closing price is being set."
            }
            (PhaseClass::ClosingPriceTrades, true) => {
                "Kapanış fiyatından işlemde; fiyat artık değişmez."
            }
            (PhaseClass::ClosingPriceTrades, false) => {
                "Trading at the closing price; the price no longer moves."
            }
            (PhaseClass::Halted, true) => "İşlem durduruldu; fiyat durdurulduğu andaki gibi.",
            (PhaseClass::Halted, false) => "Trading is halted; the price is as of the halt.",
            (PhaseClass::Continuous, true) => "Sürekli işlemde.",
            (PhaseClass::Continuous, false) => "In continuous trading.",
            (PhaseClass::Other, true) => "Sürekli işlem dışında.",
            (PhaseClass::Other, false) => "Outside continuous trading.",
        }
        .to_owned(),
    }
}

/// A headline with the note's clause: `"<headline> · <clause>"`.
#[must_use]
pub fn with_clause(headline: &str, note: MarketNote, lang: Language) -> String {
    format!("{headline} · {}", clause(note, lang))
}

/// A text (detail, R4 sentence) with the note's sentence after it.
#[must_use]
pub fn with_sentence(text: &str, note: MarketNote, lang: Language, redacted: bool) -> String {
    let s = sentence(note, lang, redacted);
    if text.is_empty() {
        s
    } else {
        format!("{text} {s}")
    }
}

/// The market's state as the shell's status line shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketState {
    /// Open, real-time data.
    Live,
    /// Open, delayed data.
    Delayed,
    /// The market is closed (or a holiday).
    Closed,
    /// No connection: the data is stale.
    Stale,
}

impl MarketState {
    /// Its machine name (`live`, `delayed`, `closed`, `stale`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            MarketState::Live => "live",
            MarketState::Delayed => "delayed",
            MarketState::Closed => "closed",
            MarketState::Stale => "stale",
        }
    }
}

/// (RP4/T7, A48) Whether the market as a whole is in its closing auction: the price cards
/// put more instruments in the closing auction (order collection for the close) than in
/// any other phase. A few instruments that have no closing session, or one halted, do not
/// make the line say "closed".
#[must_use]
pub fn in_closing_auction(status: &MarketStatus) -> bool {
    let auction = status
        .phases
        .values()
        .filter(|p| p.class() == PhaseClass::ClosingAuction)
        .count();
    auction > 0 && 2 * auction > status.phases.len()
}

/// The shell's one-line market status in `lang` and its state: "Canlı · gecikmeli 15 dk",
/// "Seans kapalı", "Kapanış seansında" (the closing auction, A48), "Bağlantı yok — veri
/// bayat · son veri HH:MM". The closing auction's state is `closed` (no continuous
/// trading; the shell's styling does not change).
#[must_use]
pub fn status_line(status: &MarketStatus, lang: Language) -> (String, MarketState) {
    let tr = lang == Language::Tr;
    if status.stale {
        let t = hhmm(status.exchange_time);
        let s = if tr {
            format!("Bağlantı yok — veri bayat · son veri {t}")
        } else {
            format!("No connection — data stale · last data {t}")
        };
        return (s, MarketState::Stale);
    }
    // (RP4/T7, A48) The continuous session is over but the closing auction is running
    // (18:00–18:10 on BIST): the market is neither "closed" nor live. The phases decide,
    // whatever the calendar's `open` says about that minute.
    if status.holiday != Some(true) && in_closing_auction(status) {
        let s = if tr {
            "Kapanış seansında"
        } else {
            "Closing auction"
        };
        return (s.to_owned(), MarketState::Closed);
    }
    if status.open == Some(false) {
        let s = match (status.holiday == Some(true), tr) {
            (true, true) => "Seans kapalı · tatil",
            (true, false) => "Session closed · holiday",
            (false, true) => "Seans kapalı",
            (false, false) => "Session closed",
        };
        return (s.to_owned(), MarketState::Closed);
    }
    if status.delayed {
        let s = if tr {
            format!("Canlı · gecikmeli {DELAY_MINUTES} dk")
        } else {
            format!("Live · delayed {DELAY_MINUTES} min")
        };
        return (s, MarketState::Delayed);
    }
    let s = if tr { "Canlı" } else { "Live" };
    (s.to_owned(), MarketState::Live)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::market::{MarketSource, PhaseCode};
    use std::collections::BTreeMap;

    // 2026-11-19 15:10 UTC = 18:10 Istanbul.
    const T: i64 = 1_795_101_000_000;

    fn status(open: Option<bool>, stale: bool, phases: &[(&str, i32, &str)]) -> MarketStatus {
        MarketStatus {
            exchange_time: T,
            open,
            holiday: None,
            source: MarketSource::Calendar,
            delayed: true,
            stale,
            phases: phases
                .iter()
                .map(|(s, c, k)| {
                    (
                        (*s).to_owned(),
                        PhaseCode {
                            code: *c,
                            key: (*k).to_owned(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn hhmm_is_exchange_local() {
        assert_eq!(hhmm(T), "18:10");
    }

    #[test]
    fn notes() {
        let s = status(
            Some(true),
            false,
            &[("THYAO", 2, "continuous"), ("GARAN", 5, "closing_session")],
        );
        assert_eq!(note_of(&s, "THYAO"), None);
        assert_eq!(
            note_of(&s, "GARAN"),
            Some(MarketNote::Phase(PhaseClass::ClosingAuction))
        );
        assert_eq!(note_of(&s, "AKBNK"), None);
        let closed = status(Some(false), false, &[]);
        assert_eq!(
            note_of(&closed, "AKBNK"),
            Some(MarketNote::Phase(PhaseClass::Closed))
        );
        let stale = status(Some(true), true, &[("THYAO", 2, "continuous")]);
        assert_eq!(
            note_of(&stale, "THYAO"),
            Some(MarketNote::Stale { last: T })
        );
    }

    #[test]
    fn words() {
        let n = MarketNote::Phase(PhaseClass::Closed);
        assert_eq!(
            with_clause("X aştı", n, Language::Tr),
            "X aştı · seans kapalı"
        );
        assert_eq!(
            with_clause("X rose", n, Language::En),
            "X rose · session closed"
        );
        let st = MarketNote::Stale { last: T };
        assert_eq!(
            with_sentence("Neden.", st, Language::Tr, false),
            "Neden. Bağlantı yok, veri bayat (son veri 18:10); yazılanlar o ana göre."
        );
        let red = sentence(st, Language::En, true);
        assert!(!red.chars().any(|c| c.is_ascii_digit()), "{red}");
        for c in [
            PhaseClass::Closed,
            PhaseClass::PreOpen,
            PhaseClass::ClosingAuction,
            PhaseClass::ClosingPriceTrades,
            PhaseClass::Halted,
            PhaseClass::Other,
        ] {
            for l in [Language::Tr, Language::En] {
                let s = sentence(MarketNote::Phase(c), l, false);
                assert!(!s.chars().any(|c| c.is_ascii_digit()), "{s}");
                assert!(s.ends_with('.'));
            }
        }
    }

    #[test]
    fn status_lines() {
        let live = status(Some(true), false, &[]);
        assert_eq!(
            status_line(&live, Language::Tr),
            ("Canlı · gecikmeli 15 dk".to_owned(), MarketState::Delayed)
        );
        assert_eq!(status_line(&live, Language::En).0, "Live · delayed 15 min");
        let closed = status(Some(false), false, &[]);
        assert_eq!(
            status_line(&closed, Language::Tr),
            ("Seans kapalı".to_owned(), MarketState::Closed)
        );
        // (A48) The closing auction: the line says so, not "closed".
        let auction = status(
            Some(false),
            false,
            &[
                ("THYAO", 5, "closing_session"),
                ("GARAN", 5, "closing_session"),
                ("AKBNK", 1, "closed"),
            ],
        );
        assert_eq!(
            status_line(&auction, Language::Tr),
            ("Kapanış seansında".to_owned(), MarketState::Closed)
        );
        assert_eq!(status_line(&auction, Language::En).0, "Closing auction");
        // Fewer than half in the auction (the session has ended for most): closed.
        let ended = status(
            Some(false),
            false,
            &[
                ("THYAO", 5, "closing_session"),
                ("GARAN", 1, "closed"),
                ("AKBNK", 1, "closed"),
            ],
        );
        assert_eq!(status_line(&ended, Language::Tr).0, "Seans kapalı");
        // A holiday is a holiday; the phases decide whatever the calendar's `open` says.
        let mut hol = auction.clone();
        hol.holiday = Some(true);
        assert_eq!(status_line(&hol, Language::Tr).0, "Seans kapalı · tatil");
        let mut open = auction.clone();
        open.open = Some(true);
        assert_eq!(status_line(&open, Language::Tr).0, "Kapanış seansında");
        // Stale data wins over the auction.
        let mut st = auction.clone();
        st.stale = true;
        assert_eq!(status_line(&st, Language::Tr).1, MarketState::Stale);
        let stale = status(Some(true), true, &[]);
        assert_eq!(
            status_line(&stale, Language::Tr),
            (
                "Bağlantı yok — veri bayat · son veri 18:10".to_owned(),
                MarketState::Stale
            )
        );
        let mut rt = live.clone();
        rt.delayed = false;
        assert_eq!(
            status_line(&rt, Language::En),
            ("Live".to_owned(), MarketState::Live)
        );
    }
}
