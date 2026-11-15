//! `FileRealizer`: a `Realizer` that returns pre-generated sentences from a JSON file, so
//! a language-model alternative (which may not run inside the deterministic core) is
//! measured through the same interface as every other realizer.
//!
//! The alternative reads the inputs from `harness m2 inputs --out <file>` (situation id +
//! the exact `SentenceInput` values, no truth), generates its sentences, and writes:
//!
//! ```json
//! {"sentences": {"s01": {"tr": "…", "en": "…"}, "…": {}, "k26": {"tr": "…", "en": "…"}}}
//! ```
//!
//! The realizer maps an input back to its situation id by a fingerprint of the input
//! ([`fingerprint`]; unique over the set, tested). A missing sentence is returned as the
//! empty string (the metrics count it as a failure). Its generation cost goes to
//! `rounds/R3/experiments/M2/<alt>/calls.json` (see `metrics`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use qp_capsule::{Lang, Realizer, SentenceInput};
use serde::Deserialize;

use super::situations::{all, stage_name};

/// Fingerprint of an input: ids, bars, stage, time kind, holds, current, redacted, ledger.
#[must_use]
pub fn fingerprint(inp: &SentenceInput<'_>) -> String {
    format!(
        "{}|{}|{}|{}|{:?}|{:?}|{:?}|{}|{}/{}",
        inp.lead.id,
        inp.latest.id,
        inp.now_bar,
        stage_name(inp.story.stage),
        inp.story.time,
        inp.still_holds,
        inp.current.map(f64::to_bits),
        inp.redacted,
        inp.story.ledger_at_birth.realized,
        inp.story.ledger_at_birth.n
    )
}

#[derive(Debug, Deserialize)]
struct File {
    sentences: BTreeMap<String, BTreeMap<String, String>>,
}

type Loaded = Result<(BTreeMap<String, String>, File), String>;

/// Reads sentences keyed by situation id and language from a JSON file (lazily, on first
/// use).
pub struct FileRealizer {
    name: &'static str,
    path: PathBuf,
    data: OnceLock<Loaded>,
}

impl FileRealizer {
    /// A file realizer named `name` reading `path`.
    #[must_use]
    pub fn new(name: &'static str, path: impl Into<PathBuf>) -> Self {
        Self {
            name,
            path: path.into(),
            data: OnceLock::new(),
        }
    }

    fn load(&self) -> &Loaded {
        self.data.get_or_init(|| {
            let text = std::fs::read_to_string(&self.path)
                .map_err(|e| format!("{}: {e}", self.path.display()))?;
            let file: File =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", self.path.display()))?;
            let index = all()
                .iter()
                .map(|s| (fingerprint(&s.input()), s.id.clone()))
                .collect();
            Ok((index, file))
        })
    }
}

impl Realizer for FileRealizer {
    fn name(&self) -> &'static str {
        self.name
    }

    /// # Panics
    /// If the sentence file cannot be read or parsed (a setup error, not a measurement).
    fn sentence(&self, inp: &SentenceInput<'_>, lang: Lang) -> String {
        let (index, file) = match self.load() {
            Ok(d) => d,
            Err(e) => panic!("FileRealizer `{}`: {e}", self.name),
        };
        index
            .get(&fingerprint(inp))
            .and_then(|id| file.sentences.get(id))
            .and_then(|m| m.get(super::lang_name(lang)))
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_unique() {
        let v = all();
        let mut f: Vec<String> = v.iter().map(|s| fingerprint(&s.input())).collect();
        f.sort();
        f.dedup();
        assert_eq!(f.len(), v.len());
    }

    #[test]
    fn reads_file() {
        let dir = std::env::temp_dir().join(format!("r3h-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("s.json");
        std::fs::write(
            &p,
            r#"{"sentences": {"k26": {"tr": "RSI üç bar önce yükseldi.", "en": "RSI rose."}}}"#,
        )
        .unwrap();
        let r = FileRealizer::new("file_test", &p);
        let k = super::super::situations::k26();
        assert_eq!(r.sentence(&k.input(), Lang::En), "RSI rose.");
        let s01 = &all()[0];
        assert_eq!(r.sentence(&s01.input(), Lang::Tr), "");
        std::fs::remove_dir_all(&dir).ok();
    }
}
