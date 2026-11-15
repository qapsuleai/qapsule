//! Reader for locally fetched real bars (`fixtures/local/bars/`, written by
//! `scripts/fetch-bars.sh`; K23, `docs/DATA_ACCESS.md`). Licensed data: it stays on the
//! machine, so everything here tolerates its absence (empty results, never a panic).
//!
//! File name: `<SYMBOL>.<period>.<adj|unadj>.json`, a `Series` with inline bars. The
//! fetch script already maps Matriks times to the project convention (K14): `t` is the
//! bar open in UTC ms; a daily bar's `t` is the XIST session open (10:00 Istanbul); hourly
//! bars start at the full hour, the closing auction merged into the last bar of the day.

use std::path::{Path, PathBuf};

use qp_schema::{Bar, Series};

/// Environment variable that overrides the local bars directory.
pub const LOCAL_BARS_ENV: &str = "QAPSULE_LOCAL_BARS";

/// The local bars directory: `$QAPSULE_LOCAL_BARS` if set, else
/// `<workspace>/fixtures/local/bars`. (In a git worktree the data usually lives in the
/// main checkout; point the variable there.)
#[must_use]
pub fn local_bars_dir() -> PathBuf {
    if let Some(p) = std::env::var_os(LOCAL_BARS_ENV) {
        return PathBuf::from(p);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/local/bars")
}

/// Adjusted bars of `period` (`1day`, `1hour`, …) for every symbol in `dir`, sorted by
/// symbol. Missing directory or files: empty. Unreadable or invalid files are skipped.
#[must_use]
pub fn load_local_bars(dir: &Path, period: &str) -> Vec<(String, Vec<Bar>)> {
    load_local_bars_with(dir, period, true)
}

/// Like [`load_local_bars`], choosing adjusted (`adj`) or unadjusted (`unadj`) files.
#[must_use]
pub fn load_local_bars_with(dir: &Path, period: &str, adjusted: bool) -> Vec<(String, Vec<Bar>)> {
    let suffix = format!(".{period}.{}.json", if adjusted { "adj" } else { "unadj" });
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<Bar>)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let symbol = name.strip_suffix(&suffix)?;
            if symbol.is_empty() || symbol.contains('.') {
                return None;
            }
            let bars = read_bars(&e.path())?;
            Some((symbol.to_owned(), bars))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn read_bars(path: &Path) -> Option<Vec<Bar>> {
    let text = crate::read(path).ok()?;
    let series: Series = serde_json::from_str(&text).ok()?;
    let bars = series.bars()?.to_vec();
    qp_schema::validate_bars(&bars).ok()?;
    Some(bars)
}
