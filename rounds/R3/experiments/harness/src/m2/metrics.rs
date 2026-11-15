//! M2 automatic metrics (`harness m2 measure --alt <name>`), per language:
//!
//! 1. **Time correctness (K26)**: expected class from the truth — `event` (past marker +
//!    a time reference), `state_holding` (present marker, no "no longer"), `state_ended`
//!    (past marker + "no longer"), `expectation` (conditional marker); violations listed.
//!    The time reference's exactness (equal to the true bars-ago) is reported separately.
//! 2. **Determinism**: each sentence produced twice; byte-equal rate.
//! 3. **Leakage** (redacted situations): any ASCII digit, any number word (lexicon), and,
//!    for diagnosis, which licensed evidence values appear in a rounded form.
//!    **Invented numbers** (other situations): every number token must equal an engine slot
//!    under the formatting rule: a token with `k` decimals (`,` or `.` as decimal mark,
//!    `.`/`,` groups of three as thousands) equals `round(v, k)` or `round(|v|, k)` for
//!    some slot `v` (evidence values of the lead and latest fact, current value, ledger
//!    n / realized / missed / percentage, bars-ago, bars since the last transition, horizon,
//!    within-bars, strength and strength×100, family count, tier, bar indices). Dates,
//!    abbreviations ("1,2M") and spelled-out numbers are not parsed (a date counts as
//!    invented).
//! 4. **Cost** (`cost.json`, not deterministic, never compared byte-wise): ns per sentence
//!    over repeated passes; for a file-based (LLM) alternative also the per-call seconds and
//!    tokens it recorded in `rounds/R3/experiments/M2/<alt>/calls.json`
//!    (`[{"id": "s01", "lang": "tr", "seconds": 1.2, "input_tokens": 900, "output_tokens": 40}]`).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use qp_capsule::{Lang, Realizer};
use qp_schema::capsule::TimeKind;
use serde::{Deserialize, Serialize};

use super::lexicon::{features, has_digit, number_words, tokens, Features};
use super::situations::{all, Situation};
use super::{lang_name, LANGS};
use crate::io::{read, round, write, write_json};

/// One number token and the slot it matched.
#[derive(Debug, Clone, Serialize)]
pub struct NumberCheck {
    /// Token text.
    pub token: String,
    /// Matched slot name, `None` = invented.
    pub slot: Option<String>,
}

/// Result for one sentence.
#[derive(Debug, Clone, Serialize)]
pub struct SentenceResult {
    /// Situation id.
    pub id: String,
    /// The sentence.
    pub sentence: String,
    /// Second call gave the same bytes.
    pub deterministic: bool,
    /// Expected time class.
    pub expected: String,
    /// Surface features.
    pub features: Features,
    /// Time-correctness violations (empty = correct).
    pub violations: Vec<String>,
    /// Redacted situation.
    pub redacted: bool,
    /// Redacted only: leakage (digit or number word).
    pub leak: bool,
    /// Number words found.
    pub number_words: Vec<String>,
    /// Number tokens and their slots.
    pub numbers: Vec<NumberCheck>,
    /// Non-redacted only: number tokens that match no slot.
    pub invented: usize,
}

/// Summary of one language.
#[derive(Debug, Clone, Serialize)]
pub struct LangSummary {
    /// Sentences.
    pub sentences: usize,
    /// Empty sentences.
    pub empty: usize,
    /// Time-correct rate (all 73).
    pub time_correct: f64,
    /// Time-correct rate per expected class.
    pub time_correct_by_class: BTreeMap<String, f64>,
    /// Violation counts.
    pub violations: BTreeMap<String, usize>,
    /// Exact time reference rate among situations with a true bars-ago (events, ended states).
    pub time_ref_exact: f64,
    /// K26 time-correct.
    pub k26_correct: bool,
    /// Byte-equal rate over two calls.
    pub determinism: f64,
    /// Redacted situations.
    pub redacted: usize,
    /// Of which leaking.
    pub leaks: usize,
    /// Non-redacted situations.
    pub open: usize,
    /// Sentences with at least one invented number.
    pub sentences_with_invented: usize,
    /// Invented number tokens.
    pub invented_numbers: usize,
}

