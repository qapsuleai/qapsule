"""`moved.py --self-test`: synthetic before/after snippets; a pure move passes and each
check (use, impl header, item-level macro, cfg) fires on its own change."""

BEFORE = '''use std::fmt;
use crate::model::Bar;

/// A price.
#[derive(Clone)]
pub struct Price(f64);

#[cfg(feature = "wide")]
pub fn wide() -> u8 { 1 }

impl<T: Clone> Holder<T> {
    pub fn get(&self) -> T { self.0.clone() }
    pub fn put(&mut self, t: T) { self.0 = t; }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

#[cfg(feature = "net")]
mod net;
#[cfg(test)]
mod tests;

register_look! { name: "tavek", weight: 3 }
thread_local!(static N: u8 = 0);

pub fn uses_bar(b: Bar) -> f64 { b.close }
'''

# The pure move: the same items split over two files, imports re-pathed, the impl split in
# two blocks, a new `#[cfg(test)] mod tests;` in the second file, rustfmt re-wrapping.
MOVED_A = '''use core::fmt;
use crate::bars::Bar;

/// A price.
#[derive(Clone)]
pub(crate) struct Price(f64);

impl<T: Clone> Holder<T> {
    pub fn get(&self) -> T {
        self.0.clone()
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

#[cfg(feature = "net")]
mod net;
#[cfg(test)]
mod tests;

pub fn uses_bar(b: Bar) -> f64 { b.close }
'''
MOVED_B = '''#[cfg(feature = "wide")]
pub fn wide() -> u8 { 1 }

impl<T: Clone> Holder<T> {
    pub fn put(&mut self, t: T) { self.0 = t; }
}

register_look! { name: "tavek", weight: 3 }
thread_local!(static N: u8 = 0);
#[cfg(test)]
mod tests;
'''


def _variant(old, new):
    return BEFORE.replace(old, new, 1)


CASES = [
    # (label, after-text, expected result kind or None for "passes")
    ("use: a glob import added is a FLAG",
     _variant("use std::fmt;", "use std::fmt;\nuse crate::model::*;"), "flag"),
    ("use: an anonymous trait import (`as _`) removed is a FLAG",
     BEFORE, "flag"),  # BEFORE side gets the import below
    ("use: a #[cfg]-gated import changed is a FLAG",
     _variant("use crate::model::Bar;", "#[cfg(feature = \"x\")]\nuse crate::model::Bar;"), "flag"),
    ("impl header: a bound changed is CHANGED",
     _variant("impl<T: Clone> Holder<T>", "impl<T: Clone + Send> Holder<T>"), "impl"),
    ("impl header: the trait changed is CHANGED",
     _variant("impl fmt::Display for Price", "impl fmt::Debug for Price"), "impl"),
    ("impl header: a #[cfg] added to an impl is CHANGED",
     _variant("impl<T: Clone> Holder<T>", "#[cfg(unix)]\nimpl<T: Clone> Holder<T>"), "impl"),
    ("macro: an item-level macro's arguments changed is CHANGED",
     _variant("weight: 3", "weight: 4"), "macro!"),
    ("macro: an item-level `foo!(…);` dropped is CHANGED",
     _variant("thread_local!(static N: u8 = 0);\n", ""), "macro!"),
    ("cfg: a fn's #[cfg] changed is CHANGED",
     _variant('#[cfg(feature = "wide")]', '#[cfg(feature = "narrow")]'), "fn"),
    ("cfg: a fn's #[cfg] dropped is CHANGED",
     _variant('#[cfg(feature = "wide")]\n', ""), "fn"),
    ("cfg: a mod's #[cfg] dropped is CHANGED",
     _variant('#[cfg(feature = "net")]\nmod net;', "mod net;"), "mod"),
    ("path: a re-pathed import and impl trait path pass as PATH-ONLY",
     _variant("impl fmt::Display", "impl core::fmt::Display"), None),
]


def _kinds(res):
    out = {"flag"} if res.flags else set()
    return out | {w[1] for _, w, _ in res.changed}


def run(compare):
    ok = True
    res = compare([("a.rs", BEFORE, "a.rs", MOVED_A), (None, "", "b.rs", MOVED_B)])
    good = not res.failed and res.moved > 0 and res.uses
    print(f"moved self-test: {'pure move (split files, re-pathed use, split impl) passes':66} "
          f"{'ok' if good else 'FAILED'}")
    if not good:
        from moved import report
        report(res, True)
    ok &= bool(good)
    for label, after, want in CASES:
        before = BEFORE
        if "as _" in label:
            before = BEFORE.replace("use std::fmt;", "use std::fmt;\nuse std::io::Write as _;", 1)
        res = compare([("a.rs", before, "a.rs", after)])
        kinds = _kinds(res)
        hit = (not res.failed) if want is None else (want in kinds)
        if want == "flag":  # a FLAG is for the reviewer; it fails only with --strict-use
            hit = hit and not res.failed and compare([("a.rs", before, "a.rs", after)], True).failed
        print(f"moved self-test: {label:66} {'ok' if hit else 'MISSED'}")
        if not hit:
            from moved import report
            report(res, True)
        ok &= hit
    print("moved self-test:", "PASS" if ok else "FAIL")
    return ok
