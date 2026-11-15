//! Per-seed metrics (M1–M13, RESEARCH §4.1) and their aggregation over seeds.
//! Definitions as implemented are in `oracle.rs` and `HARNESS.md`.

use serde::{Deserialize, Serialize};

use crate::channel::ChanStats;

/// Metrics of one run (one scenario, one seed, one approach).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    /// Seed.
    pub seed: u64,
    /// M1: early shows.
    pub m1_early: u64,
    /// M2: contradictory display episodes.
    pub m2_contradictory: u64,
    /// M2: total contradictory display time (ms).
    pub m2_contradictory_ms: f64,
    /// M3: final-labelled shows that are false on the settled truth.
    pub m3_wrong_final: u64,
    /// M3: provisional-labelled shows false on the true live bar at that time.
    pub m3_wrong_provisional: u64,
    /// M1 + M2 + M3 events that happened in the live phase (`now >= LIVE0`), i.e. not
    /// in the start-up burst before the first live bar.
    pub live_errors: u64,
    /// M4: provisional shows withdrawn before the bar closed (not an error).
    pub m4_retracted_provisional: u64,
    /// M5: final-true claims never shown within Δ_miss.
    pub m5_missed: u64,
    /// M5: final-true claims old enough to be judged.
    pub m5_eligible: u64,
    /// M6: provisional show latency (ms), one per live claim.
    pub m6_latency_provisional_ms: Vec<f64>,
    /// M6: final show latency (ms), one per live claim.
    pub m6_latency_final_ms: Vec<f64>,
    /// M7: total time an asserted claim was shown while false (ms).
    pub m7_stale_ms: f64,
    /// M7: stale episodes longer than Δ_stale.
    pub m7_stale_over: u64,
    /// M8: wrong-blocking episodes longer than the threshold.
    pub m8_blocked: u64,
    /// M8: total wrong-blocking time (ms).
    pub m8_blocked_ms: f64,
    /// M9: authority → replica bytes.
    pub m9_a2r: ChanStats,
    /// M9: replica → authority bytes.
    pub m9_r2a: ChanStats,
    /// Live duration (s).
    pub live_secs: f64,
    /// Distinct claims that were final-true at some time (normaliser for M9).
    pub true_claims: u64,
    /// M10: replica work units (deterministic).
    pub m10_work: u64,
    /// M10: replica callbacks.
    pub m10_replica_events: u64,
    /// M10: replica wall time (ns) — NOT deterministic, excluded from the log.
    pub m10_wall_ns: u64,
    /// M11: correction → display latency (ms) per flipped claim resolved.
    pub m11_latency_ms: Vec<f64>,
    /// M11: flipped claims never resolved.
    pub m11_unresolved: u64,
    /// M11: display changes on claims a correction did not flip (within the window).
    pub m11_unneeded: u64,
    /// M11: corrections / rescales in the world.
    pub m11_corrections: u64,
    /// M12: assert/unassert transitions, total.
    pub m12_flips: u64,
    /// M12: maximum transitions of one claim.
    pub m12_max_flips: u64,
    /// M13: time in Disputed / SourceDiffers (ms).
    pub m13_dispute_ms: f64,
    /// Asserting shows.
    pub shows: u64,
    /// FNV-1a 64 of the display log.
    pub log_hash: String,
    /// Display log lines.
    pub log_lines: u64,
}

impl Metrics {
    /// Hard errors (M1 + M2 + M3).
    #[must_use]
    pub fn errors(&self) -> u64 {
        self.m1_early + self.m2_contradictory + self.m3_wrong_final + self.m3_wrong_provisional
    }
}

/// Percentiles of a sample (nearest-rank).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Pct {
    /// Count.
    pub n: usize,
    /// Median.
    pub p50: f64,
    /// 95th.
    pub p95: f64,
    /// 99th.
    pub p99: f64,
    /// Max.
    pub max: f64,
}

impl Pct {
    /// From samples.
    #[must_use]
    pub fn of(mut v: Vec<f64>) -> Self {
        if v.is_empty() {
            return Self::default();
        }
        v.sort_by(f64::total_cmp);
        let at = |q: f64| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1;
            v[i]
        };
        Self {
            n: v.len(),
            p50: at(0.50),
            p95: at(0.95),
            p99: at(0.99),
            max: v[v.len() - 1],
        }
    }
}

