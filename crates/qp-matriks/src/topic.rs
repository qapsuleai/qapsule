//! Matriks MQTT topic parsing (`docs/matriks/MESSAGES.md` §1).

/// A parsed Matriks topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    /// `mx/symbol/<SYM>` or `mx/symbol/<SYM>@<level>` (price card,
    /// `SymbolMessage`). `level` is the suffix without `@`, e.g. `lvl2`.
    Symbol {
        /// Matriks symbol code from the topic.
        symbol: String,
        /// Level suffix (`lvl2`), if any.
        level: Option<String>,
    },
    /// `mx/trade/<SYM>[@<level>]` (regular trades) or `mx/tradeex/<SYM>[@<level>]`
    /// (special, reported trades): `TradeMessage` (RP4, K66).
    Trade {
        /// Matriks symbol code from the topic.
        symbol: String,
        /// Level suffix (`lvl0`), if any. No suffix = level 1.
        level: Option<String>,
        /// `mx/tradeex/…`.
        special: bool,
    },
    /// `mx/timestamp` (`TimeMessage`).
    Timestamp,
    /// `mx/messenger` (`MessengerMessage`, broker notices).
    Messenger,
    /// `mx/user/<sub>/messenger` (per-user notices).
    UserMessenger {
        /// The user segment.
        sub: String,
    },
    /// `mx/user/<sub>/mx/...`: a user-specific mirror of a regular topic.
    UserMirror {
        /// The user segment.
        sub: String,
        /// The mirrored topic (`mx/...`).
        inner: Box<Topic>,
    },
    /// Any other topic, kept verbatim.
    Other(String),
}

impl Topic {
    /// The underlying topic of a user mirror, or `self`.
    pub fn unwrap_mirror(&self) -> &Topic {
        match self {
            Topic::UserMirror { inner, .. } => inner.unwrap_mirror(),
            t => t,
        }
    }

    /// Whether this topic carries `MessengerMessage` notices (directly or mirrored).
    pub fn is_messenger(&self) -> bool {
        matches!(
            self.unwrap_mirror(),
            Topic::Messenger | Topic::UserMessenger { .. }
        )
    }
}

/// `<SYM>[@<level>]` → (symbol, level); `None` for an empty part or a `/`.
fn symbol_level(rest: &str) -> Option<(String, Option<String>)> {
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    let (symbol, level) = match rest.split_once('@') {
        Some((s, l)) => (s, Some(l.to_string())),
        None => (rest, None),
    };
    (!symbol.is_empty() && level.as_deref() != Some("")).then(|| (symbol.to_string(), level))
}

/// Which trade stream a topic names (RP4, K66).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TradeStream {
    /// `mx/trade/<SYM>`: regular trades.
    Regular,
    /// `mx/tradeex/<SYM>`: special (reported) trades.
    Special,
}

/// Trade topic level. **There is no level 2**: level 2 carries the buyer and seller
/// brokers (AKDE), which Qapsule does not take (K66); see [`trade_topic_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum TradeLevel {
    /// No suffix: level 1 (price, time, trade number, quantity, side).
    #[default]
    Level1,
    /// `@lvl0`: price and time only.
    Level0,
}

/// Why a trade topic was not built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicError {
    /// Empty symbol, or a symbol with `/`, `@`, `+`, `#` or white space.
    BadSymbol,
    /// Level 2 (`@lvl2`, buyer and seller brokers) is never subscribed (K66).
    Level2Refused,
    /// A level suffix other than `lvl0` / `lvl1` / none.
    UnknownLevel,
}

impl std::fmt::Display for TopicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TopicError::BadSymbol => "bad symbol for a topic",
            TopicError::Level2Refused => {
                "level 2 trade topics (buyer / seller brokers) are never subscribed (K66)"
            }
            TopicError::UnknownLevel => "unknown trade topic level",
        })
    }
}

impl std::error::Error for TopicError {}

