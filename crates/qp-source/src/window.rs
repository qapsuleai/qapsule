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

/// Resolves `window` over `bars` (strictly increasing `t`):
///
/// - `{"last": n}`: the last `min(n, len)` bars;
/// - `{"from_t": t}`: bars with `t >= from_t`;
/// - `{"from_t": a, "to_t": b}`: bars with `a <= t <= b` (closed; no live bars).
///
/// A selection longer than `max_bind_bars` keeps its latest `max_bind_bars` bars and says
/// so in `capped`. Lookups are O(log n).
///
/// # Errors
/// The window breaks [`Window::validate`] (one-of rule, `last` in `1..=max_bind_bars`,
/// `from_t <= to_t`).
pub fn resolve_window(
    bars: &[Bar],
    window: &Window,
    max_bind_bars: u32,
) -> Result<ResolvedWindow, String> {
    window.validate(max_bind_bars)?;
    let len = bars.len();
    let max = usize::try_from(max_bind_bars).unwrap_or(usize::MAX);
    let mut range = match (window.last, window.from_t, window.to_t) {
        (Some(n), _, _) => len.saturating_sub(usize::try_from(n).unwrap_or(usize::MAX))..len,
        (None, Some(a), to) => {
            let lo = bars.partition_point(|b| b.t < a);
            let hi = to.map_or(len, |b| bars.partition_point(|x| x.t <= b));
            lo..hi.max(lo)
        }
        // `validate` rejects every other shape.
        _ => 0..0,
    };
    let mut capped = None;
    if range.len() > max {
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
    let bounds = TimeBounds {
        from: anchor_t.or(window.from_t),
        to: window.to_t,
    };
    Ok(ResolvedWindow {
        range,
        anchor_t,
        capped,
        bounds,
        accepts_live: window.is_open(),
    })
}
