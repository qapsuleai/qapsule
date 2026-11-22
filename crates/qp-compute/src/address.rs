//! Nested study addresses (K63): `ISCTR:5m:EMA[RSI:18]:20` = the 20-bar EMA of RSI(18).
//!
//! An address names a study chain in the compact form used by the Quantex price service
//! (`docs/matriks/INDICATORS.md`, "Anahtar biçimi" and "Gösterge dizgesi sözdizimi"). Only
//! the syntax is shared: the values are computed by this crate under K13 (one value per
//! bar, warmup `None`), never taken from Quantex.
//!
//! # Grammar
//!
//! ```text
//! address  = [ symbol ":" interval ":" ] expr
//! expr     = name [ selector ] [ "[" source "]" ] [ ":" number { (":" | ",") number } ]
//! selector = "(" output ")" | "." output
//! source   = field | expr
//! field    = "open" | "high" | "low" | "close" | "volume"
//! name     = study name or alias, case-insensitive (see below)
//! output   = output name of that study, case-insensitive
//! number   = decimal f64 literal (`20`, `2.5`, `1e1`)
//! interval = `1m`..`1440m`, `1h`..`24h`, `1d`, `1w`, `1M` (case-sensitive: `1M` is a month)
//! symbol   = any text without `:`, `[`, `]`, `(`, `)`
//! ```
//!
//! - **Nesting:** `OUTER[INNER]` feeds INNER's selected output into OUTER ([`Chain`]
//!   semantics: OUTER receives nothing while INNER warms up). Depth is arbitrary up to
//!   [`MAX_DEPTH`] (a stack guard); `SMA[EMA[RSI:14]:10]:5` is two levels deep.
//! - **Source:** without `[...]` a study reads `close`; `SMA[high]:20` reads the high.
//!   Quantex's derived sources `hl2`, `hlc3`, `ohlc4` are not supported (an error says so).
//! - **Parameters** are positional in the fixed order below, separated by `:` (Quantex)
//!   or `,` (`MACD:12,26,9` = `MACD:12:26:9`). Either all are given or none: no
//!   parameters means the defaults ([`crate::default_params`]); a partial list is an
//!   error (no silent defaults). Values are not truncated to integers (unlike Quantex):
//!   a non-integer period is an error.
//! - **Main output:** a multi-output study's *main* output is its first output in
//!   [`Study::outputs`] order: `macd` for MACD, `middle` for BB. A selector picks another
//!   one: `EMA[MACD(signal)]:9` or `EMA[MACD.signal]:9`. On the outermost study the
//!   selector sets [`Address::output`], the column a caller should read (the built study
//!   still produces all outputs).
//! - **Prefix:** `SYMBOL:INTERVAL:` is optional and only recognised together: the address
//!   has one when its second `:`-separated part is a valid interval. The interval is kept
//!   as text, the same type as the schema's bar interval (`qp_schema` `interval: String`);
//!   `qp_calendar::Interval::parse` accepts the same set.
//!
//! | Name (aliases)                      | Study       | Parameters           | Outputs (main first)              |
//! |-------------------------------------|-------------|----------------------|-----------------------------------|
//! | `SMA`                               | `sma`       | period               | value                             |
//! | `EMA`                               | `ema`       | period               | value                             |
//! | `RSI`                               | `rsi`       | period               | value                             |
//! | `MACD`                              | `macd`      | fast, slow, signal   | macd, signal, histogram           |
//! | `BB` (`BOLLINGER`, `BOLLINGERBANDS`) | `bollinger` | period, k            | middle, upper, lower, bandwidth   |
//!
//! Errors carry the byte offset of the offending text ([`AddressError::pos`]).

use std::collections::BTreeMap;
use std::fmt;

use crate::graph::Stage;
use crate::{create, default_params, Chain, ComputeError, Field, Study};

/// Deepest accepted nesting (number of studies in one chain). A guard against unbounded
/// recursion on hostile input; Quantex itself allows 4 levels.
pub const MAX_DEPTH: usize = 64;

