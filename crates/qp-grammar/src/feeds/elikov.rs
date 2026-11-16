//! M9 alternative `elikov`: the feed ordered by what it means **to this reader**.
//!
//! Three groups, top to bottom, each under a short header written here (TR/EN); an empty
//! group is left out:
//!
//! 1. **Your holdings** — capsules about the reader's position or active intent
//!    (`CapsuleItem.relevant`, the authority's audience rule) whose latest transition is new
//!    since the last look (`FeedCtx::fresh`, the core's rule). Without a last look every
//!    such capsule is new to the reader, so all of them come here. Inside: negative stages
//!    (`Stage::is_negative`: weakened, invalidated, retracted) first.
//! 2. **Since your last look** — the other capsules new since the last look.
//! 3. **Everything else, best track record first** — the rest (older news on the holdings
//!    too), ordered by the ledger at birth: capsules with at least
//!    [`GLYPH_MIN_LEDGER_N`] closed cases by their hit rate, highest first; then the ones
//!    with fewer cases (the product calls them "few samples so far") and without a ledger.
//!
//! Ties keep the authority's order (importance, latest transition, id). Marks: "new" on a
//! capsule new since the last look; a group-1 capsule with a negative stage carries the
//! stage in words ("did not hold", …), after "new ·" when it is new. Older negative news
//! carries no mark (it is not news). Rules: relevant, fresh, negative, the ledger order
//! and the mark rule — 5.

use super::{FeedCtx, FeedRow};
use crate::depth::GLYPH_MIN_LEDGER_N;
use qp_schema::capsule::{LedgerStat, Stage};
use qp_schema::protocol::Language;
use std::cmp::Ordering;

/// The group of one capsule: 0 your holdings (new), 1 new since the last look, 2 the rest.
fn group(relevant: bool, fresh: bool, looked: bool) -> u8 {
    if relevant && (fresh || !looked) {
        0
    } else if fresh {
        1
    } else {
        2
    }
}

/// Group-3 order of two ledgers: enough cases before few, then hit rate highest first
/// (exact: cross-multiplied counts).
fn by_ledger(a: &LedgerStat, b: &LedgerStat) -> Ordering {
    let enough = |l: &LedgerStat| l.n >= GLYPH_MIN_LEDGER_N;
    match (enough(a), enough(b)) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => Ordering::Equal,
        (true, true) => {
            (u64::from(b.realized) * u64::from(a.n)).cmp(&(u64::from(a.realized) * u64::from(b.n)))
        }
    }
}

fn header(group: u8, looked: bool, lang: Language) -> &'static str {
    match (group, looked, lang) {
        (0, true, Language::Tr) => "Elindeki hisselerde yeni",
        (0, true, Language::En) => "New in what you hold",
        (0, false, Language::Tr) => "Elindeki hisseler",
        (0, false, Language::En) => "What you hold",
        (1, _, Language::Tr) => "Son bakışından beri",
        (1, _, Language::En) => "Since your last look",
        (_, _, Language::Tr) => "Diğerleri, geçmişte en çok tutan önce",
        (_, _, Language::En) => "The rest, best track record first",
    }
}

fn negative_word(stage: Stage, lang: Language) -> Option<&'static str> {
    Some(match (stage, lang) {
        (Stage::Weakened, Language::Tr) => "zayıfladı",
        (Stage::Weakened, Language::En) => "weakened",
        (Stage::Invalidated, Language::Tr) => "tutmadı",
        (Stage::Invalidated, Language::En) => "did not hold",
        (Stage::Retracted, Language::Tr) => "geri çekildi",
        (Stage::Retracted, Language::En) => "withdrawn",
        _ => return None,
    })
}

fn mark(group: u8, stage: Stage, fresh: bool, lang: Language) -> Option<String> {
    let new = match lang {
        Language::Tr => "yeni",
        Language::En => "new",
    };
    let neg = if group == 0 {
        negative_word(stage, lang)
    } else {
        None
    };
    match (neg, fresh) {
        (Some(w), true) => Some(format!("{new} · {w}")),
        (Some(w), false) => Some(w.to_owned()),
        (None, true) => Some(new.to_owned()),
        (None, false) => None,
    }
}

/// The layout (module docs).
pub(super) fn elikov(c: &FeedCtx<'_>) -> Vec<FeedRow> {
    let looked = c.seen.is_some();
    let fresh = |k: usize| c.fresh.get(k).copied().unwrap_or(false);
    let g = |k: usize| group(c.items[k].relevant, fresh(k), looked);
    let mut order: Vec<usize> = (0..c.items.len()).collect();
    // Stable: ties keep the authority's order.
    order.sort_by(|&a, &b| {
        let (ia, ib) = (&c.items[a], &c.items[b]);
        g(a).cmp(&g(b)).then_with(|| match g(a) {
            0 => ib.stage.is_negative().cmp(&ia.stage.is_negative()),
            1 => Ordering::Equal,
            _ => by_ledger(&ia.ledger_at_birth, &ib.ledger_at_birth),
        })
    });
    let mut rows = Vec::with_capacity(order.len() + 3);
    let mut last: Option<u8> = None;
    for k in order {
        let gk = g(k);
        if last != Some(gk) {
            rows.push(FeedRow::Header {
                text: header(gk, looked, c.lang).to_owned(),
            });
            last = Some(gk);
        }
        let i = &c.items[k];
        rows.push(FeedRow::Capsule {
            id: i.id.clone(),
            mark: mark(gk, i.stage, fresh(k), c.lang),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(n: u32, realized: u32) -> LedgerStat {
        LedgerStat { n, realized }
    }

    #[test]
    fn groups() {
        assert_eq!(group(true, true, true), 0);
        assert_eq!(group(true, false, false), 0);
        assert_eq!(group(true, false, true), 2); // older news on a holding
        assert_eq!(group(false, true, true), 1);
        assert_eq!(group(false, false, true), 2);
    }

    #[test]
    fn ledger_order() {
        // 20/33 (61%) before 13/22 (59%), both before a few-sample 4/5.
        assert_eq!(by_ledger(&l(33, 20), &l(22, 13)), Ordering::Less);
        assert_eq!(by_ledger(&l(5, 4), &l(22, 13)), Ordering::Greater);
        assert_eq!(by_ledger(&l(5, 4), &l(0, 0)), Ordering::Equal);
        assert_eq!(by_ledger(&l(40, 20), &l(20, 10)), Ordering::Equal);
    }

    #[test]
    fn marks_in_both_languages() {
        assert_eq!(
            mark(0, Stage::Invalidated, true, Language::Tr).as_deref(),
            Some("yeni · tutmadı")
        );
        assert_eq!(
            mark(0, Stage::Weakened, false, Language::En).as_deref(),
            Some("weakened")
        );
        // Older negative news, or a negative stage outside the holdings: no stage mark.
        assert_eq!(mark(2, Stage::Invalidated, false, Language::Tr), None);
        assert_eq!(
            mark(1, Stage::Weakened, true, Language::En).as_deref(),
            Some("new")
        );
        assert_eq!(header(0, true, Language::En), "New in what you hold");
        assert_eq!(
            header(2, true, Language::Tr),
            "Diğerleri, geçmişte en çok tutan önce"
        );
    }

    #[test]
    fn empty_feed_has_no_rows() {
        let ctx = FeedCtx {
            items: &[],
            lang: Language::Tr,
            seen: None,
            fresh: &[],
            now: None,
        };
        assert!(elikov(&ctx).is_empty());
    }
}