/// Aggregate over seeds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Approach.
    pub approach: String,
    /// Scenario.
    pub scenario: String,
    /// Seeds run.
    pub seeds: u64,
    /// Seeds with at least one hard error.
    pub failing_seeds: u64,
    /// Live-phase hard errors (excluding the start-up burst).
    pub live_errors: u64,
    /// Seeds with at least one live-phase hard error.
    pub failing_seeds_live: u64,
    /// Worst seed (most hard errors) and its error count.
    pub worst_seed: Option<(u64, u64)>,
    /// Sums.
    pub m1_early: u64,
    /// M2 count.
    pub m2_contradictory: u64,
    /// M2 ms.
    pub m2_contradictory_ms: f64,
    /// M3 final.
    pub m3_wrong_final: u64,
    /// M3 provisional.
    pub m3_wrong_provisional: u64,
    /// M4.
    pub m4_retracted_provisional: u64,
    /// M5 missed.
    pub m5_missed: u64,
    /// M5 eligible.
    pub m5_eligible: u64,
    /// M6 provisional.
    pub m6_provisional: Pct,
    /// M6 final.
    pub m6_final: Pct,
    /// M7 ms.
    pub m7_stale_ms: f64,
    /// M7 over threshold.
    pub m7_stale_over: u64,
    /// M8 count.
    pub m8_blocked: u64,
    /// M8 ms.
    pub m8_blocked_ms: f64,
    /// M9: raw a→r bytes per final-true claim.
    pub m9_a2r_bytes_per_claim: f64,
    /// M9: raw a→r bytes per live second.
    pub m9_a2r_bytes_per_sec: f64,
    /// M9: per-message deflate a→r bytes per live second.
    pub m9_a2r_deflate_per_sec: f64,
    /// M9: stream deflate a→r bytes per live second.
    pub m9_a2r_stream_deflate_per_sec: f64,
    /// M9: raw r→a bytes per live second.
    pub m9_r2a_bytes_per_sec: f64,
    /// M9: a→r messages per live second.
    pub m9_a2r_msgs_per_sec: f64,
    /// M10: replica work units per replica callback.
    pub m10_work_per_event: f64,
    /// M10: replica wall time per callback (ns, not deterministic).
    pub m10_wall_ns_per_event: f64,
    /// M11 latency.
    pub m11_latency: Pct,
    /// M11 unresolved.
    pub m11_unresolved: u64,
    /// M11 unneeded.
    pub m11_unneeded: u64,
    /// M11 corrections.
    pub m11_corrections: u64,
    /// M12 total.
    pub m12_flips: u64,
    /// M12 max per claim.
    pub m12_max_flips: u64,
    /// M13 ms.
    pub m13_dispute_ms: f64,
    /// Asserting shows.
    pub shows: u64,
    /// Final-true claims.
    pub true_claims: u64,
    /// Wall time of the whole scenario (s, not deterministic).
    pub wall_secs: f64,
}

/// Aggregates per-seed metrics.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn summarize(approach: &str, scenario: &str, runs: &[Metrics], wall_secs: f64) -> Summary {
    let mut s = Summary {
        approach: approach.to_owned(),
        scenario: scenario.to_owned(),
        seeds: runs.len() as u64,
        wall_secs,
        ..Summary::default()
    };
    let (mut lp, mut lf, mut lc) = (Vec::new(), Vec::new(), Vec::new());
    let (mut a2r_raw, mut a2r_def, mut a2r_stream, mut r2a_raw, mut a2r_msgs) = (0, 0, 0, 0, 0);
    let (mut live, mut work, mut events, mut wall) = (0.0, 0u64, 0u64, 0u64);
    for m in runs {
        let e = m.errors();
        if e > 0 {
            s.failing_seeds += 1;
        }
        if e > 0 && s.worst_seed.is_none_or(|(_, w)| e > w) {
            s.worst_seed = Some((m.seed, e));
        }
        s.live_errors += m.live_errors;
        if m.live_errors > 0 {
            s.failing_seeds_live += 1;
        }
        s.m1_early += m.m1_early;
        s.m2_contradictory += m.m2_contradictory;
        s.m2_contradictory_ms += m.m2_contradictory_ms;
        s.m3_wrong_final += m.m3_wrong_final;
        s.m3_wrong_provisional += m.m3_wrong_provisional;
        s.m4_retracted_provisional += m.m4_retracted_provisional;
        s.m5_missed += m.m5_missed;
        s.m5_eligible += m.m5_eligible;
        lp.extend_from_slice(&m.m6_latency_provisional_ms);
        lf.extend_from_slice(&m.m6_latency_final_ms);
        s.m7_stale_ms += m.m7_stale_ms;
        s.m7_stale_over += m.m7_stale_over;
        s.m8_blocked += m.m8_blocked;
        s.m8_blocked_ms += m.m8_blocked_ms;
        a2r_raw += m.m9_a2r.raw;
        a2r_def += m.m9_a2r.deflate_msg;
        a2r_stream += m.m9_a2r.deflate_stream;
        a2r_msgs += m.m9_a2r.msgs;
        r2a_raw += m.m9_r2a.raw;
        live += m.live_secs;
        s.true_claims += m.true_claims;
        work += m.m10_work;
        events += m.m10_replica_events;
        wall += m.m10_wall_ns;
        lc.extend_from_slice(&m.m11_latency_ms);
        s.m11_unresolved += m.m11_unresolved;
        s.m11_unneeded += m.m11_unneeded;
        s.m11_corrections += m.m11_corrections;
        s.m12_flips += m.m12_flips;
        s.m12_max_flips = s.m12_max_flips.max(m.m12_max_flips);
        s.m13_dispute_ms += m.m13_dispute_ms;
        s.shows += m.shows;
    }
    s.m6_provisional = Pct::of(lp);
    s.m6_final = Pct::of(lf);
    s.m11_latency = Pct::of(lc);
    let live = live.max(1e-9);
    s.m9_a2r_bytes_per_claim = a2r_raw as f64 / (s.true_claims.max(1)) as f64;
    s.m9_a2r_bytes_per_sec = a2r_raw as f64 / live;
    s.m9_a2r_deflate_per_sec = a2r_def as f64 / live;
    s.m9_a2r_stream_deflate_per_sec = a2r_stream as f64 / live;
    s.m9_r2a_bytes_per_sec = r2a_raw as f64 / live;
    s.m9_a2r_msgs_per_sec = a2r_msgs as f64 / live;
    s.m10_work_per_event = work as f64 / (events.max(1)) as f64;
    s.m10_wall_ns_per_event = wall as f64 / (events.max(1)) as f64;
    s
}
