//! M10 — the opening transition, sentence → capsule → chart (SPEC §4).
//!
//! For `S8` states 1, 3, 6 and 12 as found in the demo world (the screenshots come from the
//! live demo; #1 and #12 are hourly-world states, so their demo substitutes `101`/`112`
//! are used: same kind and direction, open, yesterday — `s8.rs`), at the state's replay
//! moment, through the web shell's core with the transition design by name:
//!
//! - [`plan`]: open the capsule (the `open` gesture a face tap sends), then tap its scene
//!   (`tap_at`, the core's recognizer) to open the chart; the shared elements of that
//!   capsule → chart transition (the authority's `Opening.shared` by the design) give the
//!   computed measures (shared count, area share before/after, displacement, protocol
//!   messages from the feed to the chart); the chart's anchors give the four region labels
//!   A–D (one on the capsule's fact: its marked bar at the fact's level; three others on
//!   other bars of the price panel, seeded) and the key;
//! - the screenshots (`demo_shots.mjs`): the capsule screen, two in-between frames (the
//!   core's frame at `p = 1/3` and `2/3`, frozen), the chart screen with the labels as
//!   harness overlays.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use qp_schema::scene::DataSource;
use qp_schema::scene::{PixelRect, SceneDesc};
use qp_schema::Mark;

use crate::blind;
use crate::io;
use crate::prompts;
use crate::rng::Rng;
use crate::s8::S8;
use crate::shots;
use crate::world::{Core, World};

/// SPEC §4 states.
pub const STATES: [usize; 4] = [1, 3, 6, 12];
/// In-between frames shown (progress).
pub const FRAMES: [f64; 2] = [1.0 / 3.0, 2.0 / 3.0];
/// Where the harness taps the open capsule's scene (CSS px; capsule depth: anywhere opens
/// the chart).
pub const TAP: (f64, f64) = (195.0, 150.0);
/// Port of the demo the screenshots use.
pub const PORT: u16 = 8472;

/// One region label (scene CSS px, centre).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// Label.
    pub label: String,
    /// Centre x.
    pub x: f64,
    /// Centre y.
    pub y: f64,
    /// The capsule's own fact.
    pub correct: bool,
}

/// One state's plan and measures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatePlan {
    /// S8 state.
    pub state: usize,
    /// Replay step.
    pub step: u64,
    /// Capsule id.
    pub capsule: String,
    /// Region labels on the chart.
    pub regions: Vec<Region>,
    /// G1 key (the correct label).
    pub g1: String,
    /// G2 key.
    pub g2: String,
    /// G3 key (S3 option).
    pub g3: String,
    /// Shared elements of capsule → chart with both boxes.
    pub shared: usize,
    /// Their ids.
    pub shared_ids: Vec<String>,
    /// Their area / the capsule scene's area.
    pub area_share_before: f64,
    /// Their area / the chart scene's area.
    pub area_share_after: f64,
    /// Mean and max centre displacement (px).
    pub shift_mean: f64,
    /// Max displacement.
    pub shift_max: f64,
    /// Protocol `input` messages from the feed to the chart.
    pub taps: usize,
    /// Shared elements of sentence → capsule (information).
    pub shared_open: usize,
}

/// A design's plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Harness version.
    pub harness: String,
    /// Transition design.
    pub design: String,
    /// Code.
    pub code: String,
    /// States.
    pub states: Vec<StatePlan>,
    /// Means over the states.
    pub mean: BTreeMap<String, f64>,
}

fn area(r: &PixelRect) -> f64 {
    f64::from((r.right - r.left).max(0)) * f64::from((r.bottom - r.top).max(0))
}

fn centre(r: &PixelRect) -> (f64, f64) {
    (
        f64::from(r.left + r.right) / 2.0,
        f64::from(r.top + r.bottom) / 2.0,
    )
}

fn price_series(sc: &SceneDesc) -> Option<String> {
    sc.panels
        .iter()
        .flat_map(|p| &p.layers)
        .find_map(|l| match (&l.data, l.mark) {
            (DataSource::Series { series }, Mark::Candle | Mark::Line) => Some(series.clone()),
            _ => None,
        })
}