/// Address names: (name or alias, study, positional parameter order).
const NAMES: &[(&str, &str, &[&str])] = &[
    ("sma", "sma", &["period"]),
    ("ema", "ema", &["period"]),
    ("rsi", "rsi", &["period"]),
    ("macd", "macd", &["fast", "slow", "signal"]),
    ("bb", "bollinger", &["period", "k"]),
    ("bollinger", "bollinger", &["period", "k"]),
    ("bollingerbands", "bollinger", &["period", "k"]),
];

/// Quantex sources Qapsule does not compute.
const UNSUPPORTED_SOURCES: [&str; 3] = ["hl2", "hlc3", "ohlc4"];

/// A parse error at byte offset [`AddressError::pos`] of the address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressError {
    /// Byte offset into the address where the problem is.
    pub pos: usize,
    /// What is wrong.
    pub msg: String,
}

impl AddressError {
    fn new(pos: usize, msg: impl Into<String>) -> Self {
        Self {
            pos,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for AddressError {}

/// What a study reads.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A bar field.
    Field(Field),
    /// The selected output of another study.
    Study(Box<Node>),
}

/// One study of an address, with its source.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Study name as [`create`] knows it (`sma`, `bollinger`, ...).
    pub study: &'static str,
    /// Effective parameters (defaults filled in).
    pub params: BTreeMap<String, f64>,
    /// Selected output index: the one fed to the enclosing study, or for the outermost
    /// study the column the address names. `0` (the main output) unless a selector is
    /// given.
    pub output: usize,
    /// What the study reads.
    pub source: Source,
}

/// A parsed address.
#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    /// Symbol, if the address has a prefix.
    pub symbol: Option<String>,
    /// Bar interval text (`5m`, `1d`, ...), if the address has a prefix.
    pub interval: Option<String>,
    /// The outermost study.
    pub root: Node,
}

impl Address {
    /// Parses an address (see the module documentation).
    ///
    /// # Errors
    /// Unknown study, field or output; missing, extra or invalid parameter; unbalanced
    /// brackets; invalid interval; trailing text. The error names the byte offset.
    pub fn parse(s: &str) -> Result<Self, AddressError> {
        let (symbol, interval, start) = split_prefix(s)?;
        let mut p = Parser { s, i: start };
        let root = p.expr(1)?;
        if p.i < s.len() {
            let c = p.peek().unwrap_or(' ');
            let msg = if c == ']' {
                "unbalanced `]`: no matching `[`".to_owned()
            } else {
                format!("unexpected `{c}` after the study")
            };
            return Err(AddressError::new(p.i, msg));
        }
        Ok(Self {
            symbol,
            interval,
            root,
        })
    }

    /// The column index of [`Address::build`]'s study that the address names.
    #[must_use]
    pub fn output(&self) -> usize {
        self.root.output
    }

    /// Builds the study chain (innermost study first, [`Chain`] per nesting level). Its
    /// inputs are the innermost study's bar field; it has the outermost study's outputs.
    ///
    /// # Errors
    /// Invalid parameters (already checked by [`Address::parse`]).
    pub fn build(&self) -> Result<Box<dyn Study>, ComputeError> {
        self.root.build()
    }

    /// The same chain as graph stages, innermost first, with the bar field the first
    /// stage reads (for [`crate::graph::sweep_chains`]).
    #[must_use]
    pub fn stages(&self) -> (Field, Vec<Stage>) {
        let mut stages = Vec::new();
        let mut node = &self.root;
        loop {
            stages.push(Stage {
                study: node.study.to_owned(),
                params: node.params.clone(),
                output: node.output,
            });
            match &node.source {
                Source::Field(f) => {
                    stages.reverse();
                    return (*f, stages);
                }
                Source::Study(inner) => node = inner,
            }
        }
    }
}

impl Node {
    /// Builds this study reading its source (see [`Address::build`]).
    ///
    /// # Errors
    /// Invalid parameters.
    pub fn build(&self) -> Result<Box<dyn Study>, ComputeError> {
        match &self.source {
            Source::Field(f) => create(self.study, &self.params, *f),
            Source::Study(inner) => {
                let first = inner.build()?;
                let second = create(self.study, &self.params, Field::Close)?;
                Ok(Box::new(Chain::new(first, inner.output, second)))
            }
        }
    }
}

