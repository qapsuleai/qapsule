//! Trust path **mühür** (R3/M6 choice; productized in R3/S-C) ("seal"): a licensed server fact reaches an unentitled
//! user as a digit-free comment whose leakage is bounded **by construction**, carrying a
//! receipt that binds the statement to the server's key and a replayable audit log.
//!
//! Three layers (details and proof sketch in `NOTES.md`):
//!
//! 1. **Binary lattice at ingestion.** The only raw value read is `net`. On arrival it is
//!    replaced by its [`Cell`]: sign and binary order of magnitude
//!    (`level = floor(log2 |net|)`, read exactly from the IEEE exponent bits). Everything
//!    that can reach a message ([`speak`], the reference, the stored window, the
//!    receipt) is a function of cells only; the raw value goes nowhere else except the
//!    audit commitment, which is never sent to the user. Hence: two values in the same
//!    cell give byte-identical transcripts.
//! 2. **Clipped reference on the same lattice.** The reference level `R` is the lower
//!    median of the stored levels of the last [`Params::window`] observations, frozen per
//!    epoch of [`Params::epoch`] bars. A level is stored clipped to `[R − low, R + loud]`,
//!    so everything below `2^(R−low)` (both signs) is one cell and everything at or above
//!    `2^(R+loud)` (per sign) is one cell. A message is emitted iff
//!    `level ≥ R + loud` and says only the direction: "well above this stock's usual
//!    recent level".
//! 3. **Receipt.** Each message carries `receipt = HMAC(server key, statement)` in
//!    letters only. It is a function of the statement (already in the transcript), so it
//!    adds no leakage; whoever holds the verification key can prove the statement was
//!    issued by the server unaltered. Omissions and the rule itself are checked by an
//!    entitled auditor who re-derives every statement from the licensed data
//!    ([`replay`]) and checks the audit log's commitments to the raw values
//!    ([`Muhur::audit_log`]). Receipts are deliberately not chained: a chain adds no
//!    information for a user who sees the whole history, but ties every receipt to the
//!    entire past (NOTES.md §4).
//!
//! Deterministic: no clock, no randomness, integer comparisons of exponent bits.
//!
//! Provenance: moved from the measured experiment crate `rounds/R3/experiments/M6/muhur/`
//! (`m6_muhur`, branch `r3/m6b-novel`; choice in `rounds/R3/experiments/M6/RESULTS.md`)
//! with identical behaviour; the only changes are the paths (`crate::` instead of
//! `qp_capsule::`) and the type name `MuhurPath` → [`Muhur`]. `tests/muhur_crosscheck.rs`
//! compares messages and audit logs with the experiment crate byte for byte.

use std::collections::{BTreeMap, VecDeque};

use crate::{FlowObservation, QualMessage, TrustPath};
use qp_schema::capsule::Trust;
use qp_schema::Fact;

pub mod seal;

/// Name used in reports.
pub const NAME: &str = "m6.muhur";

/// The raw key read (the only one).
pub const KEY_NET: &str = "net";

/// Demo stand-in for the server's receipt key. In production a secret held by the
/// authority (HSM); here a constant so runs are reproducible. The leakage bound does
/// **not** depend on its secrecy (receipts are a function of the statements).
/// Public demo key (not a secret): tests and the synthetic test host only.
pub const DEMO_RECEIPT_KEY: &[u8] = b"qapsule/m6/muhur/demo-receipt-key";
/// Demo stand-in for the audit-commitment key (separate from the receipt key).
pub const DEMO_AUDIT_KEY: &[u8] = b"qapsule/m6/muhur/demo-audit-key";

/// Level given to `net == 0` (below every real level).
pub const ZERO_LEVEL: i32 = -2000;

/// Parameters (public: Kerckhoffs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    /// Observations in the reference window.
    pub window: usize,
    /// Observations needed before anything is said.
    pub min_obs: usize,
    /// Levels below `R − low` are merged (stored as `R − low`).
    pub low: i32,
    /// Speak iff `level ≥ R + loud`; levels above are merged (stored as `R + loud`).
    pub loud: i32,
    /// Reference is frozen for epochs of this many bars (by bar index, public).
    pub epoch: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            window: 5,
            min_obs: 5,
            low: 2,
            loud: 2,
            epoch: 1,
        }
    }
}

/// What the path keeps of one raw value: sign and binary order of magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// `floor(log2 |net|)`; [`ZERO_LEVEL`] for zero.
    pub level: i32,
    /// `net > 0`.
    pub up: bool,
}

