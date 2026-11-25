//! Collecting program errors and warnings (PR §11.1): the checker never stops at the
//! first finding. Errors are ordered by position (root, bound values, `params`, `nodes`,
//! `outputs`; indices numerically), then reason, then discovery — and **then** cut at
//! `program.check.max_errors`: the kept errors are the first ones in that order, whatever
//! order they were found in. Memory stays bounded: a max-heap holds the best `cap` so far
//! (a hostile program cannot make the answer or the work grow), the rest is only counted.
//! A finding that will not be kept skips its detail (`did_you_mean` is computed lazily).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use qp_schema::protocol::ProgramError;

use crate::message::{Lang, Msg};
use crate::reason::Reason;

/// Longest message kept, in characters (a message quotes names from the input).
pub(crate) const MAX_MESSAGE: usize = 512;

fn bounded(mut m: String) -> String {
    if let Some((i, _)) = m.char_indices().nth(MAX_MESSAGE) {
        m.truncate(i);
        m.push('…');
    }
    m
}

/// One segment of a pointer, ordered numerically when both sides are numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seg(String);

impl Ord for Seg {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0.parse::<u64>(), other.0.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => self.0.cmp(&other.0),
        }
    }
}

impl PartialOrd for Seg {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The sort key of a finding: section, pointer segments, reason, discovery order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    section: u8,
    segs: Vec<Seg>,
    reason: String,
    seq: u64,
}

impl Key {
    fn of(e: &ProgramError, seq: u64) -> Key {
        let segs: Vec<Seg> =
            e.at.as_deref()
                .unwrap_or("")
                .split('/')
                .skip(1)
                .map(|s| Seg(s.to_owned()))
                .collect();
        Key {
            section: section(segs.first().map(|s| s.0.as_str()), e.at.is_none()),
            segs,
            reason: e.reason.clone(),
            seq,
        }
    }
}

/// A kept finding, ordered by its key (the heap's top is the worst kept one).
#[derive(Debug)]
struct Kept {
    key: Key,
    e: ProgramError,
}

impl PartialEq for Kept {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Kept {}
impl Ord for Kept {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
impl PartialOrd for Kept {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Errors and warnings of one check.
#[derive(Debug)]
pub(crate) struct Diags {
    /// The kept errors in order, after [`Diags::finish`].
    pub(crate) errors: Vec<ProgramError>,
    pub(crate) warnings: Vec<ProgramError>,
    pub(crate) lang: Lang,
    /// Errors kept at most.
    cap: usize,
    /// The best `cap` errors so far (worst on top).
    heap: BinaryHeap<Kept>,
    /// The error pushed last, kept, still open for detail (enters `heap` on the next push).
    pending: Option<Kept>,
    /// Errors found and not kept (counted only).
    dropped: usize,
    /// Findings so far (the discovery order tie-break).
    seq: u64,
    /// Costly suggestion searches left ([`Found::dym_with`]).
    searches: usize,
    /// Where a dropped error is written (and forgotten).
    sink: ProgramError,
}

/// Costly suggestion searches (over node ids) one check runs at most: each looks at
/// `names::MAX_CANDIDATES` names at most, so the whole check's suggestion work is bounded
/// even when `program.check.max_errors` is raised to the node count.
pub(crate) const MAX_SEARCHES: usize = 256;

/// Adds detail to the finding just pushed.
pub(crate) struct Found<'a> {
    e: &'a mut ProgramError,
    lang: Lang,
    /// The check's remaining search budget.
    searches: Option<&'a mut usize>,
    /// Whether the finding is kept (detail on a dropped one is skipped).
    kept: bool,
}

impl Found<'_> {
    /// Accepted values or forms.
    pub(crate) fn expected<S: ToString>(self, v: impl IntoIterator<Item = S>) -> Self {
        if self.kept {
            self.e.expected = v.into_iter().map(|s| s.to_string()).collect();
        }
        self
    }

    /// Close names (at most 5).
    pub(crate) fn dym(self, v: Vec<String>) -> Self {
        if self.kept {
            self.e.did_you_mean = v.into_iter().take(5).collect();
        }
        self
    }

    /// Close names computed only when the finding is kept and the check's search budget
    /// ([`MAX_SEARCHES`]) is not spent (the search can be costly).
    pub(crate) fn dym_with(mut self, f: impl FnOnce() -> Vec<String>) -> Self {
        if let Some(left) = self.searches.as_deref_mut().filter(|n| **n > 0) {
            *left -= 1;
            self.e.did_you_mean = f().into_iter().take(5).collect();
        }
        self
    }

    /// How to fix it.
    pub(crate) fn hint(self, h: &Msg) -> Self {
        if self.kept {
            self.e.hint = Some(h.render(self.lang));
        }
        self
    }
}

impl Diags {
    pub(crate) fn new(lang: Lang, cap: usize) -> Diags {
        Diags {
            errors: Vec::new(),
            warnings: Vec::new(),
            lang,
            cap,
            heap: BinaryHeap::new(),
            pending: None,
            dropped: 0,
            seq: 0,
            searches: MAX_SEARCHES,
            sink: Reason::Type.error(None, String::new()),
        }
    }