/// `auto.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Auto {
    /// Harness version.
    pub harness: String,
    /// Alternative.
    pub alt: String,
    /// Per language summary.
    pub summary: BTreeMap<String, LangSummary>,
    /// Per language, per sentence.
    pub sentences: BTreeMap<String, Vec<SentenceResult>>,
}

/// The engine slots of a situation, `(name, value)`.
#[must_use]
pub fn slots(s: &Situation) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = Vec::new();
    for (k, x) in &s.lead.evidence {
        v.push((format!("lead.{k}"), *x));
    }
    for (k, x) in &s.latest.evidence {
        v.push((format!("latest.{k}"), *x));
    }
    if let Some(c) = s.current {
        v.push(("current".into(), c));
    }
    let st = &s.story;
    let l = st.ledger_at_birth;
    v.push(("ledger.n".into(), f64::from(l.n)));
    v.push(("ledger.realized".into(), f64::from(l.realized)));
    v.push(("ledger.missed".into(), f64::from(l.n - l.realized)));
    if l.n > 0 {
        v.push((
            "ledger.pct".into(),
            100.0 * f64::from(l.realized) / f64::from(l.n),
        ));
    }
    #[allow(clippy::cast_precision_loss)]
    {
        v.push(("bars_ago".into(), s.bars_ago() as f64));
        v.push((
            "bars_since_last".into(),
            s.now_bar.saturating_sub(st.last_bar) as f64,
        ));
        v.push(("lead.bar".into(), s.lead.bar as f64));
        v.push(("latest.bar".into(), s.latest.bar as f64));
        v.push(("now_bar".into(), s.now_bar as f64));
        v.push(("families".into(), st.families.len() as f64));
    }
    v.push(("horizon".into(), f64::from(st.horizon_bars)));
    if let Some(w) = s
        .lead
        .meaning
        .as_ref()
        .and_then(|m| m.condition.as_ref())
        .and_then(|c| c.within_bars)
    {
        v.push(("within_bars".into(), f64::from(w)));
    }
    v.push(("strength".into(), st.strength));
    v.push(("strength.pct".into(), st.strength * 100.0));
    v.push(("tier".into(), f64::from(st.tier)));
    v
}

/// Candidate `(value, decimals)` readings of a number token.
#[must_use]
pub fn readings(tok: &str) -> Vec<(f64, i32)> {
    let seps: Vec<(usize, char)> = tok
        .char_indices()
        .filter(|(_, c)| *c == '.' || *c == ',')
        .collect();
    let digits_only = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    let parse = |s: &str| s.parse::<f64>().ok();
    let mut out = Vec::new();
    match seps.len() {
        0 => {
            if let Some(v) = parse(tok) {
                out.push((v, 0));
            }
        }
        1 => {
            let (i, _) = seps[0];
            let frac = &tok[i + 1..];
            if let Some(v) = parse(&format!("{}.{}", &tok[..i], frac)) {
                out.push((v, i32::try_from(frac.len()).unwrap_or(0)));
            }
            if frac.len() == 3 {
                if let Some(v) = parse(&digits_only(tok)) {
                    out.push((v, 0));
                }
            }
        }
        _ => {
            let last = seps[seps.len() - 1];
            let all_same = seps.iter().all(|(_, c)| *c == last.1);
            if all_same {
                if let Some(v) = parse(&digits_only(tok)) {
                    out.push((v, 0));
                }
            } else {
                let int = digits_only(&tok[..last.0]);
                let frac = &tok[last.0 + 1..];
                if let Some(v) = parse(&format!("{int}.{frac}")) {
                    out.push((v, i32::try_from(frac.len()).unwrap_or(0)));
                }
            }
        }
    }
    out
}

/// The slot a number token matches, if any.
#[must_use]
pub fn match_slot(tok: &str, slots: &[(String, f64)]) -> Option<String> {
    for (v, k) in readings(tok) {
        for (name, x) in slots {
            for cand in [*x, x.abs()] {
                let r = round(cand, k);
                if (r - v).abs() <= 1e-9 * v.abs().max(1.0) {
                    return Some(name.clone());
                }
            }
        }
    }
    None
}

/// Expected time class of a situation.
#[must_use]
pub fn expected_class(s: &Situation) -> &'static str {
    match s.story.time {
        TimeKind::Event => "event",
        TimeKind::State if s.truth.when == "now" => "state_holding",
        TimeKind::State => "state_ended",
        TimeKind::Expectation => "expectation",
    }
}

