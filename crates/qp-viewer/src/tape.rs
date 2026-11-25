//! (RP3/M15, §22) The authority ↔ viewer check (VERIFY 15, `qp replica-tape --viewer`):
//! the replica scenario of `qp_replica::tape` for a **viewer** (moved here with the viewer
//! in RF1 S3.3): the device subscribes to `viewer`, opens the capsule and gets only that
//! instrument's slice, its claims and its data. Tooling, not engine logic.

use std::path::Path;

use qp_authority::{Lists, Session, Subsystems};
use qp_replica::tape::{
    events, holds, init, memory_provider, provider_init, Violation, BARS, SEED, START, TICKS, VIEW,
};
use qp_replica::{view_digest, Shown};
use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::protocol::{ClaimRef, ClaimState, Event, Maturity, ServerMsg, SessionInit};
use qp_schema::Condition;
use serde::Serialize;
use serde_json::json;

use crate::OpenSide;

/// (RP3/M15, §22) The outcome of [`run_viewer`].
#[derive(Debug, Clone, Serialize)]
pub struct ViewerOutcome {
    /// Steps.
    pub steps: usize,
    /// Steps whose open replica's slice or view digest differs from the authority's.
    pub differing_steps: usize,
    /// Display-rule violations (wrong or early claims) on the open instrument.
    pub violations: Vec<Violation>,
    /// Claims the authority asserted as final at the end (about the open instrument).
    pub final_claims: usize,
    /// Of those, shown as final by the device at the end.
    pub shown_final: usize,
    /// Steps at which the device subscribed to the other instrument's data (must be 0).
    pub foreign_steps: usize,
    /// Display items over all steps by state (final, provisional, pending, disputed).
    pub shown_counts: (usize, usize, usize, usize),
}

type Said = std::collections::BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)>;

#[allow(clippy::expect_used)]
/// Delivers the authority's lines on the viewer's connection to the device (its open
/// side) and records the authority's word on every claim it sent.
fn viewer_deliver(
    lines: Vec<qp_schema::protocol::OutboundLine>,
    side: &mut Option<OpenSide>,
    said: &mut Said,
    subs: Subsystems,
) {
    for l in lines.into_iter().filter(|l| l.conn == "r") {
        match &l.msg {
            ServerMsg::Notify {
                event: Event::Viewer { viewer, seq },
                ..
            } => {
                if viewer.closed {
                    *side = None;
                }
                if let Some(o) = &viewer.open {
                    match side.as_mut() {
                        Some(x) => x.update(o.clone(), *seq),
                        None => {
                            // The open instrument's own source (THYAO), entitled.
                            let p = memory_provider(&provider_init()).expect("provider");
                            *side = Some(OpenSide::new("dev", o.clone(), *seq, p, None, subs));
                        }
                    }
                }
            }
            ServerMsg::Notify {
                event:
                    Event::Claim {
                        claim,
                        state,
                        evidence,
                        ..
                    },
                ..
            } => {
                said.insert((claim.clone(), evidence.t), (*state, evidence.maturity));
                if let Some(x) = side.as_mut() {
                    x.on_message(&l.msg);
                }
            }
            _ => {}
        }
    }
}