    /// Moves the pending error into the heap, dropping the worst beyond `cap`.
    fn flush(&mut self) {
        if let Some(k) = self.pending.take() {
            self.heap.push(k);
            if self.heap.len() > self.cap {
                self.heap.pop();
                self.dropped += 1;
            }
        }
    }

    /// Pushes a finding at JSON pointer `at` (`None`: outside the program, e.g. a bound
    /// value) and returns it for detail.
    pub(crate) fn push(&mut self, reason: Reason, at: Option<&str>, msg: &Msg) -> Found<'_> {
        let lang = self.lang;
        self.seq += 1;
        let at = at.map(clamp);
        if reason.is_warning() {
            let e = reason.error(at, bounded(msg.render(lang)));
            self.warnings.push(e);
            let last = self.warnings.len() - 1;
            return Found {
                e: &mut self.warnings[last],
                lang,
                searches: None,
                kept: true,
            };
        }
        self.flush();
        // Decide on the key alone before rendering anything.
        let probe = reason.error(at, String::new());
        let key = Key::of(&probe, self.seq);
        let keep = self.cap > 0
            && (self.heap.len() < self.cap || self.heap.peek().is_some_and(|w| key < w.key));
        if !keep {
            self.dropped += 1;
            self.sink = probe;
            return Found {
                e: &mut self.sink,
                lang,
                searches: None,
                kept: false,
            };
        }
        let mut e = probe;
        e.message = bounded(msg.render(lang));
        let kept = self.pending.insert(Kept { key, e });
        Found {
            e: &mut kept.e,
            lang,
            searches: Some(&mut self.searches),
            kept: true,
        }
    }

    /// Shorthand for an error at a pointer.
    pub(crate) fn at(&mut self, reason: Reason, at: &str, msg: &Msg) -> Found<'_> {
        self.push(reason, Some(at), msg)
    }

    /// Error count so far (dropped ones included).
    pub(crate) fn count(&self) -> usize {
        self.heap.len() + usize::from(self.pending.is_some()) + self.dropped
    }

    /// Sorts the kept errors and the warnings into [`Diags::errors`] and
    /// [`Diags::warnings`], cut at `max`; returns whether errors were cut.
    pub(crate) fn finish(&mut self, max: usize) -> bool {
        self.flush();
        let mut kept = std::mem::take(&mut self.heap).into_sorted_vec();
        let cut = kept.len() > max || self.dropped > 0;
        kept.truncate(max);
        self.errors = kept.into_iter().map(|k| k.e).collect();
        let mut warnings: Vec<(Key, ProgramError)> = std::mem::take(&mut self.warnings)
            .into_iter()
            .enumerate()
            .map(|(i, e)| (Key::of(&e, i as u64), e))
            .collect();
        warnings.sort_by(|a, b| a.0.cmp(&b.0));
        self.warnings = warnings.into_iter().map(|(_, e)| e).collect();
        cut
    }
}

/// Rank of the first pointer segment: root fields, bound values, `params`, `nodes`,
/// `outputs`.
fn section(seg: Option<&str>, bound: bool) -> u8 {
    if bound {
        return 2;
    }
    match seg {
        None => 0,
        Some("params") => 3,
        Some("nodes") => 4,
        Some("outputs") => 5,
        Some(_) => 1,
    }
}

/// A pointer cut to the shapes the text language maps to source spans: `/params/i[/f]`,
/// `/nodes/i[/f]`, `/outputs/i[/f]` with `rule/f` and `dims/j[/node]` one level deeper,
/// `/reject[/f]`. Deeper findings (a parameter reference's inner field, a line anchor's
/// `t`, a bucket edge, a source's `strategy`) point at the field that holds them.
pub(crate) fn clamp(at: &str) -> String {
    let segs: Vec<&str> = at.split('/').skip(1).collect();
    let keep = match segs.as_slice() {
        ["outputs", _, "rule", ..] => 4,
        ["outputs", _, "dims", _, "node", ..] => 5,
        ["outputs", _, "dims", ..] => 4,
        ["params" | "nodes" | "outputs", ..] => 3,
        ["reject", ..] => 2,
        _ => segs.len(),
    };
    segs.iter()
        .take(keep)
        .fold(String::new(), |acc, s| format!("{acc}/{s}"))
}

/// `base` + `/` + `key`, escaped as a JSON pointer segment (RFC 6901).
pub(crate) fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// `base` + `/` + `i`.
pub(crate) fn join_i(base: &str, i: usize) -> String {
    format!("{base}/{i}")
}

#[cfg(test)]
mod tests {
    use super::clamp;

    #[test]
    fn pointer_shapes() {
        for (a, b) in [
            ("", ""),
            ("/version", "/version"),
            ("/nodes/3/period/param", "/nodes/3/period"),
            ("/nodes/0/b/t", "/nodes/0/b"),
            ("/nodes/1/edges/2", "/nodes/1/edges"),
            ("/outputs/0/rule/max_bars", "/outputs/0/rule/max_bars"),
            ("/outputs/0/source/strategy", "/outputs/0/source"),
            ("/outputs/0/dims/1/name", "/outputs/0/dims/1"),
            ("/outputs/0/dims/1/node", "/outputs/0/dims/1/node"),
            ("/params/2/default", "/params/2/default"),
            ("/reject/code", "/reject/code"),
        ] {
            assert_eq!(clamp(a), b);
        }
    }
}
