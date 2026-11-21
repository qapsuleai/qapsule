//! BIST equity price step (SPEC §1; `docs/matriks/MESSAGES.md` §(c)), and the check of
//! the table against the live `priceStep` field of local raw recordings.

use std::collections::BTreeMap;
use std::path::Path;

use prost::Message as _;
use qp_matriks::messages::SymbolMessage;
use qp_matriks::{RawReader, RecordKind};
use serde::{Deserialize, Serialize};

/// Price step of an equity at price `p`: ≤20: 0.01; ≤50: 0.02; ≤100: 0.05; ≤250: 0.10;
/// ≤500: 0.25; ≤1000: 0.50; ≤2500: 1.00; above: 2.50.
#[must_use]
pub fn tick(p: f64) -> f64 {
    const TABLE: [(f64, f64); 7] = [
        (20.0, 0.01),
        (50.0, 0.02),
        (100.0, 0.05),
        (250.0, 0.10),
        (500.0, 0.25),
        (1000.0, 0.50),
        (2500.0, 1.00),
    ];
    for (hi, step) in TABLE {
        if p <= hi {
            return step;
        }
    }
    2.50
}

/// Aggregate result of the table check (no symbol names, no prices).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TickCheck {
    /// Raw files read.
    pub files: u64,
    /// Price-card messages read.
    pub symbol_messages: u64,
    /// Of those, not decodable as `SymbolMessage`.
    pub decode_errors: u64,
    /// Of those, carrying `priceStep`.
    pub with_step: u64,
    /// Messages carrying both `priceStep` and a price (compared).
    pub messages: u64,
    /// Of those, table step == live step.
    pub agree: u64,
    /// Distinct symbols seen with a step.
    pub symbols: u64,
    /// Symbols whose every observation agreed.
    pub symbols_agree: u64,
    /// Disagreements by `(table step, live step)` (both rendered with 4 decimals).
    pub disagreements: BTreeMap<String, u64>,
    /// Disagreements where the price sits exactly on a band limit (20, 50, 100, …):
    /// the table's `≤` against an exchange using `<` at the limit.
    pub edge_disagreements: u64,
    /// Files that ended in a truncated / corrupt record (counted, not fatal).
    pub damaged_files: u64,
}

/// Check the table against every `mx/symbol/...` message of the `.qpraw` files in `dir`
/// (only symbols of `universe`, when given).
/// The reference price is `last`, else `dayClose`, of the same message or, when the
/// message has none, the symbol's latest earlier one (else its next one).
///
/// # Errors
/// Unreadable directory.
pub fn check_raw(dir: &Path, universe: Option<&[String]>) -> Result<TickCheck, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "qpraw"))
        .collect();
    files.sort();
    let mut out = TickCheck::default();
    let mut per_sym: BTreeMap<String, bool> = BTreeMap::new();
    // Per topic: last price seen, steps waiting for a price.
    let mut state: BTreeMap<String, (Option<f64>, Vec<f64>)> = BTreeMap::new();
    for p in files {
        let Ok(f) = std::fs::File::open(&p) else {
            continue;
        };
        let Ok(reader) = RawReader::new(std::io::BufReader::new(f)) else {
            out.damaged_files += 1;
            continue;
        };
        out.files += 1;
        let (recs, err) = reader.read_all();
        if err.is_some() {
            out.damaged_files += 1;
        }
        for r in recs {
            if r.kind != RecordKind::Message || !r.topic.starts_with("mx/symbol/") {
                continue;
            }
            out.symbol_messages += 1;
            let Ok(m) = SymbolMessage::decode(r.payload.as_slice()) else {
                out.decode_errors += 1;
                continue;
            };
            if m.price_step.is_some() {
                out.with_step += 1;
            }
            let sym = r
                .topic
                .trim_start_matches("mx/symbol/")
                .split('@')
                .next()
                .unwrap_or_default()
                .to_owned();
            if universe.is_some_and(|u| !u.contains(&sym)) {
                continue;
            }
            // The step arrives in the snapshot, often without a price: pair it with the
            // symbol's price as of that message (the last one seen, or the next one).
            let st = state.entry(sym).or_default();
            if let Some(p) = m.last.or(m.day_close).filter(|p| *p > 0.0) {
                st.0 = Some(p);
            }
            if let Some(step) = m.price_step.filter(|s| *s > 0.0) {
                st.1.push(step);
            }
            if let Some(price) = st.0 {
                for step in std::mem::take(&mut st.1) {
                    out.messages += 1;
                    let t = tick(price);
                    let ok = (t - step).abs() < 1e-9;
                    if ok {
                        out.agree += 1;
                    } else {
                        if [20.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0].contains(&price) {
                            out.edge_disagreements += 1;
                        }
                        *out.disagreements
                            .entry(format!("{t:.4} vs {step:.4}"))
                            .or_insert(0) += 1;
                    }
                    *per_sym.entry(r.topic.clone()).or_insert(true) &= ok;
                }
            }
        }
    }
    out.symbols = per_sym.len() as u64;
    out.symbols_agree = per_sym.values().filter(|x| **x).count() as u64;
    Ok(out)
}