/// (RP3/M15, §22) The consistency scenario for a **viewer**: the authority also holds a
/// second instrument (AKBNK) with its own feed; the device subscribes to `viewer`, opens
/// the capsule (THYAO) and gets only that instrument's slice, its claims and its data
/// (`lag` events behind). The same independent oracle as [`run`] checks every claim the
/// device shows; with `lag = 0` the open replica's slice and view digests equal the
/// authority's after every step.
///
/// # Errors
/// The scenario cannot be set up.
#[allow(clippy::too_many_lines)]
pub fn run_viewer(lag: usize, subs: Lists) -> Result<ViewerOutcome, String> {
    let mut iv = init();
    iv["document"]["series"]
        .as_array_mut()
        .ok_or("series")?
        .push(
            json!({"id":"px2","instrument":{"symbol":"AKBNK"},"interval":"1d",
            "source":{"kind":"bind","symbol":"AKBNK","interval":"1d","window":{"last":START}}}),
        );
    iv["data"]["feeds"]
        .as_array_mut()
        .ok_or("feeds")?
        .push(json!({"feed":{"symbol":"AKBNK","interval":"1d"},
            "synthetic":{"n":START,"seed":SEED + 1}}));
    iv["data"]["entitlements"]["dev"] = json!(["THYAO", "AKBNK"]);
    let init: SessionInit = serde_json::from_value(iv).map_err(|e| e.to_string())?;
    let mut a = Session::new(init, Path::new("."), subs.0)?;
    let other = FeedKey {
        symbol: "AKBNK".to_owned(),
        interval: "1d".to_owned(),
        feed: None,
    };
    let mut cfg = qp_source::SyntheticEventsConfig::new(BARS, SEED + 1, TICKS);
    cfg.start = START;
    let others: Vec<DataEvent> = qp_source::SyntheticEvents::new(other.clone(), cfg).collect();
    let mine = events();
    let mut said = Said::new();
    let mut side: Option<OpenSide> = None;
    for m in [
        json!({"type":"hello","protocol":"qp/0","principal":"dev","surface":{"kind":"phone"}}),
        json!({"type":"subscribe","id":"v","topic":{"kind":"viewer"}}),
        json!({"type":"input","id":"o","gesture":{"kind":"open","capsule":"cap","to":"capsule",
            "width":390,"height":300}}),
    ] {
        let lines = a.handle_raw("r", &m.to_string());
        viewer_deliver(lines, &mut side, &mut said, subs.1);
    }
    if side.is_none() {
        return Err("the viewer did not get the open capsule's slice".to_owned());
    }
    let mut violations = Vec::new();
    let mut differing = 0;
    let mut counts = (0, 0, 0, 0);
    let mut foreign = 0;
    let total = mine.len() + lag;
    for i in 0..total {
        if let Some(ev) = mine.get(i) {
            let mut lines = a.data_event(ev).map_err(|e| e.to_string())?;
            if let Some(o) = others.get(i) {
                lines.extend(a.data_event(o).map_err(|e| e.to_string())?);
            }
            viewer_deliver(lines, &mut side, &mut said, subs.1);
        }
        // The host serves the device only the feeds it subscribes to (§22.5).
        let subs = a
            .viewer_data("r")
            .map(<[FeedKey]>::to_vec)
            .unwrap_or_default();
        if subs.contains(&other) {
            foreign += 1;
        }
        let Some(s) = side.as_mut() else {
            return Err("the open capsule closed".to_owned());
        };
        if i >= lag {
            let ev = &mine[i - lag];
            if subs.contains(&ev.feed) {
                s.data_event(ev);
            }
        }
        // Digests: the authority's slice and view against the open replica's.
        let auth_slice = s.open.digest.clone();
        let own_slice = s.replica().document_digest().unwrap_or_default();
        let auth_view = view_digest(&mut a, VIEW.0, VIEW.1, VIEW.2).map_err(|e| e.message)?;
        let own_view = s
            .replica_mut()
            .view_digest(VIEW.0, VIEW.1, VIEW.2)
            .unwrap_or_default();
        if auth_slice != own_slice || auth_view != own_view {
            differing += 1;
        }
        // The independent oracle of `run` over the device's own bars.
        let own = s
            .replica()
            .bars("px")
            .map(<[qp_schema::Bar]>::to_vec)
            .unwrap_or_default();
        let rsi = qp_compute::compute_bars(
            "rsi",
            &std::collections::BTreeMap::from([("period".to_owned(), 14.0)]),
            qp_compute::Field::Close,
            &own,
        )
        .map_err(|e| e.to_string())?;
        let col = rsi.outputs.get("rsi").cloned().unwrap_or_default();
        let doc = s.replica().document().cloned();
        for it in s.replica().display() {
            match (it.state, it.label) {
                (Shown::Shown, Some(Maturity::Final)) => counts.0 += 1,
                (Shown::Shown, _) => counts.1 += 1,
                (Shown::Pending, _) => counts.2 += 1,
                (Shown::Disputed, _) => counts.3 += 1,
            }
            if it.state != Shown::Shown {
                continue;
            }
            let bad = |w: String| Violation { step: i, what: w };
            let key = (it.claim.clone(), it.t);
            let sd = said.get(&key).copied();
            if !matches!(sd, Some((ClaimState::Asserted, _))) {
                violations.push(bad(format!("{key:?} shown but not asserted ({sd:?})")));
            }
            let cond = match &it.claim {
                ClaimRef::Condition { .. } => None,
                ClaimRef::Intent { .. } => doc
                    .as_ref()
                    .and_then(|x| x.intents.first().and_then(|i| i.trigger.clone())),
                ClaimRef::Capsule { .. } => doc
                    .as_ref()
                    .and_then(|x| x.findings.first().map(|f| f.invalidation.clone())),
            };
            let Some(Condition::Threshold { op, value, .. }) = cond else {
                violations.push(bad(format!("{key:?}: no condition")));
                continue;
            };
            let op = serde_json::to_value(op)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let Some(j) = own.iter().position(|b| b.t == it.t) else {
                violations.push(bad(format!("{key:?} shown, own data lacks t")));
                continue;
            };
            let (p, v) = (j.checked_sub(1).and_then(|k| col[k]), col[j]);
            if !holds(&op, p, v, value) {
                violations.push(bad(format!("{key:?} shown, predicate false on own data")));
            }
            match it.label {
                Some(Maturity::Final) => {
                    if j + 1 >= own.len() {
                        violations.push(bad(format!("{key:?} final before its own bar closed")));
                    }
                    if !matches!(sd, Some((_, Maturity::Final))) {
                        violations.push(bad(format!("{key:?} final before the authority's final")));
                    }
                }
                _ => violations.push(bad(format!("{key:?} provisional with live display off"))),
            }
        }
    }
    let final_keys: Vec<_> = said
        .iter()
        .filter(|(_, (st, m))| *st == ClaimState::Asserted && *m == Maturity::Final)
        .map(|(k, _)| k.clone())
        .collect();
    let shown_final = side.as_ref().map_or(0, |s| {
        final_keys
            .iter()
            .filter(|k| {
                s.replica().display().iter().any(|it| {
                    it.claim == k.0
                        && it.t == k.1
                        && it.state == Shown::Shown
                        && it.label == Some(Maturity::Final)
                })
            })
            .count()
    });
    Ok(ViewerOutcome {
        steps: total,
        differing_steps: differing,
        violations,
        final_claims: final_keys.len(),
        shown_final,
        foreign_steps: foreign,
        shown_counts: counts,
    })
}
