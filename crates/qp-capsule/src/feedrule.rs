//! (R5/M19, K53, K42) The feed's coin-flip rule ("yazı tura eşiği"; `docs/CAPSULE.md`
//! §9.10).
//!
//! A story whose ledger is near a coin flip is not shown in the feed below the `all`
//! sensitivity (K53). The rule is written in advance and never looks ahead: it judges the
//! story's **ledger at birth** ([`Story::ledger_at_birth`]: only capsules closed before
//! the birth bar), so a decision made at birth never changes as time moves on, and it is
//! the same ledger the face shows. It never touches the ledger itself (hidden stories keep
//! counting, K53 rule 2). "Few cases" is not "coin flip" (K53 rule 3): a ledger with fewer
//! than `min_n` closed cases is never hidden; the face says it has few cases.
//!
//! The rules are those of the M19 harness (`rounds/R5/experiments/harness/feed/SPEC.md`
//! §2.1) by name; `rounds/R5/experiments/M19/` checks that each gives the harness rule's
//! verdict on every ledger and records the choice (`RESULTS.md`).

use qp_schema::capsule::{LedgerStat, Story};

/// How a ledger is judged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HideRule {
    /// Never hide (the reference; the protocol's behaviour without a rule).
    Never,
    /// Hide when the hit rate is below `tau` (with at least `min_n` closed cases): the
    /// harness's `rate` family.
    Rate {
        /// Fewest closed cases the rule judges (below: few cases, shown).
        min_n: u32,
        /// Hit-rate threshold in `(0, 1)`.
        tau: f64,
    },
}

/// What the rule says about a ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedVerdict {
    /// Shown: enough cases and not near a coin flip.
    Shown,
    /// Shown and said to have few cases (`n < min_n`; K53 rule 3).
    FewCases,
    /// Near a coin flip: hidden below the `all` sensitivity, marked where shown.
    CoinFlip,
}

/// A host's feed rule: the hide rule and the K42 exemption.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeedRule {
    /// The hide rule.
    pub hide: HideRule,
    /// (K42) A negative stage (`weakened`, `invalidated`, `retracted`) of a story about the
    /// reader's position or active intent is never hidden by this rule (it reaches the
    /// person whatever the level, K42/A22). M19 recommends `true`: without it the rule hid
    /// 58 % of those transitions in the real-data simulation.
    pub exempt_negative: bool,
}

/// The named rules a host may choose (`SessionInit.feed_rule.rule`, the presentation's
/// `?feed_rule=`), the presentation's default first. Names are `[a-z][a-z0-9]*`.
pub const FEED_RULES: &[(&str, HideRule)] = &[
    // M19's pick (rate τ = 0.55, N = 20; `rounds/R5/experiments/M19/RESULTS.md`).
    (
        "rate55",
        HideRule::Rate {
            min_n: 20,
            tau: 0.55,
        },
    ),
    // The harness's runner-up (same threshold, N = 10).
    (
        "rate55n10",
        HideRule::Rate {
            min_n: 10,
            tau: 0.55,
        },
    ),
    // The reference: no rule (no hide rule told apart from it on the real data).
    ("none", HideRule::Never),
];

/// The presentation's default rule (M19's pick). The protocol's default is no rule.
pub const PRESENTED_FEED_RULE: &str = "rate55";

impl HideRule {
    /// The rule named `name` ([`FEED_RULES`]).
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        FEED_RULES.iter().find(|(n, _)| *n == name).map(|(_, r)| *r)
    }

    /// The verdict on `ledger`.
    #[must_use]
    pub fn verdict(&self, ledger: LedgerStat) -> FeedVerdict {
        match *self {
            HideRule::Never => FeedVerdict::Shown,
            HideRule::Rate { min_n, tau } => {
                if ledger.n < min_n.max(1) {
                    FeedVerdict::FewCases
                } else if f64::from(ledger.realized) / f64::from(ledger.n) < tau {
                    FeedVerdict::CoinFlip
                } else {
                    FeedVerdict::Shown
                }
            }
        }
    }
}

impl FeedRule {
    /// The rule named `name` with the exemption `exempt_negative`.
    #[must_use]
    pub fn named(name: &str, exempt_negative: bool) -> Option<Self> {
        HideRule::by_name(name).map(|hide| Self {
            hide,
            exempt_negative,
        })
    }

    /// The verdict on `story` (its ledger at birth: no look-ahead).
    #[must_use]
    pub fn verdict(&self, story: &Story) -> FeedVerdict {
        self.hide.verdict(story.ledger_at_birth)
    }

    /// Whether `story` is marked "coin flip".
    #[must_use]
    pub fn coin_flip(&self, story: &Story) -> bool {
        self.verdict(story) == FeedVerdict::CoinFlip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(n: u32, realized: u32) -> LedgerStat {
        LedgerStat { n, realized }
    }

    #[test]
    fn few_cases_are_never_hidden() {
        let r = HideRule::by_name("rate55").unwrap();
        assert_eq!(r.verdict(l(0, 0)), FeedVerdict::FewCases);
        assert_eq!(r.verdict(l(19, 0)), FeedVerdict::FewCases);
        assert_eq!(r.verdict(l(20, 10)), FeedVerdict::CoinFlip);
        assert_eq!(r.verdict(l(20, 11)), FeedVerdict::Shown); // 0.55 is not below 0.55
        assert_eq!(r.verdict(l(40, 21)), FeedVerdict::CoinFlip);
        let r10 = HideRule::by_name("rate55n10").unwrap();
        assert_eq!(r10.verdict(l(10, 5)), FeedVerdict::CoinFlip);
        assert_eq!(r10.verdict(l(9, 0)), FeedVerdict::FewCases);
        assert_eq!(
            HideRule::by_name("none").unwrap().verdict(l(100, 50)),
            FeedVerdict::Shown
        );
        assert!(HideRule::by_name("nope").is_none());
    }

    #[test]
    fn registry() {
        assert_eq!(FEED_RULES[0].0, PRESENTED_FEED_RULE);
        for (i, (a, _)) in FEED_RULES.iter().enumerate() {
            assert!(a
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
            assert!(!FEED_RULES[i + 1..].iter().any(|(b, _)| b == a));
        }
    }
}