/// Violations of the expected class given the features.
#[must_use]
pub fn violations(class: &str, f: &Features) -> Vec<String> {
    let mut v = Vec::new();
    let mut need = |ok: bool, name: &str| {
        if !ok {
            v.push(name.to_owned());
        }
    };
    match class {
        "event" => {
            need(f.past, "missing_past");
            need(f.time_ref, "missing_time_ref");
        }
        "state_holding" => {
            need(f.present, "missing_present");
            need(!f.no_longer, "says_no_longer");
        }
        "state_ended" => {
            need(f.past, "missing_past");
            need(f.no_longer, "missing_no_longer");
        }
        _ => need(f.conditional, "missing_conditional"),
    }
    v
}

/// Checks one sentence of one situation.
#[must_use]
pub fn check(s: &Situation, sentence: &str, second: &str, lang: Lang) -> SentenceResult {
    let class = expected_class(s);
    let f = features(sentence, lang, s.truth.bars_ago);
    let sl = slots(s);
    let numbers: Vec<NumberCheck> = tokens(sentence, lang)
        .into_iter()
        .filter(|t| t.starts_with(|c: char| c.is_ascii_digit()))
        .map(|t| NumberCheck {
            slot: match_slot(&t, &sl),
            token: t,
        })
        .collect();
    let nw = number_words(sentence, lang);
    let leak = s.redacted && (has_digit(sentence) || !nw.is_empty());
    let invented = if s.redacted {
        0
    } else {
        numbers.iter().filter(|n| n.slot.is_none()).count()
    };
    SentenceResult {
        id: s.id.clone(),
        sentence: sentence.to_owned(),
        deterministic: sentence == second,
        expected: class.to_owned(),
        features: f,
        violations: violations(class, &f),
        redacted: s.redacted,
        leak,
        number_words: nw,
        numbers,
        invented,
    }
}

#[allow(clippy::cast_precision_loss)]
fn rate(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        round(a as f64 / b as f64, 4)
    }
}

fn summarize(rs: &[SentenceResult], sits: &[Situation]) -> LangSummary {
    let ok = |r: &SentenceResult| r.violations.is_empty();
    let mut by_class: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut viol: BTreeMap<String, usize> = BTreeMap::new();
    for r in rs {
        let e = by_class.entry(r.expected.clone()).or_default();
        e.1 += 1;
        if ok(r) {
            e.0 += 1;
        }
        for v in &r.violations {
            *viol.entry(v.clone()).or_default() += 1;
        }
    }
    let with_ago: Vec<&SentenceResult> = rs
        .iter()
        .zip(sits)
        .filter(|(_, s)| s.truth.bars_ago.is_some())
        .map(|(r, _)| r)
        .collect();
    let red: Vec<&SentenceResult> = rs.iter().filter(|r| r.redacted).collect();
    let open: Vec<&SentenceResult> = rs.iter().filter(|r| !r.redacted).collect();
    LangSummary {
        sentences: rs.len(),
        empty: rs.iter().filter(|r| r.sentence.trim().is_empty()).count(),
        time_correct: rate(rs.iter().filter(|r| ok(r)).count(), rs.len()),
        time_correct_by_class: by_class
            .into_iter()
            .map(|(k, (a, b))| (k, rate(a, b)))
            .collect(),
        violations: viol,
        time_ref_exact: rate(
            with_ago
                .iter()
                .filter(|r| r.features.time_ref_exact)
                .count(),
            with_ago.len(),
        ),
        k26_correct: rs.iter().any(|r| r.id == "k26" && ok(r)),
        determinism: rate(rs.iter().filter(|r| r.deterministic).count(), rs.len()),
        redacted: red.len(),
        leaks: red.iter().filter(|r| r.leak).count(),
        open: open.len(),
        sentences_with_invented: open.iter().filter(|r| r.invented > 0).count(),
        invented_numbers: open.iter().map(|r| r.invented).sum(),
    }
}

