//! Deterministic numeric helpers: IEEE basic operations only (`+ - * /`), `floor`,
//! `round`, `min`, `max` and integer math.
//!
//! `f64::ln`, `log10`, `powf`, `exp` call the platform libm, whose results may differ in
//! the last bits between native targets and wasm32 (R0, `qp-render/SPIKE.md`). The
//! logarithmic scale therefore uses [`log10`] and [`pow10`] written here:
//!
//! - **`log10`**: exponent split from the IEEE bit pattern (`v = m · 2^e`, `m` reduced to
//!   `[√½, √2)`), then `ln m = 2·atanh(s)`, `s = (m − 1)/(m + 1)`, summed as a fixed
//!   16-term odd power series (`|s| ≤ 0.1716`, so the tail is below `1e-25`), and
//!   `log10 v = e·log10(2) + ln(m)·log10(e)`. Accuracy against `f64::log10` is a few ulp
//!   (tested); every call with the same input gives the same bits on every target.
//! - **`pow10`**: `x = n + f` (`n = floor(x)`), `10^f = e^(f·ln 10)` by a fixed 30-term
//!   Taylor series, `10^n` by repeated exact multiplication/division by ten.

use std::f64::consts::{LN_10, LOG10_2, LOG10_E, SQRT_2};

/// Number of odd terms of the `atanh` series in [`log10`].
const LOG_TERMS: u32 = 16;
/// Number of terms of the exponential series in [`pow10`].
const EXP_TERMS: u32 = 30;

/// Deterministic base-10 logarithm of a finite `v > 0`.
///
/// Returns `None` for `v <= 0`, NaN or infinity (the caller counts the value as skipped).
#[must_use]
pub fn log10(v: f64) -> Option<f64> {
    if v.is_nan() || v <= 0.0 || !v.is_finite() {
        return None;
    }
    // Split v = m * 2^e with m in [1, 2). Subnormals are scaled into the normal range first.
    let (mut x, mut e) = (v, 0_i64);
    if x < f64::MIN_POSITIVE {
        x *= 18_014_398_509_481_984.0; // 2^54, exact
        e -= 54;
    }
    let bits = x.to_bits();
    let biased = i64::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    e += biased - 1023;
    // Mantissa with the exponent field set to 1023 (value in [1, 2)); exact.
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023_u64 << 52));
    if m > SQRT_2 {
        m /= 2.0; // exact
        e += 1;
    }
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    // Horner form of s * (1 + s^2/3 + s^4/5 + ...), fixed number of terms, highest first.
    let mut acc = 0.0_f64;
    let mut k = LOG_TERMS;
    while k > 0 {
        k -= 1;
        acc = acc * s2 + 1.0 / f64::from(2 * k + 1);
    }
    let ln_m = 2.0 * s * acc;
    #[allow(clippy::cast_precision_loss)]
    let ef = e as f64;
    Some(ef * LOG10_2 + ln_m * LOG10_E)
}

/// Deterministic `10^x` for finite `x` (saturates to `0` / `inf` far outside the range).
#[must_use]
pub fn pow10(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 400.0 {
        return f64::INFINITY;
    }
    if x < -400.0 {
        return 0.0;
    }
    let n = x.floor();
    let f = x - n;
    let y = f * LN_10;
    // e^y, y in [0, ln 10): Horner form of sum y^k / k!, fixed number of terms.
    let mut acc = 1.0_f64;
    let mut k = EXP_TERMS;
    while k > 0 {
        acc = 1.0 + acc * y / f64::from(k);
        k -= 1;
    }
    #[allow(clippy::cast_possible_truncation)]
    let ni = n as i32;
    acc * pow10_int(ni)
}

/// Exact-as-possible `10^n` for an integer `n` by repeated multiplication.
#[must_use]
pub fn pow10_int(n: i32) -> f64 {
    let mut p = 1.0_f64;
    for _ in 0..n.unsigned_abs().min(400) {
        p *= 10.0;
    }
    if n < 0 {
        1.0 / p
    } else {
        p
    }
}

