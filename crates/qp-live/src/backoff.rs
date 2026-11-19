//! Deterministic exponential back-off (no jitter) with a failure limit.

use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackoffPolicy {
    /// Delay after the first failure.
    pub initial: Duration,
    /// Multiplier per further failure.
    pub factor: u32,
    /// Upper bound of a single delay.
    pub max: Duration,
    /// After this many consecutive failures the connection halts (`TooManyFailures`).
    pub max_failures: u32,
    /// A session that stayed up this long resets the failure count when it ends.
    pub stable_after: Duration,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(2),
            factor: 2,
            max: Duration::from_secs(60),
            max_failures: 6,
            stable_after: Duration::from_secs(60),
        }
    }
}

impl BackoffPolicy {
    /// Delay after the `failures`-th consecutive failure (1-based).
    pub fn delay_for(&self, failures: u32) -> Duration {
        let mut d = self.initial;
        for _ in 1..failures.max(1) {
            d = d.saturating_mul(self.factor.max(1));
            if d >= self.max {
                return self.max;
            }
        }
        d.min(self.max)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackoffStep {
    Retry { failures: u32, delay: Duration },
    Stop { failures: u32 },
}

#[derive(Clone, Debug)]
pub struct Backoff {
    policy: BackoffPolicy,
    failures: u32,
}

impl Backoff {
    pub fn new(policy: BackoffPolicy) -> Self {
        Self {
            policy,
            failures: 0,
        }
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }

    pub fn policy(&self) -> &BackoffPolicy {
        &self.policy
    }

    pub fn reset(&mut self) {
        self.failures = 0;
    }

    pub fn record_failure(&mut self) -> BackoffStep {
        self.failures += 1;
        if self.failures >= self.policy.max_failures {
            BackoffStep::Stop {
                failures: self.failures,
            }
        } else {
            BackoffStep::Retry {
                failures: self.failures,
                delay: self.policy.delay_for(self.failures),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sequence_is_deterministic_and_capped() {
        let p = BackoffPolicy::default();
        let secs: Vec<u64> = (1..=8).map(|n| p.delay_for(n).as_secs()).collect();
        assert_eq!(secs, vec![2, 4, 8, 16, 32, 60, 60, 60]);
    }

    #[test]
    fn stops_after_max_failures() {
        let mut b = Backoff::new(BackoffPolicy::default());
        for n in 1..6 {
            assert!(
                matches!(b.record_failure(), BackoffStep::Retry { failures, .. } if failures == n)
            );
        }
        assert_eq!(b.record_failure(), BackoffStep::Stop { failures: 6 });
        b.reset();
        assert!(matches!(
            b.record_failure(),
            BackoffStep::Retry { failures: 1, .. }
        ));
    }
}