/// Region labels on `chart` for the fact of `finding` (module docs).
fn regions(
    w: &World,
    chart: &SceneDesc,
    finding: &str,
    seed: u64,
) -> Result<(Vec<Region>, String), String> {
    let doc = w.host.session().document();
    let fact = doc
        .findings
        .iter()
        .find(|f| f.id == finding)
        .and_then(|f| f.facts.first())
        .and_then(|id| doc.facts.iter().find(|x| &x.id == id))
        .ok_or(format!("no fact for {finding}"))?;
    let ps = price_series(chart);
    let anchors = qp_grammar::depth::transition_anchors(chart, ps.as_deref(), Some(fact.bar));
    let panel = anchors
        .get("price")
        .copied()
        .ok_or("chart has no price panel")?;
    let bar = anchors
        .get(&format!("bar:{}", fact.bar))
        .copied()
        .ok_or("the fact's bar is not visible on the chart")?;
    let (bx, _) = centre(&bar);
    let level = anchors
        .get(&format!("evidence:{}", fact.id))
        .or_else(|| anchors.get(&format!("fact:{}", fact.id)))
        .map(centre);
    let clampy = |y: f64| y.clamp(f64::from(panel.top) + 14.0, f64::from(panel.bottom) - 14.0);
    let cy = clampy(level.map_or(f64::from(panel.top + panel.bottom) / 2.0, |c| c.1));
    let mut rng = Rng::derive(seed, &format!("m10-regions|{finding}"));
    let (l, r) = (f64::from(panel.left) + 14.0, f64::from(panel.right) - 14.0);
    let min_gap = (r - l) * 0.18;
    let mut pts: Vec<(f64, f64, bool)> = vec![(bx.clamp(l, r), cy, true)];
    let mut tries = 0;
    while pts.len() < 4 && tries < 1000 {
        tries += 1;
        let x = rng.range_f64(l, r);
        if pts.iter().all(|p| (p.0 - x).abs() >= min_gap) {
            let y = clampy(rng.range_f64(f64::from(panel.top), f64::from(panel.bottom)));
            pts.push((x, y, false));
        }
    }
    if pts.len() < 4 {
        return Err("cannot place four separated regions".into());
    }
    rng.shuffle(&mut pts);
    let mut out = Vec::new();
    let mut key = String::new();
    for (i, (x, y, c)) in pts.into_iter().enumerate() {
        let label = prompts::M10_REGIONS[i].to_owned();
        if c {
            key.clone_from(&label);
        }
        out.push(Region {
            label,
            x: io::round(x, 1),
            y: io::round(y, 1),
            correct: c,
        });
    }
    Ok((out, key))
}

/// Root of M10's records.
#[must_use]
pub fn root(exp: &Path) -> PathBuf {
    exp.join("M10")
}

/// The code of design `transition`.
#[must_use]
pub fn code(transition: &str, salt: u64) -> String {
    blind::code(salt, &format!("m10|{transition}"))
}

