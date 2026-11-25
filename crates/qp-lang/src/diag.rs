//! Collecting program errors and warnings (PR §11.1): the checker never stops at the
//! first finding; errors are kept up to `program.check.max_errors` (the rest only
//! counted, so a hostile program cannot make the answer grow), then sorted by position
//! (root, `params`, `nodes`, `outputs`; indices numerically) and reason.

use std::cmp::Ordering;

use qp_schema::protocol::ProgramError;

use crate::message::{Lang, Msg};
use crate::reason::Reason;

/// Errors and warnings of one check.
#[derive(Debug)]
pub(crate) struct Diags {
    pub(crate) errors: Vec<ProgramError>,
    pub(crate) warnings: Vec<ProgramError>,
    pub(crate) lang: Lang,
    /// Errors kept at most.
    cap: usize,
    /// Errors found beyond `cap` (counted, not kept).
    dropped: usize,
    /// Where a dropped error is written (and forgotten).
    sink: ProgramError,
}

/// Longest message kept, in characters (a message quotes names from the input).
const MAX_MESSAGE: usize = 512;

fn bounded(mut m: String) -> String {
    if let Some((i, _)) = m.char_indices().nth(MAX_MESSAGE) {
        m.truncate(i);
        m.push('…');
    }
    m
}

/// Adds detail to the finding just pushed.
pub(crate) struct Found<'a> {
    e: &'a mut ProgramError,
    lang: Lang,
}

impl Found<'_> {
    /// Accepted values or forms.
    pub(crate) fn expected<S: ToString>(self, v: impl IntoIterator<Item = S>) -> Self {
        self.e.expected = v.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Close names (at most 5).
    pub(crate) fn dym(self, v: Vec<String>) -> Self {
        self.e.did_you_mean = v.into_iter().take(5).collect();
        self
    }

    /// How to fix it.
    pub(crate) fn hint(self, h: &Msg) -> Self {
        self.e.hint = Some(h.render(self.lang));
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
            dropped: 0,
            sink: Reason::Type.error(None, String::new()),
        }
    }

    /// Pushes a finding at JSON pointer `at` (`None`: outside the program, e.g. a bound
    /// value) and returns it for detail.
    pub(crate) fn push(&mut self, reason: Reason, at: Option<&str>, msg: &Msg) -> Found<'_> {
        let e = reason.error(at.map(str::to_owned), bounded(msg.render(self.lang)));
        let lang = self.lang;
        if !reason.is_warning() && self.errors.len() >= self.cap {
            self.dropped += 1;
            self.sink = e;
            return Found {
                e: &mut self.sink,
                lang,
            };
        }
        let list = if reason.is_warning() {
            &mut self.warnings
        } else {
            &mut self.errors
        };
        list.push(e);
        let last = list.len() - 1;
        Found {
            e: &mut list[last],
            lang,
        }
    }

    /// Shorthand for an error at a pointer.
    pub(crate) fn at(&mut self, reason: Reason, at: &str, msg: &Msg) -> Found<'_> {
        self.push(reason, Some(at), msg)
    }

    /// Error count so far (dropped ones included).
    pub(crate) fn count(&self) -> usize {
        self.errors.len() + self.dropped
    }

    /// Clamps pointers to their shapes, sorts both lists, cuts errors at `max`; returns
    /// whether errors were cut.
    pub(crate) fn finish(&mut self, max: usize) -> bool {
        for e in self.errors.iter_mut().chain(self.warnings.iter_mut()) {
            if let Some(at) = &e.at {
                e.at = Some(clamp(at));
            }
        }
        self.errors.sort_by(order);
        self.warnings.sort_by(order);
        let cut = self.errors.len() > max || self.dropped > 0;
        self.errors.truncate(max);
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

fn cmp_segment(a: &str, b: &str) -> Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

fn order(a: &ProgramError, b: &ProgramError) -> Ordering {
    let segs = |e: &ProgramError| -> Vec<String> {
        e.at.as_deref()
            .unwrap_or("")
            .split('/')
            .skip(1)
            .map(str::to_owned)
            .collect()
    };
    let (sa, sb) = (segs(a), segs(b));
    let ka = section(sa.first().map(String::as_str), a.at.is_none());
    let kb = section(sb.first().map(String::as_str), b.at.is_none());
    ka.cmp(&kb)
        .then_with(|| {
            for (x, y) in sa.iter().zip(&sb) {
                let o = cmp_segment(x, y);
                if o != Ordering::Equal {
                    return o;
                }
            }
            sa.len().cmp(&sb.len())
        })
        .then_with(|| a.reason.cmp(&b.reason))
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
