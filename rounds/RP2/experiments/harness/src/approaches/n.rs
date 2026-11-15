//! Control baseline N ("show on arrival"; RESEARCH §3 N — not a candidate).
//!
//! Authority: runs the shared engine on its feed; every claim-set change is sent at once
//! as JSON (`show` with the claim and its maturity, `hide`). Replica: keeps its own feed
//! in the shared engine only for the chart (visible data) and displays whatever the
//! authority says the moment it arrives. It never checks anything. Its purpose is to
//! prove that each scenario really produces early / contradictory / wrong shows.

use serde::{Deserialize, Serialize};

use crate::adapter::{Approach, AuthorityAdapter, Ctx, ReplicaAdapter, Setup, SourceEvent};
use crate::detector::{Claim, ClaimKey, Maturity};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::Engine;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Op {
    Show { claim: Claim },
    Hide { key: ClaimKey },
}

struct Authority {
    engine: Engine,
}

impl AuthorityAdapter for Authority {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        self.engine.apply(ev);
        let ops: Vec<Op> = self
            .engine
            .take_deltas()
            .into_iter()
            .map(|d| match d.after {
                Some(claim) => Op::Show { claim },
                None => Op::Hide { key: d.key },
            })
            .collect();
        if !ops.is_empty() {
            ctx.send(serde_json::to_vec(&ops).expect("json"));
        }
    }

    fn on_replica_msg(&mut self, _ctx: &mut Ctx, _msg: &[u8]) {}

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}

struct Replica {
    engine: Engine,
    display: DisplayState,
}

impl ReplicaAdapter for Replica {
    fn on_source(&mut self, _ctx: &mut Ctx, ev: &SourceEvent) {
        self.engine.apply(ev);
        self.engine.take_deltas();
    }

    fn on_authority_msg(&mut self, _ctx: &mut Ctx, msg: &[u8]) {
        let Ok(ops) = serde_json::from_slice::<Vec<Op>>(msg) else {
            return;
        };
        for op in ops {
            match op {
                Op::Show { claim } => self.display.put(DisplayItem {
                    key: claim.key,
                    label: match claim.maturity {
                        Maturity::Provisional => Label::Provisional,
                        Maturity::Final => Label::Final,
                    },
                    shown_values: claim.values,
                    evidence: claim.evidence,
                }),
                Op::Hide { key } => {
                    self.display.remove(&key);
                }
            }
        }
    }

    fn display(&self) -> &DisplayState {
        &self.display
    }

    fn visible_data(&self, binding: usize) -> &[qp_schema::Bar] {
        self.engine.bars(binding)
    }

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "n",
        summary: "control baseline: show on arrival (not a candidate)",
        authority: |s: &Setup| Box::new(Authority { engine: Engine::new(s) }),
        replica: |s: &Setup| {
            Box::new(Replica {
                engine: Engine::new(s),
                display: DisplayState::default(),
            })
        },
    }
}
