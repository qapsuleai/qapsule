//! M2 comprehension test material (`harness m2 sheets --alt <name> --lang tr|en --seed s`).
//!
//! 24 sentences: the K26 situation plus 23 seeded from the 72 (the selection depends on the
//! seed only, not on the language, so TR and EN with the same seed ask about the same
//! situations), in seeded order, numbered 1…24. The prompt carries the sentences; the
//! hidden key (`keys/<lang>_s<seed>.json`) never goes to the reader.

use std::path::{Path, PathBuf};

use qp_capsule::{Lang, Realizer};
use serde::{Deserialize, Serialize};

use super::lang_name;
use super::situations::{all, Situation, Truth};
use crate::io::{write, write_json};
use crate::rng::Rng;

/// Sentences per sheet.
pub const SHEET_SIZE: usize = 24;

/// One item of the key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Number shown to the reader (`"1"` … `"24"`).
    pub label: String,
    /// Situation id.
    pub situation: String,
    /// The sentence shown.
    pub sentence: String,
    /// Truth.
    pub truth: Truth,
}

/// Hidden key of one sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    /// Harness version.
    pub harness: String,
    /// Alternative.
    pub alt: String,
    /// `tr` / `en`.
    pub lang: String,
    /// Seed.
    pub seed: u64,
    /// Items.
    pub items: Vec<Item>,
}

/// The 24 situations of a sheet, in reader order.
#[must_use]
pub fn select(seed: u64) -> Vec<Situation> {
    let mut sits = all();
    let k = sits.pop().expect("k26 is last");
    let mut r = Rng::derive(seed, "m2.sheet");
    r.shuffle(&mut sits);
    sits.truncate(SHEET_SIZE - 1);
    sits.push(k);
    r.shuffle(&mut sits);
    sits
}

/// The reader prompt for a list of sentences.
#[must_use]
pub fn prompt(lang: Lang, sentences: &[(String, String)]) -> String {
    let lang_word = match lang {
        Lang::Tr => "Turkish",
        Lang::En => "English",
    };
    let mut p = format!(
        "You are taking a reading test. Below are {} numbered sentences in {lang_word}. Each \
         describes one observation about a stock's chart. A \"bar\" is one period of the chart \
         (for example one trading day). For every sentence answer three questions:\n\n\
         1. What happened? `kind` (one of the six kinds below) and `direction` (up, down or \
         neutral).\n\
         2. When? `when` = \"past\" (it happened some bars ago; then give `bars_ago` as an \
         integer, 0 = this bar / today, or null if the sentence does not say), \"now\" (a \
         state that holds now) or \"conditional\" (an expectation that depends on a \
         condition); `bars_ago` is null unless `when` is \"past\".\n\
         3. Is it still valid now? `still_valid` = \"yes\", \"no\" or \"unknown\".\n\n\
         The six kinds:\n\
         - continuation: the move goes on\n\
         - reversal: the move may be turning\n\
         - breakout: a boundary was crossed\n\
         - squeeze: the move is narrowing; direction unknown\n\
         - extreme: an unusual stretch (overbought / oversold)\n\
         - flow: money flowing in or out, independent of price\n\n\
         Sentences:\n",
        sentences.len()
    );
    for (l, s) in sentences {
        p.push_str(&format!("{l}. {s}\n"));
    }
    p.push_str(
        "\nGive your best guess for every sentence. Reply with a single JSON object and nothing \
         else, keyed by sentence number, for example:\n\
         {\"1\": {\"kind\": \"breakout\", \"direction\": \"up\", \"when\": \"past\", \"bars_ago\": 3, \"still_valid\": \"no\"}, \"2\": {...}, ...}\n",
    );
    p
}

/// Writes the prompt and the hidden key. Returns the prompt path.
///
/// # Errors
/// I/O failure.
pub fn write_sheet(
    r: &dyn Realizer,
    root: &Path,
    lang: Lang,
    seed: u64,
) -> Result<PathBuf, String> {
    crate::io::check_name(r.name())?;
    let sel = select(seed);
    let items: Vec<Item> = sel
        .iter()
        .enumerate()
        .map(|(i, s)| Item {
            label: (i + 1).to_string(),
            situation: s.id.clone(),
            sentence: r.sentence(&s.input(), lang),
            truth: s.truth.clone(),
        })
        .collect();
    let pairs: Vec<(String, String)> = items
        .iter()
        .map(|i| (i.label.clone(), i.sentence.clone()))
        .collect();
    let raw = super::raw_dir(root, r.name());
    let tag = format!("{}_s{seed}", lang_name(lang));
    let pp = raw.join("prompts").join(format!("{tag}.txt"));
    write(&pp, prompt(lang, &pairs).as_bytes())?;
    write_json(
        &raw.join("keys").join(format!("{tag}.json")),
        &Key {
            harness: crate::HARNESS_VERSION.to_owned(),
            alt: r.name().to_owned(),
            lang: lang_name(lang).to_owned(),
            seed,
            items,
        },
    )?;
    Ok(pp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection() {
        let a = select(1);
        assert_eq!(a.len(), 24);
        assert!(a.iter().any(|s| s.id == "k26"));
        let ids = |v: &[Situation]| v.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&a), ids(&select(1)));
        assert_ne!(ids(&a), ids(&select(2)));
        let mut u = ids(&a);
        u.sort();
        u.dedup();
        assert_eq!(u.len(), 24);
    }
}