/// Runs the deterministic metrics.
#[must_use]
pub fn measure(r: &dyn Realizer) -> Auto {
    let sits = all();
    let mut summary = BTreeMap::new();
    let mut sentences = BTreeMap::new();
    for lang in LANGS {
        let rs: Vec<SentenceResult> = sits
            .iter()
            .map(|s| {
                let a = r.sentence(&s.input(), lang);
                let b = r.sentence(&s.input(), lang);
                check(s, &a, &b, lang)
            })
            .collect();
        summary.insert(lang_name(lang).to_owned(), summarize(&rs, &sits));
        sentences.insert(lang_name(lang).to_owned(), rs);
    }
    Auto {
        harness: crate::HARNESS_VERSION.to_owned(),
        alt: r.name().to_owned(),
        summary,
        sentences,
    }
}

/// One LLM call recorded by a file-based alternative.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Call {
    /// Situation id.
    pub id: String,
    /// `tr` / `en`.
    pub lang: String,
    /// Wall seconds.
    pub seconds: f64,
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
}

/// `cost.json` (not deterministic).
#[derive(Debug, Clone, Serialize)]
pub struct Cost {
    /// Alternative.
    pub alt: String,
    /// Note.
    pub note: String,
    /// Per language: mean ns per sentence of the in-process `sentence()` call.
    pub ns_per_sentence: BTreeMap<String, f64>,
    /// Passes timed.
    pub passes: usize,
    /// LLM calls summary, if `M2/<alt>/calls.json` exists.
    pub llm: Option<LlmCost>,
}

/// Summary of recorded LLM calls.
#[derive(Debug, Clone, Serialize)]
pub struct LlmCost {
    /// Calls.
    pub calls: usize,
    /// Mean seconds per call.
    pub mean_seconds: f64,
    /// Max seconds.
    pub max_seconds: f64,
    /// Mean input tokens.
    pub mean_input_tokens: f64,
    /// Mean output tokens.
    pub mean_output_tokens: f64,
}

/// Times the realizer (`passes` passes over all situations per language) and reads the
/// alternative's `calls.json` if present.
///
/// # Errors
/// A malformed `calls.json`.
pub fn cost(r: &dyn Realizer, root: &Path, passes: usize) -> Result<Cost, String> {
    let sits = all();
    let mut ns = BTreeMap::new();
    for lang in LANGS {
        let mut sink = 0usize;
        let t = Instant::now();
        for _ in 0..passes {
            for s in &sits {
                sink = sink.wrapping_add(r.sentence(&s.input(), lang).len());
            }
        }
        let el = t.elapsed().as_nanos();
        std::hint::black_box(sink);
        #[allow(clippy::cast_precision_loss)]
        let per = el as f64 / (passes * sits.len()).max(1) as f64;
        ns.insert(lang_name(lang).to_owned(), per.round());
    }
    let calls_path = root.join("M2").join(r.name()).join("calls.json");
    let llm = if calls_path.exists() {
        let calls: Vec<Call> = serde_json::from_str(&read(&calls_path)?)
            .map_err(|e| format!("{}: {e}", calls_path.display()))?;
        #[allow(clippy::cast_precision_loss)]
        let n = calls.len().max(1) as f64;
        Some(LlmCost {
            calls: calls.len(),
            mean_seconds: round(calls.iter().map(|c| c.seconds).sum::<f64>() / n, 3),
            max_seconds: calls.iter().map(|c| c.seconds).fold(0.0, f64::max),
            #[allow(clippy::cast_precision_loss)]
            mean_input_tokens: round(
                calls.iter().map(|c| c.input_tokens as f64).sum::<f64>() / n,
                1,
            ),
            #[allow(clippy::cast_precision_loss)]
            mean_output_tokens: round(
                calls.iter().map(|c| c.output_tokens as f64).sum::<f64>() / n,
                1,
            ),
        })
    } else {
        None
    };
    Ok(Cost {
        alt: r.name().to_owned(),
        note: "wall-clock timing on this machine (release build recommended); not deterministic, \
               not part of any byte-equality claim"
            .to_owned(),
        ns_per_sentence: ns,
        passes,
        llm,
    })
}