fn check_symbol(symbol: &str) -> Result<&str, TopicError> {
    let s = symbol.trim();
    if s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || matches!(c, '/' | '@' | '+' | '#'))
    {
        return Err(TopicError::BadSymbol);
    }
    Ok(s)
}

/// The trade topic of `symbol` at `level` (`mx/trade/<SYM>`, `mx/tradeex/<SYM>@lvl0`, …).
///
/// # Errors
/// [`TopicError::BadSymbol`].
pub fn trade_topic(
    stream: TradeStream,
    symbol: &str,
    level: TradeLevel,
) -> Result<String, TopicError> {
    let s = check_symbol(symbol)?;
    let base = match stream {
        TradeStream::Regular => "mx/trade/",
        TradeStream::Special => "mx/tradeex/",
    };
    Ok(match level {
        TradeLevel::Level1 => format!("{base}{s}"),
        TradeLevel::Level0 => format!("{base}{s}@lvl0"),
    })
}

/// The trade topic from a textual level (`None`, `""`, `lvl1` = no suffix; `lvl0`), e.g.
/// from Topach or a command line. `lvl2` is refused.
///
/// # Errors
/// [`TopicError::Level2Refused`] for `lvl2` (any case, with or without `@`);
/// [`TopicError::UnknownLevel`]; [`TopicError::BadSymbol`].
pub fn trade_topic_at(
    stream: TradeStream,
    symbol: &str,
    level: Option<&str>,
) -> Result<String, TopicError> {
    let lvl = level
        .map(|l| l.trim().trim_start_matches('@').to_ascii_lowercase())
        .unwrap_or_default();
    let level = match lvl.as_str() {
        "" | "lvl1" => TradeLevel::Level1,
        "lvl0" => TradeLevel::Level0,
        "lvl2" => return Err(TopicError::Level2Refused),
        _ => return Err(TopicError::UnknownLevel),
    };
    trade_topic(stream, symbol, level)
}

/// Level-1 trade topics of `symbol`: `mx/trade/<SYM>` and, if `with_special`,
/// `mx/tradeex/<SYM>`.
///
/// # Errors
/// [`TopicError::BadSymbol`].
pub fn trade_topics(symbol: &str, with_special: bool) -> Result<Vec<String>, TopicError> {
    let mut v = vec![trade_topic(
        TradeStream::Regular,
        symbol,
        TradeLevel::Level1,
    )?];
    if with_special {
        v.push(trade_topic(
            TradeStream::Special,
            symbol,
            TradeLevel::Level1,
        )?);
    }
    Ok(v)
}