/// Canonical text: upper-case names, explicit output selector when not the main one,
/// explicit source when not `close`, every parameter in positional order.
impl fmt::Display for Node {
    #[allow(clippy::expect_used)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry = NAMES
            .iter()
            .find(|(_, s, _)| *s == self.study)
            .expect("node study is a known name");
        f.write_str(&entry.0.to_ascii_uppercase())?;
        if self.output != 0 {
            write!(f, "({})", outputs_of(self.study)[self.output])?;
        }
        match &self.source {
            Source::Field(Field::Close) => {}
            Source::Field(field) => write!(f, "[{}]", field.name())?,
            Source::Study(inner) => write!(f, "[{inner}]")?,
        }
        for key in entry.2 {
            write!(f, ":{}", self.params[*key])?;
        }
        Ok(())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(s), Some(i)) = (&self.symbol, &self.interval) {
            write!(f, "{s}:{i}:")?;
        }
        write!(f, "{}", self.root)
    }
}

/// Whether `s` is a bar interval accepted by the schema (`1m`..`1440m`, `1h`..`24h`,
/// `1d`, `1w`, `1M`).
#[must_use]
pub fn is_interval(s: &str) -> bool {
    let Some(split) = s.find(|c: char| !c.is_ascii_digit()) else {
        return false;
    };
    let (num, unit) = s.split_at(split);
    let Ok(n) = num.parse::<u32>() else {
        return false;
    };
    match unit {
        "m" => (1..=1440).contains(&n),
        "h" => (1..=24).contains(&n),
        "d" | "w" | "M" => n == 1,
        _ => false,
    }
}

/// Splits off `SYMBOL:INTERVAL:` when the second `:`-part is an interval. Returns the
/// symbol, the interval and the offset where the study expression starts.
#[allow(clippy::type_complexity)]
fn split_prefix(s: &str) -> Result<(Option<String>, Option<String>, usize), AddressError> {
    let Some(c1) = s.find(':') else {
        return Ok((None, None, 0));
    };
    let first = &s[..c1];
    // A first part with brackets or a selector is a study, never a symbol.
    if first.contains(['[', ']', '(', ')', '.']) && lookup(name_prefix(first)).is_some() {
        return Ok((None, None, 0));
    }
    let rest = &s[c1 + 1..];
    let c2 = rest.find(':');
    let second = c2.map_or(rest, |c| &rest[..c]);
    if c2.is_some() && is_interval(second) {
        if first.is_empty() || first.contains(['[', ']', '(', ')']) {
            return Err(AddressError::new(0, format!("invalid symbol `{first}`")));
        }
        let start = c1 + 1 + second.len() + 1;
        return Ok((Some(first.to_owned()), Some(second.to_owned()), start));
    }
    if lookup(first).is_some() {
        return Ok((None, None, 0));
    }
    if c2.is_some() {
        return Err(AddressError::new(
            c1 + 1,
            format!(
                "invalid interval `{second}`; expected 1m..1440m, 1h..24h, 1d, 1w or 1M \
                 (after the symbol `{first}`)"
            ),
        ));
    }
    // `NAME:...` with an unknown name: let the study parser report it.
    Ok((None, None, 0))
}

/// The leading identifier of `s`.
fn name_prefix(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

/// The study and parameter order of an address name (case-insensitive).
fn lookup(name: &str) -> Option<(&'static str, &'static [&'static str])> {
    NAMES
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, s, p)| (*s, *p))
}

#[allow(clippy::expect_used)]
/// Output names of a known study.
fn outputs_of(study: &str) -> &'static [&'static str] {
    let params = default_params(study).expect("known study");
    create(study, &params, Field::Close)
        .expect("defaults are valid")
        .outputs()
}

