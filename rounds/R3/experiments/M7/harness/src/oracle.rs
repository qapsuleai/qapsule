//! The oracle ("hakem"): knows the true bars at every virtual time (it replays the
//! world's source stream without loss), samples the replica's visible data and display
//! after every virtual instant, derives the display log and computes M1–M13.
//!
//! Definitions as implemented (RESEARCH §1.3, §4.1). "Asserting" labels are
//! Provisional, Final and SourceDiffers; Disputed and Pending assert nothing.
//! A *show* is a transition of a claim into an asserting label, or between two
//! asserting labels.
//!
//! - **Visible holds**: the claim is in the reference detector's claim set computed on
//!   the replica's visible data (RSI from the replica's first visible bar).
//! - **Truth holds**: the claim is in the claim set on the true bars (RSI from the
//!   world's bar 0). *Final-true*: holds and its bar is closed in the truth.
//! - **M1 early**: at a show, the claim's bar (each bar for a pair claim) is not in the
//!   visible data, or fewer than `period` visible bars precede it (evidence window not
//!   visible), or the label is Final and the bar is not closed in the visible data.
//! - **M2 contradictory**: an asserting label is on screen while the claim does not
//!   hold on the visible data; counted once per episode, duration summed.
//! - **M3 wrong**: at a show, Final (or SourceDiffers on a bar closed in the truth) but
//!   not final-true; Provisional (or SourceDiffers on a live bar) but the claim does not
//!   hold on the true bars and has not held within `live_grace_ms` before.
//! - **M4 retracted provisional**: Provisional → non-asserting before the bar's true close.
//! - **M5 missed**: final-true at the end, final-true since `τ0 = max(since, 0)` with
//!   `τ0 + Δ_miss <= end`, and never asserted on screen during `[τ0, τ0 + Δ_miss]`.
//! - **M6 latency**: first Provisional show − first time the claim held on the true live
//!   bar; first final show − time it became final-true. Live-phase claims only, and only
//!   if the claim was true at the show.
//! - **M7 stale**: time an asserting label is on screen while the claim is false in the
//!   truth (Final label: not final-true); episodes longer than Δ_stale counted.
//! - **M8 wrong blocking**: final-true, final on the visible data, yet not asserted;
//!   time summed, episodes longer than the threshold counted.
//! - **M11**: for each correction/rescale instant, claims whose truth flipped; latency
//!   until the display agrees (asserted iff true); display changes within the window on
//!   claims whose bar closed before the correction and which did not flip ("unneeded").
//! - **M12**: assert/unassert transitions per claim. **M13**: time in Disputed or
//!   SourceDiffers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use qp_schema::data::DataChange;

use crate::adapter::ReplicaAdapter;
use crate::detector::{Claim, ClaimDelta, ClaimKey, DetectorCfg, Maturity};
use crate::display::{fmt_values, DisplayItem, Label};
use crate::metrics::Metrics;
use crate::series::SeriesState;
use crate::spec::{ms, ScenarioSpec, VTime, BASE_T, LIVE0};
use crate::world::{StepKind, World, WorldStep};

#[derive(Debug, Default, Clone)]
struct Interval {
    since: Option<VTime>,
}

