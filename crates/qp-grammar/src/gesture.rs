//! Pixel gestures on a compiled scene (R4, `docs/PROTOCOL.md` §4.6): a drag in pixels as
//! bars, a pinch centre as the anchor bar, and the visible-range arithmetic of `pan` and
//! `zoom` as pure functions. The authority's bar-space gestures (`pan`, `zoom`) follow
//! the same rules (cross-checked in `qp-session`'s tests).

use qp_schema::scene::XMap;

use crate::scale::slot_at;

/// Bars a horizontal drag of `dx` pixels moves the visible range by: the finger moving
/// right (`dx > 0`) brings older bars into view, so the range moves towards bar 0.
/// `round(-dx / slot)`; 0 for a degenerate slot width.
#[must_use]
pub fn drag_bars(x: &XMap, dx: f64) -> i64 {
    if !(x.slot.is_finite() && x.slot > 0.0 && dx.is_finite()) {
        return 0;
    }
    let b = (-dx / x.slot).round();
    #[allow(clippy::cast_possible_truncation)]
    let b = b.clamp(-1e15, 1e15) as i64;
    b
}

/// The bar a pinch centred at pixel column `px` keeps in place: the slot under it,
/// clamped to the visible range.
#[must_use]
pub fn pinch_anchor(x: &XMap, px: f64) -> u64 {
    if !px.is_finite() {
        return x.to_bar;
    }
    slot_at(x, px).map_or(x.from_bar, |s| s.clamp(x.from_bar, x.to_bar))
}

/// `pan`: the range `from..=to` moved by `bars`, clamped to `0..=last_slot` (its width
/// kept).
#[must_use]
pub fn pan_range(from: u64, to: u64, bars: i64, last_slot: u64) -> (u64, u64) {
    let width = to - from;
    let max_from = last_slot.saturating_sub(width);
    let target = i128::from(from) + i128::from(bars);
    let nf = u64::try_from(target.clamp(0, i128::from(max_from))).unwrap_or(0);
    (nf, nf + width)
}

/// `zoom`: width `round(width / factor)`, at least 2 slots and at most `slots`; `anchor`
/// keeps its relative position in the range. `factor` must be finite and `> 0`, `anchor
/// <= slots - 1`.
#[must_use]
pub fn zoom_range(from: u64, to: u64, factor: f64, anchor: u64, slots: u64) -> (u64, u64) {
    let width = to - from + 1;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let nw = {
        let w = (width as f64 / factor).round();
        let w = if w.is_finite() { w.max(2.0) } else { 2.0 };
        (w.min(slots as f64) as u64).max(1)
    };
    #[allow(clippy::cast_precision_loss)]
    let r = if width > 1 {
        (anchor as f64 - from as f64) / (width - 1) as f64
    } else {
        1.0
    };
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let off = (r * (nw - 1) as f64).round() as i128;
    let max_from = i128::from(slots - nw);
    let nf = u64::try_from((i128::from(anchor) - off).clamp(0, max_from)).unwrap_or(0);
    (nf, nf + nw - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xm(from: u64, to: u64, slot: f64) -> XMap {
        XMap {
            from_bar: from,
            to_bar: to,
            left: 8.0,
            slot,
        }
    }

    #[test]
    fn drag_right_shows_older_bars() {
        let x = xm(100, 159, 5.0);
        assert_eq!(drag_bars(&x, 50.0), -10);
        assert_eq!(drag_bars(&x, -12.0), 2);
        assert_eq!(drag_bars(&x, f64::NAN), 0);
    }

    #[test]
    fn pinch_anchor_is_the_slot_under_the_finger() {
        let x = xm(100, 159, 5.0);
        assert_eq!(pinch_anchor(&x, 8.0 + 5.0 * 10.0 + 1.0), 110);
        assert_eq!(pinch_anchor(&x, 0.0), 100);
        assert_eq!(pinch_anchor(&x, 10_000.0), 159);
    }

    #[test]
    fn ranges_clamp() {
        assert_eq!(pan_range(100, 159, 10, 199), (110, 169));
        assert_eq!(pan_range(100, 159, 1000, 199), (140, 199));
        assert_eq!(pan_range(10, 69, -100, 199), (0, 59));
        assert_eq!(zoom_range(100, 159, 2.0, 159, 200), (130, 159));
        assert_eq!(zoom_range(0, 1, 100.0, 1, 200), (0, 1));
    }
}
