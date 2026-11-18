//! R4/H-M8910: feed layouts (M9; the `style` gesture's `feed`).
//!
//! A feed layout orders and groups the capsules of the reader's feed: from the `capsules`
//! answer (the authority's order and projection for this reader) it returns the rows the
//! shell stacks top to bottom — capsule rows (a face) and layout rows (a group header, its
//! text written here in the reader's language). A capsule row may carry a short mark text
//! (e.g. "since your last look"); the "last look" time is in the connection's view state
//! (`ViewState.seen`, set with `style.seen`; never logged). The shell does not sort, filter,
//! group or write any of it.
//!
//! The reference `r3` is the `capsules` order as it comes (importance, then latest
//! transition, then id; `docs/PROTOCOL.md` §20.2), no headers, no marks.
//!
//! **Adding an alternative (M9):** in this file only — write the layout function below the
//! `M9 alternatives` marker (or `mod <name>;` with `src/feeds/<name>.rs`) and add one line to
//! [`LAYOUTS`]. `rules` is the layout's own count of rules/parameters (M9's last tiebreak:
//! fewer is better; `r3` has 0). Pure and deterministic: same items, language, `seen` and
//! `now` → same rows.

use qp_schema::protocol::{CapsuleItem, Language};

/// What a layout sees.
#[derive(Debug, Clone, Copy)]
pub struct FeedCtx<'a> {
    /// The reader's feed in the authority's order (`capsules` with `audience: true`).
    pub items: &'a [CapsuleItem],
    /// The reader's language (header and mark texts).
    pub lang: Language,
    /// The reader's last look (ms), if set (`ViewState.seen`).
    pub seen: Option<i64>,
    /// Per item (same order): whether its latest transition became known after the last
    /// look — the core's rule: the transition's bar was closed (the next bar opened) or is
    /// the newest bar, after `seen`; all `false` without a last look. (`CapsuleItem.t` is
    /// the transition **bar's** time, not when it became known.)
    pub fresh: &'a [bool],
    /// The replay clock now (ms), if known.
    pub now: Option<i64>,
    /// (R4/S1) Per item (same order): the local wall time of `CapsuleItem.t` in the
    /// reader's display zone — the core reads the zone's calendar (`qp_calendar`); a layout
    /// never assumes a zone offset.
    pub local: &'a [LocalStamp],
    /// (R4/S1) Local wall time of `now`, if known.
    pub now_local: Option<LocalStamp>,
    /// (R4/S1) Local wall time of `seen`, if set.
    pub seen_local: Option<LocalStamp>,
}

/// (R4/S1) A local wall time: day number (days since 1970-01-01 on the local calendar) and
/// minute of the day. The core fills it from the display zone; weekdays follow from the day
/// number (1970-01-01 was a Thursday).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalStamp {
    /// Local day number.
    pub day: i64,
    /// Minute of the local day, `0..1440`.
    pub minute: u32,
}

impl LocalStamp {
    /// The local wall time of UTC instant `t` (ms) at a fixed offset east of UTC (ms):
    /// for a caller that already knows the zone's offset at `t` (and for tests).
    #[must_use]
    pub fn at_offset(t: i64, offset_ms: i64) -> Self {
        const MS_PER_DAY: i64 = 86_400_000;
        let l = t + offset_ms;
        Self {
            day: l.div_euclid(MS_PER_DAY),
            minute: u32::try_from(l.rem_euclid(MS_PER_DAY) / 60_000).unwrap_or(0),
        }
    }
}

/// One row of the feed, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedRow {
    /// A layout row: a group header, its text in the reader's language.
    Header {
        /// The header text.
        text: String,
    },
    /// A capsule's face.
    Capsule {
        /// Capsule id (one of `FeedCtx::items`).
        id: String,
        /// A short mark shown with the face (e.g. new since the last look), if any.
        mark: Option<String>,
    },
}