/// Picks a 1/2/5 × 10^k step `>= raw` without `log10`/`powf` (identical to R0's
/// `qp_render::scene::nice_step`).
#[must_use]
pub fn nice_step(raw: f64) -> f64 {
    // Guard (security review): +inf would never leave the first loop; NaN, <= 0 and
    // subnormal inputs would end with a zero or non-finite step. Callers check the result.
    if !raw.is_finite() || raw < f64::MIN_POSITIVE * 1e16 {
        return f64::NAN;
    }
    let mut mag = 1.0_f64;
    while mag * 10.0 <= raw {
        mag *= 10.0;
    }
    while mag > raw {
        mag /= 10.0;
    }
    let r = raw / mag;
    let m = if r <= 1.0 {
        1.0
    } else if r <= 2.0 {
        2.0
    } else if r <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = m * mag;
    if step.is_finite() {
        step
    } else {
        f64::NAN
    }
}

/// Days since 1970-01-01 to (year, month). Howard Hinnant's `civil_from_days`, integer-only
/// (identical to R0).
#[must_use]
pub fn year_month(days: i64) -> (i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m)
}

/// Saturating `f64 -> i32` after rounding (Rust `as` saturates; NaN maps to 0).
#[must_use]
pub fn px(v: f64) -> i32 {
    // Clamped to ±2^24 so that the small offsets added to pixel coordinates (labels, hit
    // boxes) can never overflow `i32`, whatever the data (security review).
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-PX_LIMIT, PX_LIMIT) as i32;
    r
}

/// Largest pixel coordinate magnitude the compiler emits.
pub const PX_LIMIT: f64 = 16_777_216.0;

/// A number as the shortest string that parses back to it (`70`, `0.5`), as `qp/0`
/// writes numeric constants.
#[must_use]
pub fn num(v: f64) -> String {
    format!("{v}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log10_matches_std_within_ulps() {
        let mut v = 1e-300_f64;
        let mut worst = 0.0_f64;
        while v < 1e300 {
            for m in [1.0, 1.1, 1.414, 1.5, 2.0, 3.3, 5.0, 7.77, 9.99] {
                let x = v * m;
                let ours = log10(x).unwrap();
                let std = x.log10();
                let err = (ours - std).abs() / std.abs().max(1.0);
                worst = worst.max(err);
            }
            v *= 10.0;
        }
        assert!(worst < 4.0 * f64::EPSILON, "worst relative error {worst:e}");
        // Exact powers of ten land exactly or within one ulp.
        for k in -20..=20 {
            let x = pow10_int(k);
            let l = log10(x).unwrap();
            assert!((l - f64::from(k)).abs() <= 4.0 * f64::EPSILON * f64::from(k.abs().max(1)));
        }
    }

    #[test]
    fn log10_rejects_non_positive() {
        assert_eq!(log10(0.0), None);
        assert_eq!(log10(-1.0), None);
        assert_eq!(log10(f64::NAN), None);
        assert_eq!(log10(f64::INFINITY), None);
        assert!(log10(5e-324).is_some()); // smallest subnormal
        let l = log10(5e-324).unwrap();
        assert!((l - 5e-324_f64.log10()).abs() < 1e-12);
    }

    #[test]
    fn log10_is_bit_stable() {
        let xs = [0.1, 1.0, 2.5, 123.456, 9.999e15, 3.3e-7, 400.0, 10.0];
        let a: Vec<u64> = xs.iter().map(|x| log10(*x).unwrap().to_bits()).collect();
        for _ in 0..3 {
            let b: Vec<u64> = xs.iter().map(|x| log10(*x).unwrap().to_bits()).collect();
            assert_eq!(a, b);
        }
        // Pinned bit patterns: a change in the algorithm must be deliberate.
        assert_eq!(log10(10.0).unwrap(), 1.0);
        assert_eq!(log10(1.0).unwrap(), 0.0);
    }

    #[test]
    fn pow10_matches_std() {
        let mut x = -30.0_f64;
        while x < 30.0 {
            let ours = pow10(x);
            let std = 10f64.powf(x);
            assert!(
                ((ours - std) / std).abs() < 8.0 * f64::EPSILON,
                "{x}: {ours} vs {std}"
            );
            x += 0.137;
        }
        assert_eq!(pow10(2.0), 100.0);
        assert_eq!(pow10(0.0), 1.0);
    }

    #[test]
    fn helpers() {
        assert_eq!(nice_step(0.7), 1.0);
        assert_eq!(nice_step(3.0), 5.0);
        assert_eq!(nice_step(13.0), 20.0);
        assert_eq!(year_month(0), (1970, 1));
        assert_eq!(year_month(19_782), (2024, 2));
        assert_eq!(num(70.0), "70");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(px(2.5), 3);
        assert_eq!(px(f64::NAN), 0);
    }
}
