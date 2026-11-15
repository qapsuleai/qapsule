//! R3/S-A cross-check: `qp-capsule`'s productized strategies (`theme::KindDir`,
//! `outcome::Barrier`, `rank::Confirm`, `realize::Template`, `EngineConfig::r3`) against
//! the measured experiment crates (`M4/kinddir`, `M5/barrier`, `M3/confirm`,
//! `M2/template`).
//!
//! - Sentences: both realizers over the M2 harness situations
//!   (`crates/qp-capsule/tests/fixtures/m2_inputs.json`, exported with
//!   `harness m2 inputs`; synthetic), byte by byte. The only allowed differences are the
//!   two documented sentence fixes ([`CHANGED`]).
//! - Engine: `simulate` over the regime synthetic market with synthetic flow, product
//!   config vs experiment config; story events and ledger records as JSON, byte by byte.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_capsule::engine::EngineConfig;
use qp_capsule::{simulate, InstrumentInput, Lang, Realizer, SentenceInput};
use qp_schema::capsule::Story;
use qp_schema::Fact;
use serde::{Deserialize, Serialize};

#[path = "../../../../../../crates/qp-capsule/examples/support/mod.rs"]
pub mod support;

/// Situations whose sentences the product deliberately changed (S-A.md): flow
/// state/expectation on net institutional flow (M2b finding) and extreme stories closed
/// as realized / invalidated (M5 RESULTS). Every other situation must be byte-identical.
// S-A fixes (flow wording, extreme under the barrier outcome) + S-C fix (TR vowel harmony,
// "Sinyalın" → "Sinyalin": s07 s09 s14 s23 s29 s35 s46).
pub const CHANGED: &[&str] = &[
    "s07", "s09", "s14", "s19", "s23", "s29", "s35", "s42", "s44", "s45", "s46", "s47", "s48",
    "s65", "s66", "s67", "s69", "s70", "s71", "s72",
];

/// One M2 harness situation as exported by `harness m2 inputs`.
#[derive(Debug, Clone, Deserialize)]
pub struct Situation {
    /// Situation id.
    pub id: String,
    /// Story.
    pub story: Story,
    /// Lead fact.
    pub lead: Fact,
    /// Latest fact.
    pub latest: Fact,
    /// Now bar.
    pub now_bar: u64,
    /// Now time.
    pub now_t: i64,
    /// Still holds.
    pub still_holds: Option<bool>,
    /// Current value.
    pub current: Option<f64>,
    /// Redacted.
    pub redacted: bool,
}

impl Situation {
    /// As a realizer input.
    #[must_use]
    pub fn input(&self) -> SentenceInput<'_> {
        SentenceInput {
            story: &self.story,
            lead: &self.lead,
            latest: &self.latest,
            now_bar: self.now_bar,
            now_t: self.now_t,
            still_holds: self.still_holds,
            current: self.current,
            redacted: self.redacted,
        }
    }
}

/// Repository root (five levels up from this crate).
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..")
}

/// `crates/qp-capsule/tests/fixtures/`.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    repo_root().join("crates/qp-capsule/tests/fixtures")
}

/// The exported situations.
///
/// # Panics
/// Missing or malformed fixture.
#[must_use]
pub fn situations() -> Vec<Situation> {
    let p = fixtures_dir().join("m2_inputs.json");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// TR and EN sentence of one situation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
}

/// Sentences of every situation by id.
#[must_use]
pub fn sentences(r: &dyn Realizer, sits: &[Situation]) -> BTreeMap<String, Pair> {
    sits.iter()
        .map(|s| {
            let i = s.input();
            (
                s.id.clone(),
                Pair {
                    tr: r.sentence(&i, Lang::Tr),
                    en: r.sentence(&i, Lang::En),
                },
            )
        })
        .collect()
}

/// The sentence fixture kept in `qp-capsule` (`m2_sentences.json`): the measured
/// realizer's sentences, and the product's sentences where they differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentenceFixture {
    /// Provenance note.
    pub note: String,
    /// `m2_template` (measured, commit `85b5b5d`) sentences, every situation.
    pub measured: BTreeMap<String, Pair>,
    /// `qp_capsule::realize::Template` sentences where they differ from `measured`.
    pub product_changes: BTreeMap<String, Pair>,
}

/// Builds the fixture from the two realizers.
#[must_use]
pub fn sentence_fixture() -> SentenceFixture {
    let sits = situations();
    let measured = sentences(&m2_template::Realizer, &sits);
    let product = sentences(&qp_capsule::realize::Template, &sits);
    let product_changes = product
        .into_iter()
        .filter(|(id, p)| measured.get(id) != Some(p))
        .collect();
    SentenceFixture {
        note: "R3/S-A: `measured` = rounds/R3/experiments/M2/template (85b5b5d) on \
               tests/fixtures/m2_inputs.json (harness m2 inputs, synthetic); \
               `product_changes` = qp_capsule::realize::Template where it differs \
               (rounds/R3/experiments/S/S-A.md). Regenerate: cargo run --release \
               --manifest-path rounds/R3/experiments/S/crosscheck/Cargo.toml -- fixture"
            .to_owned(),
        measured,
        product_changes,
    }
}

/// Pretty JSON + newline.
///
/// # Panics
/// Never for these types.
#[must_use]
pub fn pretty<T: Serialize>(v: &T) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("serializable");
    s.push('\n');
    s
}

/// Compact JSON.
///
/// # Panics
/// Never for these types.
#[must_use]
pub fn compact<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("serializable")
}

/// The measured experiment config (M4 kinddir, M5 barrier, M3 confirm).
#[must_use]
pub fn experiment_config() -> EngineConfig {
    EngineConfig {
        theme: m4_kinddir::theme(),
        outcome: m5_barrier::outcome(),
        ranker: m3_confirm::ranker(),
    }
}

/// Result of one engine comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimCheck {
    /// Dataset.
    pub dataset: String,
    /// Story events.
    pub events: usize,
    /// Ledger rows.
    pub ledger: usize,
    /// Facts.
    pub facts: usize,
    /// Events JSON byte-identical.
    pub events_equal: bool,
    /// Ledger records JSON byte-identical.
    pub ledger_equal: bool,
    /// Facts JSON byte-identical.
    pub facts_equal: bool,
    /// Byte length of the events JSON.
    pub events_bytes: usize,
}

/// Runs both configs over `inputs` and compares.
#[must_use]
pub fn sim_check(dataset: &str, inputs: &[InstrumentInput]) -> SimCheck {
    let a = simulate(inputs, &qp_facts::default_detectors, EngineConfig::r3());
    let b = simulate(inputs, &qp_facts::default_detectors, experiment_config());
    let (ea, eb) = (compact(&a.events), compact(&b.events));
    SimCheck {
        dataset: dataset.to_owned(),
        events: a.events.len(),
        ledger: a.records.len(),
        facts: a.facts.len(),
        events_equal: ea == eb,
        ledger_equal: compact(&a.records) == compact(&b.records),
        facts_equal: compact(&a.facts) == compact(&b.facts),
        events_bytes: ea.len(),
    }
}