/// Builds the rows.
pub type LayoutFn = fn(&FeedCtx<'_>) -> Vec<FeedRow>;

/// One feed layout.
#[derive(Debug, Clone, Copy)]
pub struct FeedLayout {
    /// Setting name (`[a-z][a-z0-9_-]*`, not an ordinary word).
    pub name: &'static str,
    /// Number of rules / parameters the layout uses (M9 tiebreak).
    pub rules: u32,
    /// The layout.
    pub layout: LayoutFn,
}

/// The protocol's default layout (`ViewState.feed` absent; a bare connection).
pub const DEFAULT_FEED: &str = "r3";

/// (R4/S1) The layout the presentation shell asks for when the reader chose none (M9's
/// pick, `rounds/R4/experiments/M9/RESULTS.md`). The shell core sends it with the `style`
/// gesture at start; the protocol default ([`DEFAULT_FEED`]) does not change, so bare
/// connections, transcripts and existing goldens stay byte-identical.
pub const PRESENTED_FEED: &str = "elikov";

/// `r3`: the `capsules` order, nothing added.
fn r3(c: &FeedCtx<'_>) -> Vec<FeedRow> {
    c.items
        .iter()
        .map(|i| FeedRow::Capsule {
            id: i.id.clone(),
            mark: None,
        })
        .collect()
}

// --- M9 alternatives: layout functions (or `mod <name>;`) go below this line -----------

mod elikov;
mod m19;
mod vakiro;

/// Feed layouts a connection may choose; the first is the default.
pub static LAYOUTS: &[FeedLayout] = &[
    FeedLayout {
        name: "r3",
        rules: 0,
        layout: r3,
    },
    // --- M9 alternatives: one line each
    FeedLayout {
        name: "elikov",
        rules: 5,
        layout: elikov::elikov,
    },
    FeedLayout {
        name: "vakiro",
        rules: vakiro::RULES,
        layout: vakiro::layout,
    },
];

/// (R5/M19) The feed orders M19 measured (`feeds/m19.rs`), best first by its pre-written
/// rule (`rounds/R5/experiments/M19/RESULTS.md`). A presentation setting
/// (`?feed_order=`, the settings page), not a protocol `feed` value: the protocol's layout
/// names stay as they were (transcript 56 lists them). When chosen, the order replaces the
/// connection's layout in the presentation core. (R5/fix47) The presentation orders its
/// feed by [`PRESENTED_FEED_ORDER`]; the protocol's layout ([`DEFAULT_FEED`]) and the
/// presented layout ([`PRESENTED_FEED`], `?feed_order=layout`) do not change.
pub static ORDERS: &[FeedLayout] = &[
    FeedLayout {
        name: "grouped",
        rules: m19::GROUPED_RULES,
        layout: m19::grouped,
    },
    FeedLayout {
        name: "live_new",
        rules: m19::LIVE_NEW_RULES,
        layout: m19::live_new,
    },
];

/// The names of [`ORDERS`].
pub const FEED_ORDERS: [&str; 2] = ["grouped", "live_new"];

/// (R5/fix47, acceptance criterion 7) The feed order the presentation opens with: `live_new`,
/// live and new first. M19 measured `grouped` first (0.881 against 0.727), but `grouped`
/// orders instruments by their newest transition whatever its validity, so an instrument
/// whose only news is a closed story stands above one with a live claim
/// (`m19::tests::grouped_can_put_a_closed_instrument_above_a_live_one`); `live_new` puts
/// every claim that still holds first, newest first. A presentation setting: the protocol's
/// default layout does not change; `?feed_order=layout` gives the connection's layout.
pub const PRESENTED_FEED_ORDER: &str = "live_new";

/// (R5/M19) The rows of `ctx` by feed order `name` ([`ORDERS`]; `None`: unknown), with the
/// same clean-up and coin-flip mark as [`layout`].
#[must_use]
pub fn order(name: &str, ctx: &FeedCtx<'_>) -> Option<Vec<FeedRow>> {
    let o = ORDERS.iter().find(|l| l.name == name)?;
    Some(finish((o.layout)(ctx), ctx))
}

/// (R5/M19, K53) The rows of an empty feed: it says honestly that there is nothing to tell
/// now and why — `coin_flip_hidden` stories kept out by the host's coin-flip rule (they
/// show, marked, at the most open sensitivity), or none in what the reader follows. Two
/// header rows (the shell paints them as it paints group headers); deterministic, no
/// advice.
#[must_use]
pub fn empty_rows(lang: Language, coin_flip_hidden: u32) -> Vec<FeedRow> {
    let head = match lang {
        Language::Tr => "Şu an anlatacak bir şey yok.",
        Language::En => "Nothing to tell right now.",
    };
    let why = match (lang, coin_flip_hidden) {
        (Language::Tr, 0) => "İzlediklerinde akışa girecek bir kapsül yok.".to_owned(),
        (Language::En, 0) => "No capsule in what you follow reaches the feed.".to_owned(),
        (Language::Tr, n) => format!(
            "Karnesi yazı turaya yakın {n} kapsül gizlendi; en açık hassasiyette (Hepsi) işaretli görünür."
        ),
        (Language::En, 1) => "1 capsule whose track record is close to a coin flip is hidden; it shows, marked, at the most open sensitivity (All).".to_owned(),
        (Language::En, n) => format!(
            "{n} capsules whose track record is close to a coin flip are hidden; they show, marked, at the most open sensitivity (All)."
        ),
    };
    vec![
        FeedRow::Header {
            text: head.to_owned(),
        },
        FeedRow::Header { text: why },
    ]
}

/// (R5/M19, K53 rule 4) The mark of a capsule whose ledger is near a coin flip (in the feed
/// only at the `all` sensitivity), joined to the layout's own mark.
fn coin_flip_mark(lang: Language) -> &'static str {
    match lang {
        Language::Tr => "karnesi yazı tura",
        Language::En => "coin-flip record",
    }
}