impl Cell {
    /// The cell of a finite value (exact: from the exponent bits).
    #[must_use]
    pub fn of(x: f64) -> Self {
        let bits = x.abs().to_bits();
        let exp = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
        let level = if x == 0.0 {
            ZERO_LEVEL
        } else if exp == 0 {
            // Subnormal: below every normal level; one cell.
            -1075
        } else {
            exp - 1023
        };
        Self { level, up: x > 0.0 }
    }
}

/// A statement: codes and the two sentences. A pure function of a cell and the
/// reference level — it cannot see raw values (the signature enforces it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Machine codes.
    pub codes: Vec<String>,
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
}

/// The statement for `cell` under reference `r` (`None` → silence).
#[must_use]
pub fn speak(cell: Cell, r: Option<i32>, p: &Params) -> Option<Statement> {
    let r = r?;
    if cell.level < r + p.loud {
        return None;
    }
    let (code, tr, en) = if cell.up {
        (
            "net_buy",
            "Sunucu doğruladı: bu hissede kurumsal net alış, hissenin yakın dönemdeki olağan \
             düzeyinin belirgin biçimde üzerinde. Ham akış rakamları akış veri yetkisi gerektirir.",
            "Server-verified: institutional net buying in this stock is well above its usual \
             recent level. Raw flow figures require the flow data entitlement.",
        )
    } else {
        (
            "net_sell",
            "Sunucu doğruladı: bu hissede kurumsal net satış, hissenin yakın dönemdeki olağan \
             düzeyinin belirgin biçimde üzerinde. Ham akış rakamları akış veri yetkisi gerektirir.",
            "Server-verified: institutional net selling in this stock is well above its usual \
             recent level. Raw flow figures require the flow data entitlement.",
        )
    };
    Some(Statement {
        codes: vec![
            "flow_institutional".to_owned(),
            code.to_owned(),
            "well_above_usual".to_owned(),
        ],
        tr: tr.to_owned(),
        en: en.to_owned(),
    })
}

/// One audit-log entry (kept by the authority; delivered only to entitled auditors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// Bar index.
    pub bar: u64,
    /// The receipt the user got (empty for a silent bar).
    pub receipt: String,
    /// Binding commitment to the raw `net` of the bar (HMAC with the audit key). Never
    /// part of a user message.
    pub evidence_commitment: String,
}

/// The trust path.
#[derive(Debug, Clone)]
pub struct Muhur {
    p: Params,
    receipt_key: Vec<u8>,
    audit_key: Vec<u8>,
    /// Stored (clipped) levels of the last `window` observations, oldest first.
    stored: VecDeque<i32>,
    reference: Option<i32>,
    ref_epoch: Option<u64>,
    audit: Vec<AuditEntry>,
}

impl Muhur {
    /// A fresh path with `p` and the **demo keys** — for tests, the measurement harness and
    /// the synthetic test-host flow only. A real host supplies its own secret keys
    /// ([`Muhur::with_keys`]; in the authority `Session::set_trust_keys`), never these.
    #[must_use]
    pub fn demo(p: Params) -> Self {
        Self::with_keys(p, DEMO_RECEIPT_KEY, DEMO_AUDIT_KEY)
    }

    /// A fresh path with explicit keys.
    #[must_use]
    pub fn with_keys(p: Params, receipt_key: &[u8], audit_key: &[u8]) -> Self {
        Self {
            p,
            receipt_key: receipt_key.to_vec(),
            audit_key: audit_key.to_vec(),
            stored: VecDeque::with_capacity(p.window + 1),
            reference: None,
            ref_epoch: None,
            audit: Vec::new(),
        }
    }

    /// Current reference level (tests, audit).
    #[must_use]
    pub fn reference(&self) -> Option<i32> {
        self.reference
    }

    /// The audit log so far (entitled auditors only).
    #[must_use]
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit
    }

    fn median(&self) -> i32 {
        let mut v: Vec<i32> = self.stored.iter().copied().collect();
        v.sort_unstable();
        v[(v.len() - 1) / 2]
    }

    /// One step on a cell (no raw value past this point).
    fn step_cell(&mut self, bar: u64, cell: Cell) -> Option<Statement> {
        let epoch = bar / self.p.epoch.max(1);
        if self.stored.len() >= self.p.min_obs.max(1) && self.ref_epoch != Some(epoch) {
            self.reference = Some(self.median());
            self.ref_epoch = Some(epoch);
        }
        let st = speak(cell, self.reference, &self.p);
        let keep = match self.reference {
            Some(r) => cell.level.clamp(r - self.p.low, r + self.p.loud),
            None => cell.level,
        };
        self.stored.push_back(keep);
        while self.stored.len() > self.p.window.max(1) {
            self.stored.pop_front();
        }
        st
    }
}

