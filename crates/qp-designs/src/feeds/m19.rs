//! R5/M19 feed orders (K53; `rounds/R5/experiments/M19/RESULTS.md`): the two orderings the
//! M19 harness ranked first and second for the persona tasks ("what changed since I last
//! looked", "is anything negative on what I hold"), by the harness's names.
//!
//! - `live_new` (harness `LiveNew`, acceptance criterion 7's "live and new first"): open
//!   claims that still hold first, then open ones that no longer hold, then closed ones
//!   ([`Validity`]); inside, the newest latest transition first (`CapsuleItem.t`), then the
//!   capsule id. No headers, no marks.
//! - `grouped` (harness `Grouped`, research F15: one row per instrument): the capsules of
//!   one instrument stand together; instruments by their newest latest transition (then
//!   the symbol), inside an instrument `live_new`. An instrument with more than one capsule
//!   gets a header with its symbol; a single capsule needs none.
//!
//! Pure and deterministic (same items → same rows; ties by id). Neither reads the last
//! look: what is new is on top by construction.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use qp_schema::capsule::Validity;
use qp_schema::protocol::CapsuleItem;

use super::{FeedCtx, FeedRow};

/// Rules of `live_new`: validity rank, newest first.
pub const LIVE_NEW_RULES: u32 = 2;
/// Rules of `grouped`: instrument groups by newest, the header rule, plus `live_new`.
pub const GROUPED_RULES: u32 = 4;

fn live_rank(v: Validity) -> u8 {
    match v {
        Validity::Valid => 0,
        Validity::NoLonger => 1,
        Validity::Closed => 2,
    }
}

fn live_new_cmp(a: &CapsuleItem, b: &CapsuleItem) -> Ordering {
    live_rank(a.validity)
        .cmp(&live_rank(b.validity))
        .then(b.t.cmp(&a.t))
        .then(a.id.cmp(&b.id))
}

fn row(i: &CapsuleItem) -> FeedRow {
    FeedRow::Capsule {
        id: i.id.clone(),
        mark: None,
    }
}

/// `live_new` (module docs).
pub(super) fn live_new(c: &FeedCtx<'_>) -> Vec<FeedRow> {
    let mut v: Vec<&CapsuleItem> = c.items.iter().collect();
    v.sort_by(|a, b| live_new_cmp(a, b));
    v.into_iter().map(row).collect()
}

/// `grouped` (module docs).
pub(super) fn grouped(c: &FeedCtx<'_>) -> Vec<FeedRow> {
    let mut groups: BTreeMap<&str, Vec<&CapsuleItem>> = BTreeMap::new();
    for i in c.items {
        groups.entry(i.instrument.as_str()).or_default().push(i);
    }
    let mut order: Vec<(&str, Vec<&CapsuleItem>)> = groups.into_iter().collect();
    for (_, g) in &mut order {
        g.sort_by(|a, b| live_new_cmp(a, b));
    }
    let newest = |g: &[&CapsuleItem]| g.iter().map(|i| i.t).max().unwrap_or(i64::MIN);
    order.sort_by(|(sa, ga), (sb, gb)| newest(gb).cmp(&newest(ga)).then(sa.cmp(sb)));
    let mut rows = Vec::with_capacity(c.items.len() + order.len());
    for (sym, g) in order {
        if g.len() > 1 {
            rows.push(FeedRow::Header {
                text: sym.to_owned(),
            });
        }
        rows.extend(g.into_iter().map(row));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::protocol::Language;

    fn item(id: &str, sym: &str, t: i64, validity: Validity) -> CapsuleItem {
        serde_json::from_value(serde_json::json!({
            "id": id, "finding": id, "instrument": sym, "series": format!("{sym}.1d"),
            "kind": "breakout", "direction": "up", "time": "event", "stage": "born",
            "state": "live", "tier": 2, "importance": 0.5, "confirmations": 1,
            "trust": "replica", "born_bar": 1, "last_bar": 1, "t": t,
            "ledger_at_birth": {"n": 0, "realized": 0},
            "sentence": {"text": "", "tr": "", "en": ""}, "validity": validity,
        }))
        .unwrap_or_else(|e| panic!("item: {e}"))
    }

    fn ctx(items: &[CapsuleItem]) -> FeedCtx<'_> {
        FeedCtx {
            items,
            lang: Language::Tr,
            seen: None,
            fresh: &[],
            now: None,
            local: &[],
            now_local: None,
            seen_local: None,
        }
    }

    fn ids(rows: &[FeedRow]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                FeedRow::Header { text } => format!("#{text}"),
                FeedRow::Capsule { id, .. } => id.clone(),
            })
            .collect()
    }

    #[test]
    fn live_new_puts_live_and_new_first_deterministically() {
        let items = [
            item("a", "AAA", 10, Validity::Valid),
            item("b", "BBB", 30, Validity::Closed),
            item("c", "CCC", 20, Validity::NoLonger),
            item("d", "DDD", 40, Validity::Valid),
            item("e", "EEE", 40, Validity::Valid),
        ];
        assert_eq!(ids(&live_new(&ctx(&items))), ["d", "e", "a", "c", "b"]);
        // Same items in another order: same rows.
        let mut rev = items.to_vec();
        rev.reverse();
        assert_eq!(live_new(&ctx(&rev)), live_new(&ctx(&items)));
    }

    #[test]
    fn grouped_keeps_an_instrument_together_newest_instrument_first() {
        let items = [
            item("a1", "AAA", 10, Validity::Valid),
            item("b1", "BBB", 30, Validity::Valid),
            item("a2", "AAA", 50, Validity::Closed),
            item("c1", "CCC", 20, Validity::Valid),
        ];
        assert_eq!(
            ids(&grouped(&ctx(&items))),
            ["#AAA", "a1", "a2", "b1", "c1"]
        );
        let mut rev = items.to_vec();
        rev.reverse();
        assert_eq!(grouped(&ctx(&rev)), grouped(&ctx(&items)));
        assert!(grouped(&ctx(&[])).is_empty());
    }

    /// (R5/fix47) Why the presentation's default is `live_new` and not `grouped`
    /// (`feeds::PRESENTED_FEED_ORDER`): `grouped` is live-first inside an instrument but
    /// orders instruments by their newest transition whatever its validity: an instrument
    /// whose only news is a closed (or no longer holding) story stands above one with a
    /// live claim. `live_new` puts every live claim first.
    #[test]
    fn grouped_can_put_a_closed_instrument_above_a_live_one() {
        let items = [
            item("live", "AAA", 20, Validity::Valid),
            item("gone", "BBB", 30, Validity::Closed),
            item("lapsed", "CCC", 25, Validity::NoLonger),
        ];
        assert_eq!(ids(&grouped(&ctx(&items))), ["gone", "lapsed", "live"]);
        assert_eq!(ids(&live_new(&ctx(&items))), ["live", "lapsed", "gone"]);
        // Inside an instrument both are live first.
        let one = [
            item("x1", "AAA", 30, Validity::Closed),
            item("x2", "AAA", 10, Validity::Valid),
        ];
        assert_eq!(ids(&grouped(&ctx(&one))), ["#AAA", "x2", "x1"]);
        assert_eq!(ids(&live_new(&ctx(&one))), ["x2", "x1"]);
    }
}