/// Registered names.
#[must_use]
pub fn names() -> Vec<&'static str> {
    LAYOUTS.iter().map(|l| l.name).collect()
}

/// Whether `name` is a known layout.
#[must_use]
pub fn is_feed(name: &str) -> bool {
    LAYOUTS.iter().any(|l| l.name == name)
}

/// The layout named `name` (`None` or unknown: the default).
#[must_use]
pub fn by_name(name: Option<&str>) -> &'static FeedLayout {
    let n = name.unwrap_or(DEFAULT_FEED);
    LAYOUTS.iter().find(|l| l.name == n).unwrap_or(&LAYOUTS[0])
}

/// The rows of `ctx` by layout `name`. Rows naming a capsule that is not in the items, and
/// repeated capsules, are dropped (a layout cannot invent or duplicate a capsule). (R5/M19)
/// A capsule marked `coin_flip` carries the coin-flip mark after the layout's own.
#[must_use]
pub fn layout(name: Option<&str>, ctx: &FeedCtx<'_>) -> Vec<FeedRow> {
    finish((by_name(name).layout)(ctx), ctx)
}

/// Drops rows naming a capsule not in the items and repeats; adds the coin-flip mark.
fn finish(rows: Vec<FeedRow>, ctx: &FeedCtx<'_>) -> Vec<FeedRow> {
    let mut seen: Vec<&str> = Vec::new();
    rows.into_iter()
        .map(|r| match r {
            FeedRow::Capsule { id, mark }
                if ctx.items.iter().any(|i| i.id == id && i.coin_flip) =>
            {
                let cf = coin_flip_mark(ctx.lang);
                let mark = Some(match mark {
                    Some(m) => format!("{m} · {cf}"),
                    None => cf.to_owned(),
                });
                FeedRow::Capsule { id, mark }
            }
            r => r,
        })
        .filter(|r| match r {
            FeedRow::Header { .. } => true,
            FeedRow::Capsule { id, .. } => {
                let ok = ctx.items.iter().any(|i| &i.id == id) && !seen.contains(&id.as_str());
                if ok {
                    if let Some(i) = ctx.items.iter().find(|i| &i.id == id) {
                        seen.push(i.id.as_str());
                    }
                }
                ok
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(LAYOUTS[0].name, DEFAULT_FEED);
        assert!(is_feed(PRESENTED_FEED));
        let n = names();
        for (i, a) in n.iter().enumerate() {
            assert!(!n[i + 1..].contains(a), "duplicate layout {a}");
        }
        let ctx = FeedCtx {
            items: &[],
            lang: Language::Tr,
            seen: None,
            fresh: &[],
            now: None,
            local: &[],
            now_local: None,
            seen_local: None,
        };
        assert!(layout(None, &ctx).is_empty());
        assert_eq!(
            LocalStamp::at_offset(1_695_162_600_000, 3 * 3_600_000),
            LocalStamp {
                day: 19_620,
                minute: 90
            }
        );
        assert!(!is_feed("nope"));
        for (o, l) in FEED_ORDERS.iter().zip(ORDERS) {
            assert_eq!(*o, l.name);
            assert!(!is_feed(o), "an order is not a protocol layout: {o}");
            assert!(order(o, &ctx).is_some_and(|r| r.is_empty()));
        }
        assert!(order("nope", &ctx).is_none());
    }

    #[test]
    fn empty_feed_says_why_in_both_languages() {
        let t = |r: &[FeedRow]| -> Vec<String> {
            r.iter()
                .map(|x| match x {
                    FeedRow::Header { text } => text.clone(),
                    FeedRow::Capsule { id, .. } => id.clone(),
                })
                .collect()
        };
        assert_eq!(
            t(&empty_rows(Language::Tr, 3)),
            [
                "Şu an anlatacak bir şey yok.",
                "Karnesi yazı turaya yakın 3 kapsül gizlendi; en açık hassasiyette (Hepsi) işaretli görünür."
            ]
        );
        assert_eq!(
            t(&empty_rows(Language::En, 0)),
            [
                "Nothing to tell right now.",
                "No capsule in what you follow reaches the feed."
            ]
        );
        assert!(t(&empty_rows(Language::En, 1))[1].starts_with("1 capsule whose"));
        assert_eq!(empty_rows(Language::Tr, 2), empty_rows(Language::Tr, 2));
    }
}