/// The plan of design `transition` (module docs).
///
/// # Errors
/// Unknown design, a state that cannot be opened, a world that does not start.
#[allow(clippy::cast_precision_loss)]
pub fn plan(transition: &str, s8: &S8, salt: u64) -> Result<Plan, String> {
    if !qp_grammar::transitions::is_transition(transition) {
        return Err(format!(
            "unknown transition `{transition}` (known: {})",
            qp_grammar::transitions::names().join(", ")
        ));
    }
    let code = code(transition, salt);
    let mut sts: Vec<&crate::s8::State> = STATES.iter().filter_map(|n| s8.in_demo(*n)).collect();
    sts.sort_by_key(|s| s.step);
    let mut w = World::demo();
    let mut c = Core::connect(&mut w, "m10", crate::READER, "tr")?;
    if transition != qp_grammar::transitions::DEFAULT_TRANSITION {
        c.act(&mut w, |k| k.set_look(&json!({"transition": transition})));
    }
    let mut out = Vec::new();
    for st in sts {
        if w.step() < st.step {
            c.seek(&mut w, st.step);
        }
        let sent0 = c.core.sent().len();
        let cap = st.capsule.clone();
        c.act(&mut w, |k| k.open_capsule(&cap));
        let shared_open = c
            .core
            .shared()
            .iter()
            .filter(|s| s.before.is_some() && s.after.is_some())
            .count();
        let before = c
            .core
            .scene()
            .ok_or(format!("S8 #{}: no capsule scene", st.state))?;
        c.act(&mut w, |k| {
            k.pointer("scene", "down", 1, TAP.0, TAP.1);
            k.pointer("scene", "up", 1, TAP.0, TAP.1);
        });
        let depth = c.core.open_json()["depth"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        if depth != "chart" {
            return Err(format!(
                "S8 #{}: the tap opened `{depth}`, not the chart",
                st.state
            ));
        }
        let after = c
            .core
            .scene()
            .ok_or(format!("S8 #{}: no chart scene", st.state))?;
        let taps = c.core.sent()[sent0..]
            .iter()
            .filter(|m| m["type"] == "input")
            .count();
        let both: Vec<_> = c
            .core
            .shared()
            .iter()
            .filter_map(|s| Some((s.id.clone(), s.before?, s.after?)))
            .collect();
        let ab: f64 = both.iter().map(|x| area(&x.1)).sum();
        let aa: f64 = both.iter().map(|x| area(&x.2)).sum();
        let shifts: Vec<f64> = both
            .iter()
            .map(|(_, b, a)| {
                let (p, q) = (centre(b), centre(a));
                ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()
            })
            .collect();
        let (regs, g1) = regions(
            &w,
            &after,
            &st.finding,
            u64::from_str_radix(&code, 16).unwrap_or(0),
        )?;
        out.push(StatePlan {
            state: st.state,
            step: st.step,
            capsule: st.capsule.clone(),
            regions: regs,
            g1,
            g2: prompts::M10_G2[0].to_owned(),
            g3: st.key.as_ref().map(|k| k.s3.clone()).unwrap_or_default(),
            shared: both.len(),
            shared_ids: both.iter().map(|x| x.0.clone()).collect(),
            area_share_before: io::round(
                ab / (f64::from(before.width) * f64::from(before.height)),
                4,
            ),
            area_share_after: io::round(aa / (f64::from(after.width) * f64::from(after.height)), 4),
            shift_mean: io::round(
                if shifts.is_empty() {
                    0.0
                } else {
                    shifts.iter().sum::<f64>() / shifts.len() as f64
                },
                2,
            ),
            shift_max: io::round(shifts.iter().copied().fold(0.0, f64::max), 2),
            taps,
            shared_open,
        });
        // Back to the feed for the next state.
        c.act(&mut w, |k| k.back());
        c.act(&mut w, |k| k.back());
    }
    let n = out.len().max(1) as f64;
    let mut mean = BTreeMap::new();
    for (k, f) in [
        (
            "shared",
            (|s: &StatePlan| s.shared as f64) as fn(&StatePlan) -> f64,
        ),
        ("area_share_before", |s| s.area_share_before),
        ("area_share_after", |s| s.area_share_after),
        ("shift_mean", |s| s.shift_mean),
        ("taps", |s| s.taps as f64),
    ] {
        mean.insert(
            k.to_owned(),
            io::round(out.iter().map(f).sum::<f64>() / n, 4),
        );
    }
    Ok(Plan {
        harness: crate::HARNESS_VERSION.to_owned(),
        design: transition.to_owned(),
        code,
        states: out,
        mean,
    })
}

/// Hidden key of a design (per state index 1..: G1–G3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Experiment.
    pub experiment: String,
    /// Design.
    pub design: String,
    /// Sequence index (1-based, plan order) → (state, G1, G2, G3).
    pub seqs: BTreeMap<String, (usize, String, String, String)>,
}

/// Builds design `transition`'s plan, computed measures (`raw/<design>/computed.json`),
/// screens (`raw/screens/<code>-<i>-<j>.png`), key. `shoot: false` skips Chromium.
///
/// # Errors
/// As [`plan`], or Chromium.
pub fn screens(
    transition: &str,
    s8: &S8,
    exp: &Path,
    salt: u64,
    shoot: bool,
) -> Result<Plan, String> {
    io::check_name(transition)?;
    let p = plan(transition, s8, salt)?;
    let raw = root(exp).join("raw");
    io::write_json(&raw.join(transition).join("computed.json"), &p)?;
    let mut seqs = BTreeMap::new();
    for (i, s) in p.states.iter().enumerate() {
        seqs.insert(
            (i + 1).to_string(),
            (s.state, s.g1.clone(), s.g2.clone(), s.g3.clone()),
        );
    }
    io::write_json(
        &raw.join("keys").join(format!("{}.json", p.code)),
        &Key {
            experiment: "m10".into(),
            design: transition.to_owned(),
            seqs,
        },
    )?;
    let mut m: BTreeMap<String, Value> = if raw.join("key.json").exists() {
        serde_json::from_str(&io::read(&raw.join("key.json"))?).map_err(|e| e.to_string())?
    } else {
        BTreeMap::new()
    };
    m.insert(
        p.code.clone(),
        json!({"experiment":"m10","design":transition}),
    );
    io::write_json(&raw.join("key.json"), &m)?;
    if shoot {
        let plan = json!({
            "kind": "m10", "qp": shots::qp_bin().display().to_string(), "port": PORT,
            "settings": {"transition": transition, "lang": "tr", "theme": "light"},
            "code": p.code, "tap": [TAP.0, TAP.1], "frames": FRAMES,
            "states": p.states.iter().enumerate().map(|(i, s)| json!({
                "i": i + 1, "step": s.step, "capsule": s.capsule, "regions": s.regions})).collect::<Vec<_>>(),
            "out": raw.join("screens").display().to_string(),
        });
        let pf = io::scratch()
            .join("m10")
            .join(format!("{}.plan.json", p.code));
        io::write_json(&pf, &plan)?;
        shots::node("demo_shots.mjs", &[&pf.display().to_string()])?;
    }
    Ok(p)
}
