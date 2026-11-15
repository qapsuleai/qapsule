//! Window resolution (`docs/PROTOCOL.md` §18.2, §18.7): which bars of a feed a binding's
//! [`Window`] selects, its anchor, and the `max_bind_bars` cap.

use std::ops::Range;

use qp_schema::data::Window;
use qp_schema::{Bar, TimestampMs};

use crate::feed::TimeBounds;

/// The result of resolving a window over a feed's bars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWindow {
    /// Selected bars as an index range into the given bars (no copy).
    pub range: Range<usize>,
    /// Time of bar 0 of the bound series (the anchor); `None` if the window is empty.
    pub anchor_t: Option<TimestampMs>,
    /// Set when the selection was cut to the latest `max_bind_bars` bars (§18.7); the
    /// text goes to the `binding` query's `reason`.
    pub capped: Option<String>,
    /// Set when the binding is anchored and more than `max_bind_bars` bars lie since the
    /// anchor: the anchor must stay bar 0, so nothing is trimmed and the binding is
    /// `stale` (reason: limit; §18.3, §18.7). `range` still covers all bars since the
    /// anchor.
    pub limit: Option<String>,
    /// True if the binding is anchored but the source has no bar at exactly `anchor_t`
    /// (bar 0 is then the first bar after it; bar numbers may differ from the authority's).
    pub anchor_missing: bool,
    /// Bounds for a [`crate::FeedStore`] holding the window: `from` = anchor (or `from_t`
    /// while empty), `to` = `to_t` of a closed window.
    pub bounds: TimeBounds,
    /// False for a closed window (`to_t`): it accepts no live bars past `to_t`.
    pub accepts_live: bool,
}

impl ResolvedWindow {
    /// True if no bar was selected (the provider answers `no_data`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// The selected bars.
    #[must_use]
    pub fn slice<'a>(&self, bars: &'a [Bar]) -> &'a [Bar] {
        &bars[self.range.clone()]
    }
}

/// Resolves `window` over `bars` (strictly increasing `t`).
///
/// Without an anchor (first resolution):
///
/// - `{"last": n}`: the last `min(n, len)` bars;
/// - `{"from_t": t}`: bars with `t >= from_t`;
/// - `{"from_t": a, "to_t": b}`: bars with `a <= t <= b` (closed; no live bars).
///
/// A selection longer than `max_bind_bars` keeps its latest `max_bind_bars` bars and says
/// so in `capped` (the anchor is that first kept bar, §18.7).
///
/// With `anchor_t` (the binding's anchor, §18.3): bars with `t >= anchor_t` (and
/// `<= to_t` for a closed window); `last` was only the first resolution's size and does
/// not trim the start. More than `max_bind_bars` bars since the anchor are not trimmed:
/// `limit` is set and the binding is `stale`.
///
/// Lookups are O(log n).
///
/// # Errors
/// The window breaks [`Window::validate`] (one-of rule, `last` in `1..=max_bind_bars`,
/// `from_t <= to_t`).
pub fn resolve_window(
    bars: &[Bar],
    window: &Window,
    anchor: Option<TimestampMs>,
    max_bind_bars: u32,
) -> Result<ResolvedWindow, String> {
    window.validate(max_bind_bars)?;
    let len = bars.len();
    let max = usize::try_from(max_bind_bars).unwrap_or(usize::MAX);
    let mut range = match (anchor, window.last, window.from_t, window.to_t) {
        (Some(a), _, _, to) => {
            let lo = bars.partition_point(|b| b.t < a);
            let hi = to.map_or(len, |b| bars.partition_point(|x| x.t <= b));
            lo..hi.max(lo)
        }
        (None, Some(n), _, _) => len.saturating_sub(usize::try_from(n).unwrap_or(usize::MAX))..len,
        (None, None, Some(a), to) => {
            let lo = bars.partition_point(|b| b.t < a);
            let hi = to.map_or(len, |b| bars.partition_point(|x| x.t <= b));
            lo..hi.max(lo)
        }
        // `validate` rejects every other shape.
        _ => 0..0,
    };
    let mut capped = None;
    let mut limit = None;
    // Anchored: the same 2 x max_bind_bars bound as live growth (§18.7), so a restart
    // never turns a ready binding stale.
    if anchor.is_some() && range.len() > max.saturating_mul(2) {
        limit = Some(format!(
            "limit: {} bars since the anchor exceed 2 x limits.max_bind_bars ({}); the anchor stays bar 0, re-anchor by rewriting the binding",
            range.len(),
            max.saturating_mul(2)
        ));
    } else if anchor.is_none() && range.len() > max {
        let selected = range.len();
        range.start = range.end - max;
        capped = Some(format!(
            "the window selects {selected} bars; only the latest {max_bind_bars} (`limits.max_bind_bars`) are kept"
        ));
    }
    let anchor_t = bars
        .get(range.start)
        .filter(|_| !range.is_empty())
        .map(|b| b.t);
    let anchor_missing = anchor.is_some_and(|a| anchor_t != Some(a));
    let bounds = TimeBounds {
        from: anchor.or(anchor_t).or(window.from_t),
        to: window.to_t,
    };
    Ok(ResolvedWindow {
        range,
        anchor_t,
        capped,
        limit,
        anchor_missing,
        bounds,
        accepts_live: window.is_open(),
    })
}