impl Interval {
    /// Sets the state; returns `Some(duration)` when an episode ends, and whether one
    /// started.
    fn set(&mut self, on: bool, now: VTime) -> (bool, Option<VTime>) {
        match (self.since, on) {
            (None, true) => {
                self.since = Some(now);
                (true, None)
            }
            (Some(s), false) => {
                self.since = None;
                (false, Some(now - s))
            }
            _ => (false, None),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Track {
    truth_holds: bool,
    truth_final: bool,
    first_true_live: Option<VTime>,
    final_since: Option<VTime>,
    false_since: Option<VTime>,
    ever_final: bool,
    vis_holds: bool,
    vis_final: bool,
    label: Option<Label>,
    asserting_since: Option<VTime>,
    asserting: Vec<(VTime, VTime)>,
    contra: Interval,
    stale: Interval,
    blocked: Interval,
    dispute: Interval,
    lat_p_done: bool,
    lat_f_done: bool,
    flips: u64,
}

#[derive(Debug)]
struct CorrRec {
    at: VTime,
    pending: BTreeMap<ClaimKey, bool>,
    flipped: BTreeSet<ClaimKey>,
}

/// The oracle.
#[derive(Debug)]
pub struct Oracle {
    det: DetectorCfg,
    ids: Vec<String>,
    interval_ms: i64,
    period: usize,
    miss: VTime,
    stale_thr: VTime,
    block_thr: VTime,
    corr_window: VTime,
    grace: VTime,
    truth: Vec<SeriesState>,
    truth_claims: BTreeMap<ClaimKey, Claim>,
    vis: Vec<SeriesState>,
    vis_claims: BTreeMap<ClaimKey, Claim>,
    tracks: BTreeMap<ClaimKey, Track>,
    display: BTreeMap<ClaimKey, DisplayItem>,
    dirty: BTreeSet<ClaimKey>,
    corr_before: Option<(VTime, BTreeMap<ClaimKey, bool>)>,
    corrections: Vec<CorrRec>,
    log: Vec<u8>,
    /// Metrics so far.
    pub m: Metrics,
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl Oracle {
    /// A new oracle at time 0 (truth = history).
    #[must_use]
    pub fn new(spec: &ScenarioSpec, world: &World) -> Self {
        let det = DetectorCfg::new(spec.detector.clone());
        let n = world.ids.len();
        let mut o = Self {
            det,
            ids: world.ids.clone(),
            interval_ms: world.interval_ms,
            period: spec.detector.period as usize,
            miss: ms(spec.oracle.miss_ms),
            stale_thr: ms(spec.oracle.stale_ms),
            block_thr: ms(spec.oracle.block_ms),
            corr_window: ms(spec.oracle.corr_window_ms),
            grace: ms(spec.oracle.live_grace_ms),
            truth: (0..n)
                .map(|_| SeriesState::new(spec.detector.period, 0))
                .collect(),
            truth_claims: BTreeMap::new(),
            vis: (0..n)
                .map(|_| SeriesState::new(spec.detector.period, 0))
                .collect(),
            vis_claims: BTreeMap::new(),
            tracks: BTreeMap::new(),
            display: BTreeMap::new(),
            dirty: BTreeSet::new(),
            corr_before: None,
            corrections: Vec::new(),
            log: Vec::new(),
            m: Metrics::default(),
        };
        for b in 0..n {
            o.truth[b].set_bars(world.history[b].clone());
        }
        for b in 0..n {
            o.recompute_truth(0, b, i64::MIN);
        }
        o.log.clear();
        o
    }

    /// True bars of binding `b` now.
    #[must_use]
    pub fn truth_bars(&self, b: usize) -> &[qp_schema::Bar] {
        self.truth[b].bars()
    }

    fn close_vtime(&self, bar_t: i64) -> VTime {
        LIVE0 + (bar_t - BASE_T + self.interval_ms) * 1000
    }

    fn recompute_truth(&mut self, now: VTime, b: usize, from_t: i64) {
        let mut deltas = Vec::new();
        let series: Vec<&SeriesState> = self.truth.iter().collect();
        self.det.recompute(
            &self.ids,
            &series,
            b,
            from_t,
            &mut self.truth_claims,
            &mut deltas,
        );
        for d in deltas {
            self.truth_delta(now, &d);
        }
    }

    fn truth_delta(&mut self, now: VTime, d: &ClaimDelta) {
        if let Some((_, before)) = &mut self.corr_before {
            before.entry(d.key.clone()).or_insert(d.before.is_some());
        }
        let t = self.tracks.entry(d.key.clone()).or_default();
        match &d.after {
            Some(c) => {
                if !t.truth_holds {
                    t.false_since = None;
                }
                t.truth_holds = true;
                if c.maturity == Maturity::Provisional && t.first_true_live.is_none() {
                    t.first_true_live = Some(now);
                }
                let fin = c.maturity == Maturity::Final;
                if fin && !t.truth_final {
                    t.final_since = Some(now);
                    t.ever_final = true;
                }
                t.truth_final = fin;
            }
            None => {
                if t.truth_holds {
                    t.false_since = Some(now);
                }
                t.truth_holds = false;
                t.truth_final = false;
            }
        }
        self.dirty.insert(d.key.clone());
    }

    /// Applies a world step to the truth.
    pub fn on_world(&mut self, now: VTime, step: &WorldStep) {
        let b = step.binding;
        let (ev, info) = match &step.kind {
            StepKind::Close { .. } => return,
            StepKind::Snapshot {
                bars, correction, ..
            } => {
                if *correction && self.corr_before.as_ref().is_none_or(|(at, _)| *at != now) {
                    self.corr_before = Some((now, BTreeMap::new()));
                }
                if self.truth[b].bars() != bars.as_slice() {
                    self.truth[b].set_bars(bars.clone());
                    self.recompute_truth(now, b, i64::MIN);
                }
                return;
            }
            StepKind::Data { ev, info } => (ev, info),
        };
        if info.correction && self.corr_before.as_ref().is_none_or(|(at, _)| *at != now) {
            self.corr_before = Some((now, BTreeMap::new()));
        }
        let s = &mut self.truth[b];
        let old_last = s.bars().last().map(|x| x.t);
        let from = match &ev.change {
            DataChange::Bar { bar } => s.upsert(bar.clone()),
            DataChange::Patch { t, fields } => match s.index_of(*t) {
                Some(i) => {
                    let mut bar = s.bars()[i].clone();
                    fields.apply_to(&mut bar);
                    s.upsert(bar)
                }
                None => return,
            },
        };
        let new_last = s.bars().last().map(|x| x.t);
        let from = match (old_last, new_last) {
            (Some(o), Some(n)) if n > o => o.min(from),
            _ => from,
        };
        self.recompute_truth(now, b, from);
    }

    fn line(&mut self, s: &str) {
        self.m.log_lines += 1;
        self.log.extend_from_slice(s.as_bytes());
        self.log.push(b'\n');
    }

    /// Samples the replica at the end of a virtual instant. `touched`: bindings whose
    /// visible data may have changed (`None`: all).
    pub fn assess(
        &mut self,
        now: VTime,
        rep: &dyn ReplicaAdapter,
        touched: Option<&BTreeSet<usize>>,
    ) {
        // 1. Visible data.
        let all: Vec<usize> = (0..self.ids.len()).collect();
        let bs: Vec<usize> = touched.map_or(all, |t| t.iter().copied().collect());
        for b in bs {
            if let Some(from) = self.vis[b].sync_from(rep.visible_data(b)) {
                let mut deltas = Vec::new();
                let series: Vec<&SeriesState> = self.vis.iter().collect();
                self.det.recompute(
                    &self.ids,
                    &series,
                    b,
                    from,
                    &mut self.vis_claims,
                    &mut deltas,
                );
                for d in deltas {
                    let t = self.tracks.entry(d.key.clone()).or_default();
                    t.vis_holds = d.after.is_some();
                    t.vis_final = d
                        .after
                        .as_ref()
                        .is_some_and(|c| c.maturity == Maturity::Final);
                    self.dirty.insert(d.key);
                }
            }
        }
        // 2. Corrections of this instant.
        if let Some((at, before)) = self.corr_before.take() {
            let mut flipped = BTreeSet::new();
            let mut pending = BTreeMap::new();
            for (k, held) in before {
                let holds = self.truth_claims.contains_key(&k);
                if holds != held {
                    flipped.insert(k.clone());
                    pending.insert(k, holds);
                }
            }
            self.m.m11_corrections += 1;
            self.corrections.push(CorrRec {
                at,
                pending,
                flipped,
            });
        }
        // 3. Display.
        let new = &rep.display().items;
        if *new != self.display {
            let mut keys: BTreeSet<ClaimKey> = self.display.keys().cloned().collect();
            keys.extend(new.keys().cloned());
            let old = std::mem::take(&mut self.display);
            for k in keys {
                let (a, b) = (old.get(&k), new.get(&k));
                if a != b {
                    self.transition(now, &k, a, b);
                }
            }
            self.display = new.clone();
        }
        // 4. Intervals.
        let dirty = std::mem::take(&mut self.dirty);
        for k in &dirty {
            self.refresh(now, k);
        }
        // 5. Correction resolution.
        for rec in &mut self.corrections {
            if rec.pending.is_empty() {
                continue;
            }
            let mut done = Vec::new();
            for (k, want) in &rec.pending {
                let shown = self
                    .tracks
                    .get(k)
                    .and_then(|t| t.label)
                    .is_some_and(Label::asserts);
                if shown == *want {
                    done.push(k.clone());
                }
            }
            for k in done {
                rec.pending.remove(&k);
                #[allow(clippy::cast_precision_loss)]
                self.m.m11_latency_ms.push((now - rec.at) as f64 / 1000.0);
            }
        }
    }

    fn final_rule(&self, now: VTime, k: &ClaimKey, label: Label) -> bool {
        label == Label::Final || (label == Label::SourceDiffers && now >= self.close_vtime(k.bar_t))
    }

    fn covered(&self, k: &ClaimKey, label: Label) -> bool {
        k.bindings().iter().all(|id| {
            let Some(b) = self.ids.iter().position(|x| x == id) else {
                return false;
            };
            let s = &self.vis[b];
            match s.index_of(k.bar_t) {
                Some(i) => i >= self.period && (label != Label::Final || i + 1 < s.bars().len()),
                None => false,
            }
        })
    }

    #[allow(clippy::cast_precision_loss)]
    fn transition(
        &mut self,
        now: VTime,
        k: &ClaimKey,
        a: Option<&DisplayItem>,
        b: Option<&DisplayItem>,
    ) {
        let la = a.map(|x| x.label);
        let lb = b.map(|x| x.label);
        let mut s = String::new();
        let _ = write!(
            s,
            "{now} {} {} -> {}",
            k.text(),
            la.map_or("-", Label::name),
            lb.map_or("-", Label::name)
        );
        if let Some(b) = b {
            let _ = write!(s, " {}", fmt_values(&b.shown_values));
        }
        self.line(&s);
        if la == lb {
            return;
        }
        let was = la.is_some_and(Label::asserts);
        let is = lb.is_some_and(Label::asserts);
        if is {
            let lb = lb.expect("asserting");
            self.m.shows += 1;
            let live = now >= LIVE0;
            if !self.covered(k, lb) {
                self.m.m1_early += 1;
                self.m.live_errors += u64::from(live);
            }
            let fr = self.final_rule(now, k, lb);
            let grace = self.grace;
            let live_phase = k.bar_t >= BASE_T;
            let t = self.tracks.entry(k.clone()).or_default();
            if fr {
                if !t.truth_final {
                    self.m.m3_wrong_final += 1;
                    self.m.live_errors += u64::from(live);
                }
            } else if !t.truth_holds && t.false_since.is_none_or(|f| now - f > grace) {
                self.m.m3_wrong_provisional += 1;
                self.m.live_errors += u64::from(live);
            }
            if lb == Label::Provisional && !t.lat_p_done {
                t.lat_p_done = true;
                if let (true, true, Some(f)) = (t.truth_holds, live_phase, t.first_true_live) {
                    self.m
                        .m6_latency_provisional_ms
                        .push((now - f) as f64 / 1000.0);
                }
            }
            if fr && !t.lat_f_done {
                t.lat_f_done = true;
                if let (true, true, Some(f)) = (t.truth_final, live_phase, t.final_since) {
                    self.m.m6_latency_final_ms.push((now - f) as f64 / 1000.0);
                }
            }
        }
        let close = self.close_vtime(k.bar_t);
        let t = self.tracks.entry(k.clone()).or_default();
        if was && !is && la == Some(Label::Provisional) && now < close {
            self.m.m4_retracted_provisional += 1;
        }
        if was != is {
            t.flips += 1;
            self.m.m12_flips += 1;
            if is {
                t.asserting_since = Some(now);
            } else if let Some(s) = t.asserting_since.take() {
                t.asserting.push((s, now));
            }
        }
        t.label = lb;
        for rec in &self.corrections {
            if now <= rec.at + self.corr_window && !rec.flipped.contains(k) && close <= rec.at {
                self.m.m11_unneeded += 1;
            }
        }
        self.dirty.insert(k.clone());
    }

    #[allow(clippy::cast_precision_loss)]
    fn refresh(&mut self, now: VTime, k: &ClaimKey) {
        let Some(t) = self.tracks.get(k) else {
            return;
        };
        let label = t.label;
        let asserting = label.is_some_and(Label::asserts);
        let truth_ok = match label {
            Some(l) if self.final_rule(now, k, l) => t.truth_final,
            _ => t.truth_holds,
        };
        let contra = asserting && !t.vis_holds;
        let stale = asserting && !truth_ok;
        let blocked = t.truth_final && t.vis_final && !asserting;
        let dispute = matches!(label, Some(Label::Disputed | Label::SourceDiffers));
        self.apply_flags(now, k, contra, stale, blocked, dispute);
    }

    #[allow(clippy::cast_precision_loss)]
    fn apply_flags(
        &mut self,
        now: VTime,
        k: &ClaimKey,
        contra: bool,
        stale: bool,
        blocked: bool,
        dispute: bool,
    ) {
        let (stale_thr, block_thr) = (self.stale_thr, self.block_thr);
        let t = self.tracks.get_mut(k).expect("track");
        let (started, ended) = t.contra.set(contra, now);
        if started {
            self.m.m2_contradictory += 1;
            self.m.live_errors += u64::from(now >= LIVE0);
        }
        if let Some(d) = ended {
            self.m.m2_contradictory_ms += d as f64 / 1000.0;
        }
        if let (_, Some(d)) = t.stale.set(stale, now) {
            self.m.m7_stale_ms += d as f64 / 1000.0;
            if d > stale_thr {
                self.m.m7_stale_over += 1;
            }
        }
        if let (_, Some(d)) = t.blocked.set(blocked, now) {
            self.m.m8_blocked_ms += d as f64 / 1000.0;
            if d > block_thr {
                self.m.m8_blocked += 1;
            }
        }
        if let (_, Some(d)) = t.dispute.set(dispute, now) {
            self.m.m13_dispute_ms += d as f64 / 1000.0;
        }
    }

    /// True if the incrementally maintained truth and visible claim sets equal a batch
    /// recomputation (checked in debug builds and tests).
    #[must_use]
    pub fn self_check(&self) -> bool {
        let t: Vec<&SeriesState> = self.truth.iter().collect();
        let v: Vec<&SeriesState> = self.vis.iter().collect();
        self.det.claims_batch(&self.ids, &t) == self.truth_claims
            && self.det.claims_batch(&self.ids, &v) == self.vis_claims
    }

    /// Closes all open intervals at `end` and computes M5, M11 leftovers, M12 max.
    /// Returns the metrics and the display log.
    #[must_use]
    pub fn finish(mut self, end: VTime) -> (Metrics, Vec<u8>) {
        debug_assert!(
            self.self_check(),
            "incremental claim sets differ from batch"
        );
        let keys: Vec<ClaimKey> = self.tracks.keys().cloned().collect();
        for k in &keys {
            self.apply_flags(end, k, false, false, false, false);
            let t = self.tracks.get_mut(k).expect("track");
            if let Some(s) = t.asserting_since.take() {
                t.asserting.push((s, end));
            }
        }
        for (k, c) in &self.truth_claims {
            if c.maturity != Maturity::Final {
                continue;
            }
            let t = &self.tracks[k];
            let tau0 = t.final_since.unwrap_or(0).max(0);
            if tau0 + self.miss > end {
                continue;
            }
            self.m.m5_eligible += 1;
            let hit = t
                .asserting
                .iter()
                .any(|&(a, b)| a <= tau0 + self.miss && b >= tau0);
            if !hit {
                self.m.m5_missed += 1;
            }
        }
        self.m.m11_unresolved = self
            .corrections
            .iter()
            .map(|r| r.pending.len() as u64)
            .sum();
        self.m.m12_max_flips = self.tracks.values().map(|t| t.flips).max().unwrap_or(0);
        self.m.true_claims = self.tracks.values().filter(|t| t.ever_final).count() as u64;
        self.m.log_hash = format!("{:016x}", fnv(&self.log));
        (self.m, self.log)
    }
}