/// Markdown summary of the automatic metrics.
#[must_use]
pub fn markdown(a: &Auto, c: Option<&Cost>) -> String {
    let mut m = format!(
        "# M2 automatic metrics — `{}`\n\n| metric | tr | en |\n|---|---:|---:|\n",
        a.alt
    );
    let row = |name: &str, f: &dyn Fn(&LangSummary) -> String| {
        format!(
            "| {name} | {} | {} |\n",
            f(&a.summary["tr"]),
            f(&a.summary["en"])
        )
    };
    m.push_str(&row("time correct (73)", &|s| {
        format!("{:.2}", s.time_correct)
    }));
    for class in ["event", "state_holding", "state_ended", "expectation"] {
        m.push_str(&row(&format!("time correct: {class}"), &|s| {
            format!(
                "{:.2}",
                s.time_correct_by_class.get(class).copied().unwrap_or(0.0)
            )
        }));
    }
    m.push_str(&row("K26 time correct", &|s| s.k26_correct.to_string()));
    m.push_str(&row("exact time reference", &|s| {
        format!("{:.2}", s.time_ref_exact)
    }));
    m.push_str(&row("determinism (byte-equal)", &|s| {
        format!("{:.2}", s.determinism)
    }));
    m.push_str(&row("leaks / redacted", &|s| {
        format!("{}/{}", s.leaks, s.redacted)
    }));
    m.push_str(&row("sentences with invented numbers / open", &|s| {
        format!("{}/{}", s.sentences_with_invented, s.open)
    }));
    m.push_str(&row("invented number tokens", &|s| {
        s.invented_numbers.to_string()
    }));
    m.push_str(&row("empty sentences", &|s| s.empty.to_string()));
    if let Some(c) = c {
        m.push_str(&format!(
            "| ns per sentence (timing, not deterministic) | {} | {} |\n",
            c.ns_per_sentence["tr"], c.ns_per_sentence["en"]
        ));
    }
    m.push_str("\n## Violations\n\n| violation | tr | en |\n|---|---:|---:|\n");
    let mut keys: Vec<&String> = a.summary["tr"]
        .violations
        .keys()
        .chain(a.summary["en"].violations.keys())
        .collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        m.push_str(&format!(
            "| {k} | {} | {} |\n",
            a.summary["tr"].violations.get(k).copied().unwrap_or(0),
            a.summary["en"].violations.get(k).copied().unwrap_or(0)
        ));
    }
    m
}

/// Measures and writes `auto.json`, `auto.md` and `cost.json` under `M2/raw/<alt>/`.
///
/// # Errors
/// I/O failure.
pub fn run(r: &dyn Realizer, root: &Path, passes: usize) -> Result<(Auto, Cost), String> {
    crate::io::check_name(r.name())?;
    let a = measure(r);
    let c = cost(r, root, passes)?;
    let dir = super::raw_dir(root, r.name());
    write_json(&dir.join("auto.json"), &a)?;
    write_json(&dir.join("cost.json"), &c)?;
    write(&dir.join("auto.md"), markdown(&a, None).as_bytes())?;
    Ok((a, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m2::situations::k26;

    #[test]
    fn number_readings() {
        assert_eq!(readings("70"), vec![(70.0, 0)]);
        assert_eq!(readings("71,4"), vec![(71.4, 1)]);
        assert_eq!(readings("1.250"), vec![(1.25, 3), (1250.0, 0)]);
        assert_eq!(readings("1.250.000"), vec![(1_250_000.0, 0)]);
        assert_eq!(readings("1.250,5"), vec![(1250.5, 1)]);
    }

    #[test]
    fn k26_good_and_bad() {
        let k = k26();
        let good = "RSI 3 bar önce 70'in üstüne çıktı; şu an 55, artık 70'in üstünde değil.";
        let r = check(&k, good, good, Lang::Tr);
        assert!(r.violations.is_empty(), "{:?}", r.violations);
        assert_eq!(r.invented, 0);
        let bad = "RSI 70'in üstüne çıkıyor; şu an 56.";
        let r = check(&k, bad, "x", Lang::Tr);
        assert!(!r.violations.is_empty());
        assert_eq!(r.invented, 1);
        assert!(!r.deterministic);
    }

    #[test]
    fn leakage() {
        let mut k = k26();
        k.redacted = true;
        assert!(check(&k, "RSI yetmişin üstüne çıktı.", "", Lang::Tr).leak);
        assert!(check(&k, "RSI 70 üstü.", "", Lang::Tr).leak);
        assert!(!check(&k, "RSI eşiğin üstüne çıktı, artık değil.", "", Lang::Tr).leak);
    }
}
