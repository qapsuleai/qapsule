//! `flow.institutional` v1: unusual institutional (broker) net flow (Flow family,
//! licensed: a server fact, K33).
//!
//! Reads `net = flow.values["net"]` of the bar (a missing flow point, a missing `net`
//! or a non-finite one = no observation). The window holds `|net|` of the last 5
//! *observations* (bars without one are skipped, not counted as 0).
//!
//! - With 5 observations in the window: `avg5 = ((((a0 + a1) + a2) + a3) + a4) / 5`
//!   (oldest first). `net != 0 && |net| > 2 * avg5` → point event, code `net_buy` (`Up`,
//!   `net > 0`) or `net_sell` (`Down`).
//! - Strength: `(|net| / avg5 - 2) / 2` clipped; 1 when `avg5 == 0`.
//! - Evidence `net`, `avg5` (licensed numbers).
//!
//! A bar without an observation emits nothing and leaves the window unchanged, so the
//! state advances deterministically whatever the feed's gaps.

use std::collections::VecDeque;

use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, License, LicenseClass, MeaningKind, TimeKind,
};
use qp_schema::Bar;

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// `flow.institutional` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct InstitutionalFlow {
    window: usize,
    mult: f64,
    /// `|net|` of the last `window` observations (front = oldest).
    abs: VecDeque<f64>,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct InstitutionalFlowStep {
    net: Option<f64>,
}

impl InstitutionalFlow {
    /// Observations averaged.
    pub const WINDOW: usize = 5;
    /// Threshold as a multiple of the average `|net|`.
    pub const MULT: f64 = 2.0;
    /// The flow value read.
    pub const KEY: &'static str = "net";

    /// A detector with `window` observations (at least 1) and threshold `mult`.
    #[must_use]
    pub fn new(window: usize, mult: f64) -> Self {
        let window = window.max(1);
        Self {
            window,
            mult,
            abs: VecDeque::with_capacity(window + 1),
        }
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    fn net(flow: Option<&FlowPoint>) -> Option<f64> {
        flow.and_then(|f| f.values.get(Self::KEY))
            .copied()
            .filter(|v| v.is_finite())
    }
}

impl Default for InstitutionalFlow {
    fn default() -> Self {
        Self::new(Self::WINDOW, Self::MULT)
    }
}

impl Engine for InstitutionalFlow {
    type Step = InstitutionalFlowStep;

    fn id(&self) -> &'static str {
        "flow.institutional"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Flow
    }
    fn license(&self) -> License {
        License {
            class: LicenseClass::Licensed,
            entitlement: Some("flow/*".to_owned()),
        }
    }
    fn warmup(&self) -> usize {
        self.window
    }

    fn eval(
        &self,
        _bar: &Bar,
        flow: Option<&FlowPoint>,
    ) -> (InstitutionalFlowStep, Vec<Detection>) {
        let net = Self::net(flow);
        let mut out = Vec::new();
        if let Some(net) = net {
            if self.abs.len() == self.window {
                let mut sum = 0.0;
                for &a in &self.abs {
                    sum += a;
                }
                #[allow(clippy::cast_precision_loss)]
                let avg = sum / self.window as f64;
                let a = net.abs();
                if net != 0.0 && a > self.mult * avg {
                    out.push(Detection {
                        code: if net > 0.0 { "net_buy" } else { "net_sell" },
                        kind: MeaningKind::Flow,
                        direction: if net > 0.0 {
                            Direction::Up
                        } else {
                            Direction::Down
                        },
                        time: TimeKind::Event,
                        edge: Edge::Point,
                        strength: if avg > 0.0 {
                            clip01((a / avg - self.mult) / self.mult)
                        } else {
                            1.0
                        },
                        evidence: evidence(&[("net", net), ("avg5", avg)]),
                        geometry: Vec::new(),
                        condition: None,
                    });
                }
            }
        }
        (InstitutionalFlowStep { net }, out)
    }

    fn commit(&mut self, _bar: &Bar, step: InstitutionalFlowStep) {
        if let Some(net) = step.net {
            self.abs.push_back(net.abs());
            if self.abs.len() > self.window {
                self.abs.pop_front();
            }
        }
    }
}
