//! Crate-wide macros (declared first in `lib.rs` so every module sees them).

/// The reading document of a session (`Session::rd`, the document with ready bound bars;
/// `plane.rs`), as a field-level borrow so that `&mut self.cache` can be taken alongside.
macro_rules! rd {
    ($s:expr) => {
        $s.rd.as_ref().unwrap_or(&$s.doc)
    };
}