/// Canonical bytes of a statement for the receipt (public fields only).
#[must_use]
pub fn statement_bytes(instrument: &str, bar: u64, t: i64, codes: &[String]) -> Vec<u8> {
    format!("{NAME}|{instrument}|{bar}|{t}|{}", codes.join(",")).into_bytes()
}

/// Receipt of a statement.
#[must_use]
pub fn receipt(key: &[u8], statement: &[u8]) -> [u8; 32] {
    seal::hmac_sha256(key, statement)
}

/// Letters-only rendering of a receipt.
#[must_use]
pub fn receipt_text(r: &[u8; 32]) -> String {
    seal::letters(r, 5)
}

impl TrustPath for Muhur {
    fn name(&self) -> &'static str {
        NAME
    }

    fn observe(&mut self, obs: &FlowObservation, _facts: &[Fact]) -> Vec<QualMessage> {
        // Detector facts are deliberately not consulted: their thresholds are not on the
        // lattice and would re-open the fine-grained channel (NOTES.md §2).
        let Some(net) = obs.values.get(KEY_NET).copied().filter(|v| v.is_finite()) else {
            return Vec::new();
        };
        // Audit commitment: the only use of the raw value besides its cell.
        let commit = seal::hmac_sha256(
            &self.audit_key,
            format!(
                "{NAME}|{}|{}|{}|{:016x}",
                obs.instrument,
                obs.bar,
                obs.t,
                net.to_bits()
            )
            .as_bytes(),
        );
        let cell = Cell::of(net);
        let Some(st) = self.step_cell(obs.bar, cell) else {
            self.audit.push(AuditEntry {
                bar: obs.bar,
                receipt: String::new(),
                evidence_commitment: seal::letters(&commit, 8),
            });
            return Vec::new();
        };
        let r = receipt(
            &self.receipt_key,
            &statement_bytes(&obs.instrument, obs.bar, obs.t, &st.codes),
        );
        let receipt = receipt_text(&r);
        self.audit.push(AuditEntry {
            bar: obs.bar,
            receipt: receipt.clone(),
            evidence_commitment: seal::letters(&commit, 8),
        });
        let attachments = BTreeMap::from([
            ("entitlement".to_owned(), "flow/*".to_owned()),
            ("evidence".to_owned(), "flow.institutional/net".to_owned()),
            ("rule".to_owned(), "muhur".to_owned()),
            ("rule_version".to_owned(), "one".to_owned()),
            ("receipt".to_owned(), receipt),
            (
                "check".to_owned(),
                "receipt by server key; replay by entitled auditor".to_owned(),
            ),
        ]);
        vec![QualMessage {
            t: obs.t,
            bar: obs.bar,
            instrument: obs.instrument.clone(),
            codes: st.codes,
            tr: st.tr,
            en: st.en,
            trust: Trust::Server,
            shown_bar: obs.bar,
            attachments,
        }]
    }
}

/// The M6 entry point: a fresh path with the default parameters.
#[must_use]
pub fn trust_path() -> Box<dyn TrustPath> {
    Box::new(Muhur::demo(Params::default()))
}

/// A fresh path with explicit parameters (studies).
#[must_use]
pub fn trust_path_with(p: Params) -> Box<dyn TrustPath> {
    Box::new(Muhur::demo(p))
}

/// Verification of a served message sequence (one instrument, in order) against the
/// receipt key: every receipt must be the HMAC of its statement and every text the
/// rule's text for its codes. Returns the index of the first bad message, or `None`
/// when all verify.
#[must_use]
pub fn verify_receipts(messages: &[QualMessage], key: &[u8]) -> Option<usize> {
    for (i, m) in messages.iter().enumerate() {
        let r = receipt(key, &statement_bytes(&m.instrument, m.bar, m.t, &m.codes));
        if m.attachments.get("receipt") != Some(&receipt_text(&r)) || m.trust != Trust::Server {
            return Some(i);
        }
        // Text must be the rule's text for its codes (nothing else can be said).
        let up = m.codes.iter().any(|c| c == "net_buy");
        let canonical = speak(
            Cell { level: 0, up },
            Some(-Params::default().loud),
            &Params::default(),
        );
        match canonical {
            Some(s) if s.codes == m.codes && s.tr == m.tr && s.en == m.en => {}
            _ => return Some(i),
        }
    }
    None
}

/// Entitled replay: run the public rule on the licensed observations from scratch.
/// An entitled auditor compares the result with what was served (byte equality) and
/// with the audit log's evidence commitments.
#[must_use]
pub fn replay(observations: &[FlowObservation], p: Params) -> (Vec<QualMessage>, Vec<AuditEntry>) {
    let mut path = Muhur::demo(p);
    let mut out = Vec::new();
    for o in observations {
        out.extend(path.observe(o, &[]));
    }
    (out, path.audit)
}
