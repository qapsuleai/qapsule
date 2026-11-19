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

/// Parses a Matriks topic. Never fails: unknown shapes become [`Topic::Other`].
pub fn parse_topic(topic: &str) -> Topic {
    if topic == "mx/timestamp" {
        return Topic::Timestamp;
    }
    if topic == "mx/messenger" {
        return Topic::Messenger;
    }
    if let Some(rest) = topic.strip_prefix("mx/symbol/") {
        if !rest.is_empty() && !rest.contains('/') {
            let (symbol, level) = match rest.split_once('@') {
                Some((s, l)) => (s, Some(l.to_string())),
                None => (rest, None),
            };
            if !symbol.is_empty() && level.as_deref() != Some("") {
                return Topic::Symbol {
                    symbol: symbol.to_string(),
                    level,
                };
            }
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
    fn unknown_shapes_are_other() {
        for t in [
            "mx/symbol/",
            "mx/symbol/@lvl2",
            "mx/symbol/TEST1@",
            "mx/symbol/A/B",
            "mx/trade/TEST1",
            "mx/user//messenger",
            "mx/user/u1",
            "other",
        ] {
            assert_eq!(parse_topic(t), Topic::Other(t.into()), "{t}");
        }
    }
}
