//! Seeded pseudo-random numbers (SplitMix64; same generator as the RP2 harness). No OS
//! entropy, no clock: the same seed gives the same sequence on every platform.

/// SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator from a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// An independent stream derived from `seed` and a label (FNV-1a of the label).
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
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        #[allow(clippy::cast_possible_truncation)]
        let r = (self.next_u64() % n as u64) as usize;
        r
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let a: Vec<u64> = (0..5).map(|_| Rng::new(7).next_u64()).collect();
        assert!(a.windows(2).all(|w| w[0] == w[1]));
        let mut r = Rng::derive(1, "x");
        let mut s = Rng::derive(1, "x");
        assert_eq!(r.next_u64(), s.next_u64());
        assert_ne!(
            Rng::derive(1, "x").next_u64(),
            Rng::derive(1, "y").next_u64()
        );
    }
}
