//! (R5B/V) The agent comment validator (`rounds/R5B/PLAN.md` §2.1, GOAL principle 2).
//!
//! Pure and deterministic: no clock, network, randomness or language model. The same
//! draft, evidence and strictness always give the same result (and the same reasons in
//! the same order).
//!
//! Checks:
//!
//! 1. **Citations** (every strictness): `cites` is not empty; every id is known to the
//!    reader (`license`/`not_visible` when it exists but the reader may not see it,
//!    `cites`/`unknown_id` otherwise); every `[n]` marker points into `cites`; every
//!    external quote names its source.
//! 2. **Numbers** ([`Strictness::CitesNumbers`], [`Strictness::PerSentence`]): every
//!    number in the text is one of the evidence values or an external quote value, by the
//!    rounding rule of [`number_matches`]; every external value appears in its quote.
//! 3. **Advice** (every strictness): the R5 advice list (`ADVICE_TR`, `ADVICE_EN`; R5
//!    `lang/SPEC.md` §2.2) with the Turkish suffix rule.
//! 4. **Licence** (every strictness): a number that matches a withheld value and no
//!    visible value is `licensed_value`, even when an external quote carries it; an
//!    external value that matches a withheld value is `licensed_external`.
//! 5. **Per sentence** ([`Strictness::PerSentence`]): a sentence with a number carries a
//!    valid `[n]` marker.
//! 6. **Direction** ([`Strictness::CitesNumbers`], [`Strictness::PerSentence`]): when
//!    every cited directional item ([`Evidence::directions`]) points the same way and the
//!    text uses only words of the opposite direction (`DIR_*`), `wrong_direction`. Both
//!    directions or none pass (false rejections kept low).
//!
//! The advice scan always uses both lists (TR and EN); `lang` decides only how numbers,
//! dates and direction words are read.
//!
//! What is not a number: instrument symbols ([`Evidence::symbols`], removed first),
//! citation markers `[1]`, clock times (`14:30`), dates (`12 Kas`, `12 Nov`, `Nov 12`,
//! `Kasım 2026`, `2026-11-21`, `21.11.2026`) and a year written as a year (`2026'da`,
//! `2026 yılında`, `in 2026`). A bare `2026` is a number.
//!
//! Reading a number: `1.234,5` / `1,234.5` (both separators: the last one is the
//! decimal mark); one separator repeated (`1.234.567`) is a thousands separator; a
//! single separator is the decimal mark of the language (TR `,`, EN `.`), and the other
//! one is a thousands separator only when exactly three digits follow and at most three
//! (not starting with `0`) precede it: TR `1.234` = 1234, EN `1.234` = 1.234, TR `12.5`
//! = 12.5, EN `1,234` = 1234, EN `12,5` = 12.5.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

pub use qp_schema::comment::{Check, CommentDraft, ExternalQuote, Reason, Strictness};
use qp_schema::protocol::Language;

use crate::Lang;

mod check;
mod lexicon;
mod number;

pub use check::validate;
pub use lexicon::{
    advice_hits, direction_words, ADVICE_EN, ADVICE_TR, DIR_DOWN_EN, DIR_DOWN_TR, DIR_UP_EN,
    DIR_UP_TR,
};
use number::re;
pub use number::{
    markers, number_matches, number_word_run, numbers, raw_numbers, spelled_decimal, strip_symbols,
    unmasked_numbers, withheld_matches, Num, NumKind,
};

/// The evidence the authority gathered for one draft, projected to the reader.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// Ids that exist and the reader may see.
    pub known_ids: BTreeSet<String>,
    /// Ids that exist but the reader may not see (licence).
    pub hidden_ids: BTreeSet<String>,
    /// Numbers of the cited items, visible to the reader.
    pub values: Vec<CitedValue>,
    /// Numbers in the cited items' or the instrument's scope the reader may not see.
    pub withheld: Vec<f64>,
    /// Instrument symbols (removed before the number scan, e.g. `SYN03`).
    pub symbols: Vec<String>,
    /// Directions of the cited items (facts / capsules); neutral ones are ignored.
    #[serde(default)]
    pub directions: Vec<CitedDirection>,
}

/// A direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Up.
    Up,
    /// Down.
    Down,
    /// No direction.
    Neutral,
}

/// The direction of one cited item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitedDirection {
    /// The cited item.
    pub id: String,
    /// Its direction.
    pub direction: Direction,
}

/// One number of a cited item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CitedValue {
    /// The cited item.
    pub id: String,
    /// What the number is (e.g. `close`, `rsi`).
    pub key: String,
    /// The value.
    pub value: f64,
    /// Decimals the engine shows it with.
    pub decimals: u8,
}

/// The validator's language for a protocol language.
#[must_use]
pub fn lang_of(l: Language) -> Lang {
    match l {
        Language::Tr => Lang::Tr,
        Language::En => Lang::En,
    }
}

/// Lower case; Turkish maps `I` → `ı` and `İ` → `i`.
#[must_use]
pub fn lower(s: &str, lang: Lang) -> String {
    match lang {
        Lang::Tr => s
            .chars()
            .flat_map(|c| match c {
                'I' => vec!['ı'],
                'İ' => vec!['i'],
                c => c.to_lowercase().collect(),
            })
            .collect(),
        Lang::En => s.to_lowercase(),
    }
}

/// Splits into sentences: a break after `.`, `;`, `!`, `?` followed by whitespace or
/// the end (a decimal mark is never followed by whitespace). Empty pieces dropped.
#[must_use]
pub fn sentences(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end = matches!(c, '.' | ';' | '!' | '?')
            && chars.get(i + 1).is_none_or(|n| n.is_whitespace());
        if end {
            let t = cur.trim().to_owned();
            if !t.is_empty() {
                out.push(t);
            }
            cur.clear();
        }
    }
    let t = cur.trim().to_owned();
    if !t.is_empty() {
        out.push(t);
    }
    out
}
