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

/// The default layout.
pub const DEFAULT_FEED: &str = "r3";

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
];

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
/// repeated capsules, are dropped (a layout cannot invent or duplicate a capsule).
#[must_use]
pub fn layout(name: Option<&str>, ctx: &FeedCtx<'_>) -> Vec<FeedRow> {
    let mut seen: Vec<&str> = Vec::new();
    let rows = (by_name(name).layout)(ctx);
    rows.into_iter()
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
        };
        assert!(layout(None, &ctx).is_empty());
        assert!(!is_feed("nope"));
    }
}