/// Parses a Matriks topic. Never fails: unknown shapes become [`Topic::Other`].
pub fn parse_topic(topic: &str) -> Topic {
    if topic == "mx/timestamp" {
        return Topic::Timestamp;
    }
    if topic == "mx/messenger" {
        return Topic::Messenger;
    }
    if let Some((symbol, level)) = topic.strip_prefix("mx/symbol/").and_then(symbol_level) {
        return Topic::Symbol { symbol, level };
    }
    for (prefix, special) in [("mx/trade/", false), ("mx/tradeex/", true)] {
        if let Some((symbol, level)) = topic.strip_prefix(prefix).and_then(symbol_level) {
            return Topic::Trade {
                symbol,
                level,
                special,
            };
        }
    }
    if let Some(rest) = topic.strip_prefix("mx/user/") {
        if let Some((sub, tail)) = rest.split_once('/') {
            if !sub.is_empty() {
                if tail == "messenger" {
                    return Topic::UserMessenger {
                        sub: sub.to_string(),
                    };
                }
                if tail.starts_with("mx/") {
                    return Topic::UserMirror {
                        sub: sub.to_string(),
                        inner: Box::new(parse_topic(tail)),
                    };
                }
            }
        }
    }
    Topic::Other(topic.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_topics() {
        assert_eq!(
            parse_topic("mx/symbol/TEST1@lvl2"),
            Topic::Symbol {
                symbol: "TEST1".into(),
                level: Some("lvl2".into())
            }
        );
        assert_eq!(
            parse_topic("mx/symbol/TEST1"),
            Topic::Symbol {
                symbol: "TEST1".into(),
                level: None
            }
        );
        assert_eq!(parse_topic("mx/timestamp"), Topic::Timestamp);
        assert_eq!(parse_topic("mx/messenger"), Topic::Messenger);
        assert_eq!(
            parse_topic("mx/user/u1/messenger"),
            Topic::UserMessenger { sub: "u1".into() }
        );
        let m = parse_topic("mx/user/u1/mx/symbol/TEST2@lvl2");
        assert_eq!(
            m,
            Topic::UserMirror {
                sub: "u1".into(),
                inner: Box::new(Topic::Symbol {
                    symbol: "TEST2".into(),
                    level: Some("lvl2".into())
                })
            }
        );
        assert!(matches!(m.unwrap_mirror(), Topic::Symbol { .. }));
        assert!(parse_topic("mx/user/u1/mx/messenger").is_messenger());
        assert!(parse_topic("mx/user/u1/messenger").is_messenger());
    }

    #[test]
    fn parses_trade_topics() {
        assert_eq!(
            parse_topic("mx/trade/TEST1"),
            Topic::Trade {
                symbol: "TEST1".into(),
                level: None,
                special: false
            }
        );
        assert_eq!(
            parse_topic("mx/tradeex/TEST1@lvl0"),
            Topic::Trade {
                symbol: "TEST1".into(),
                level: Some("lvl0".into()),
                special: true
            }
        );
        assert!(matches!(
            parse_topic("mx/user/u1/mx/trade/TEST2").unwrap_mirror(),
            Topic::Trade { .. }
        ));
    }

    #[test]
    fn trade_topic_builders_never_make_level2() {
        let r = TradeStream::Regular;
        let x = TradeStream::Special;
        assert_eq!(
            trade_topic(r, "TEST1", TradeLevel::Level1).unwrap(),
            "mx/trade/TEST1"
        );
        assert_eq!(
            trade_topic(x, "TEST1", TradeLevel::Level0).unwrap(),
            "mx/tradeex/TEST1@lvl0"
        );
        assert_eq!(
            trade_topics("TEST1", true).unwrap(),
            vec!["mx/trade/TEST1".to_string(), "mx/tradeex/TEST1".to_string()]
        );
        assert_eq!(trade_topic_at(r, "TEST1", None).unwrap(), "mx/trade/TEST1");
        assert_eq!(
            trade_topic_at(r, "TEST1", Some("lvl1")).unwrap(),
            "mx/trade/TEST1"
        );
        assert_eq!(
            trade_topic_at(x, "TEST1", Some("@LVL0")).unwrap(),
            "mx/tradeex/TEST1@lvl0"
        );
        for l in ["lvl2", "@lvl2", " LVL2 "] {
            assert_eq!(
                trade_topic_at(r, "TEST1", Some(l)),
                Err(TopicError::Level2Refused)
            );
            assert_eq!(
                trade_topic_at(x, "TEST1", Some(l)),
                Err(TopicError::Level2Refused)
            );
        }
        assert_eq!(
            trade_topic_at(r, "TEST1", Some("lvl9")),
            Err(TopicError::UnknownLevel)
        );
        // A level smuggled in through the symbol is refused too.
        for s in ["TEST1@lvl2", "", "A/B", "A B", "#", "+"] {
            assert_eq!(
                trade_topic(r, s, TradeLevel::Level1),
                Err(TopicError::BadSymbol),
                "{s}"
            );
        }
    }

    #[test]
    fn unknown_shapes_are_other() {
        for t in [
            "mx/symbol/",
            "mx/symbol/@lvl2",
            "mx/symbol/TEST1@",
            "mx/symbol/A/B",
            "mx/trade/",
            "mx/trade/A/B",
            "mx/tradeex/@lvl0",
            "mx/user//messenger",
            "mx/user/u1",
            "other",
        ] {
            assert_eq!(parse_topic(t), Topic::Other(t.into()), "{t}");
        }
    }
}
