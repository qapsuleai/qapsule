//! Seeded pseudo-random numbers (SplitMix64). No OS entropy, no clock: the same seed
//! always gives the same sequence on every platform (only integer ops and exact
//! conversions; normal variates use the Irwin–Hall sum, no `ln`/`exp`).

/// SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator from a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// A child generator for an independent stream (e.g. one per feed), derived from
    /// this seed and a stream label, without consuming from `self`.
    #[must_use]
    pub fn derive(seed: u64, stream: &str) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in stream.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let mut r = Self::new(seed ^ h.rotate_left(17));
        r.next_u64();
        r
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)` (53 bits).
    pub fn f64(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let x = (self.next_u64() >> 11) as f64;
        x / 9_007_199_254_740_992.0
    }

    /// Uniform in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniform integer in `[0, n)` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.next_u64() % n
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        p > 0.0 && self.f64() < p
    }

    /// Approximately standard normal (Irwin–Hall, 12 uniforms).
    pub fn normal(&mut self) -> f64 {
        let mut s = 0.0;
        for _ in 0..12 {
            s += self.f64();
        }
        s - 6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = Rng::derive(7, "feed");
        let mut d = Rng::derive(7, "feed");
        assert_eq!(c.next_u64(), d.next_u64());
        assert_ne!(
            Rng::derive(7, "a").next_u64(),
            Rng::derive(7, "b").next_u64()
        );
    }
}