fn supported() -> String {
    NAMES
        .iter()
        .map(|(n, _, _)| n.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(", ")
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn ident(&mut self) -> &str {
        let start = self.i;
        let len = name_prefix(&self.s[start..]).len();
        self.i += len;
        &self.s[start..self.i]
    }

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    fn expr(&mut self, depth: usize) -> Result<Node, AddressError> {
        if depth > MAX_DEPTH {
            return Err(AddressError::new(
                self.i,
                format!("nesting deeper than {MAX_DEPTH} studies"),
            ));
        }
        let start = self.i;
        let name = self.ident();
        if name.is_empty() {
            let what = self
                .peek()
                .map_or("end of address".to_owned(), |c| format!("`{c}`"));
            return Err(AddressError::new(
                start,
                format!("expected a study name, found {what}"),
            ));
        }
        let (study, order) = lookup(name).ok_or_else(|| {
            AddressError::new(
                start,
                format!("unknown study `{name}`; expected one of: {}", supported()),
            )
        })?;
        let name = name.to_owned();

        // Output selector.
        let mut output = 0;
        if let Some(open @ ('(' | '.')) = self.peek() {
            let at = self.i;
            self.i += 1;
            let sel_at = self.i;
            let sel = self.ident().to_owned();
            if open == '(' {
                if self.peek() != Some(')') {
                    return Err(AddressError::new(
                        self.i,
                        format!("expected `)` to close `(` at byte {at}"),
                    ));
                }
                self.i += 1;
            }
            let outs = outputs_of(study);
            output = outs
                .iter()
                .position(|o| o.eq_ignore_ascii_case(&sel))
                .ok_or_else(|| {
                    AddressError::new(
                        sel_at,
                        format!(
                            "`{name}` has no output `{sel}`; expected one of: {}",
                            outs.join(", ")
                        ),
                    )
                })?;
        }

        // Source.
        let mut source = Source::Field(Field::Close);
        if self.peek() == Some('[') {
            let open = self.i;
            self.i += 1;
            let at = self.i;
            let word = name_prefix(&self.s[at..]);
            let after = self.s[at + word.len()..].chars().next();
            let field = Field::ALL
                .into_iter()
                .find(|f| f.name().eq_ignore_ascii_case(word));
            if let (Some(f), Some(']')) = (field, after) {
                self.i += word.len();
                source = Source::Field(f);
            } else if UNSUPPORTED_SOURCES
                .iter()
                .any(|u| u.eq_ignore_ascii_case(word))
                && after == Some(']')
            {
                return Err(AddressError::new(
                    at,
                    format!(
                        "source `{word}` is not supported; use open, high, low, close, \
                         volume or a study"
                    ),
                ));
            } else {
                source = Source::Study(Box::new(self.expr(depth + 1)?));
            }
            if self.peek() != Some(']') {
                return Err(AddressError::new(
                    self.i,
                    format!("unbalanced `[` at byte {open}: expected `]`"),
                ));
            }
            self.i += 1;
        }

        // Parameters.
        let mut values: Vec<(usize, f64)> = Vec::new();
        if self.peek() == Some(':') {
            loop {
                self.i += 1;
                let at = self.i;
                let len = self.s[at..]
                    .find(|c: char| {
                        !(c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
                    })
                    .unwrap_or(self.s.len() - at);
                let text = &self.s[at..at + len];
                let v = text
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| {
                        let found = if text.is_empty() {
                            self.peek()
                                .map_or("end of address".to_owned(), |c| format!("`{c}`"))
                        } else {
                            format!("`{text}`")
                        };
                        AddressError::new(
                            at,
                            format!("expected a number for `{name}`, found {found}"),
                        )
                    })?;
                self.i += len;
                values.push((at, v));
                if !matches!(self.peek(), Some(':' | ',')) {
                    break;
                }
            }
        }
        let mut params = default_params(study).expect("known study");
        if !values.is_empty() {
            if values.len() < order.len() {
                return Err(AddressError::new(
                    self.i,
                    format!(
                        "missing parameter `{}` for `{name}`; expects {} ({}) or none for \
                         the defaults",
                        order[values.len()],
                        order.len(),
                        order.join(", ")
                    ),
                ));
            }
            if values.len() > order.len() {
                return Err(AddressError::new(
                    values[order.len()].0,
                    format!(
                        "too many parameters for `{name}`; expects {} ({})",
                        order.len(),
                        order.join(", ")
                    ),
                ));
            }
            for (key, (_, v)) in order.iter().zip(&values) {
                params.insert((*key).to_owned(), *v);
            }
        }
        // Validate the values now so the error points at the study.
        create(study, &params, Field::Close)
            .map_err(|e| AddressError::new(start, format!("`{name}`: {e}")))?;
        Ok(Node {
            study,
            params,
            output,
            source,
        })
    }
}
