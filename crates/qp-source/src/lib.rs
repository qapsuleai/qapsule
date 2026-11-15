//! Series sources ("Seri kaynağı", `SeriesSource` in `docs/CONCEPTS.md`).
//!
//! A source delivers a bar series as a stream of [`BarEvent`]s: a new bar, or a revision
//! of the last bar (a live bar whose price keeps changing). Consumers (the compute core,
//! the renderer, the CLI) see only this trait and never the kind of source: a file, a
//! deterministic synthetic generator today; a Matriks feed or NATS subject later (R6).
//!
//! The events map one-to-one onto `qp_compute::Study::push` / `update_last`.
//!
//! The data plane (RP2, K23; `docs/PROTOCOL.md` §18) adds field-level [`DataEvent`]s
//! (`qp_schema::data`): [`FeedStore`] applies them by the §18.4 rules and keeps an
//! incremental digest for `DataPosition`; [`resolve_window`] resolves a binding's window;
//! [`SourceProvider`] is the host-supplied provider ([`MemoryProvider`] for tests and
//! simple hosts); [`SyntheticEvents`] generates a deterministic field-level stream.
//!
//! [`DataEvent`]: qp_schema::data::DataEvent
//!
//! Note on names: `qp_schema::SeriesSource` is the *description* of a binding inside a
//! document (`{"kind": "file", "path": ...}`); this crate's [`SeriesSource`] trait is the
//! runtime side that produces the data. [`open_binding`] turns the former into the latter.

use std::fmt;
use std::path::{Path, PathBuf};

use qp_schema::{Bar, Instrument, Series, SeriesData};

mod events;
mod feed;
mod provider;
mod synthetic;
mod window;

pub use events::{
    bar_patch_diff, data_events_from_bar_events, SyntheticEvents, SyntheticEventsConfig,
};
pub use feed::{Applied, EventError, FeedStore, StaleReason, TimeBounds};
pub use provider::{
    bind, entitlement_matches, validate_binding, validate_entitlement, validate_id,
    validate_interval, Bound, MemoryProvider, Resolution, SourceProvider,
};
pub use synthetic::{synthetic_bars, synthetic_series, SyntheticSource};
pub use window::{resolve_window, ResolvedWindow};

/// One step of a bar stream.
#[derive(Debug, Clone, PartialEq)]
pub enum BarEvent {
    /// A new bar after the current last one (strictly later `t`).
    New(Bar),
    /// The last bar's values changed (same `t`).
    UpdateLast(Bar),
}

/// Identity of the series a source produces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeriesMeta {
    /// Series id.
    pub id: String,
    /// Instrument, if known.
    pub instrument: Option<Instrument>,
    /// Bar interval, e.g. `1d`.
    pub interval: Option<String>,
}

/// Any source failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError(String);

impl SourceError {
    /// An error with this message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SourceError {}

/// Where a series comes from, as seen by the core: a stream of bar events.
pub trait SeriesSource {
    /// Identity of the series.
    fn meta(&self) -> &SeriesMeta;
    /// The next event, or `None` when the source has nothing more (for finite sources:
    /// the end; live sources will block or return `None` until data arrives — R6).
    ///
    /// # Errors
    /// I/O or data errors of the underlying source.
    fn next_event(&mut self) -> Result<Option<BarEvent>, SourceError>;
}

/// Applies `event` to a bar list, checking ordering rules.
///
/// # Errors
/// A `New` bar that is not strictly later, or an `UpdateLast` with no bar or another `t`.
pub fn apply(bars: &mut Vec<Bar>, event: BarEvent) -> Result<(), SourceError> {
    match event {
        BarEvent::New(b) => {
            if bars.last().is_some_and(|l| b.t <= l.t) {
                return Err(SourceError::new(format!(
                    "new bar at t={} is not after the last bar",
                    b.t
                )));
            }
            bars.push(b);
        }
        BarEvent::UpdateLast(b) => match bars.last_mut() {
            Some(l) if l.t == b.t => *l = b,
            Some(l) => {
                return Err(SourceError::new(format!(
                    "update for t={} but the last bar is t={}",
                    b.t, l.t
                )))
            }
            None => return Err(SourceError::new("update before any bar")),
        },
    }
    Ok(())
}

/// Drains a source into its final bar list (all revisions applied).
///
/// # Errors
/// Source errors or ordering violations (see [`apply`]).
pub fn collect_bars(source: &mut dyn SeriesSource) -> Result<Vec<Bar>, SourceError> {
    let mut bars = Vec::new();
    while let Some(ev) = source.next_event()? {
        apply(&mut bars, ev)?;
    }
    Ok(bars)
}

/// Drains a source into a `Series` document with embedded bars.
///
/// # Errors
/// See [`collect_bars`].
pub fn collect_series(source: &mut dyn SeriesSource) -> Result<Series, SourceError> {
    let bars = collect_bars(source)?;
    let meta = source.meta().clone();
    Ok(Series {
        id: meta.id,
        instrument: meta.instrument,
        interval: meta.interval,
        source: qp_schema::SeriesSource::Inline {},
        data: Some(SeriesData::Bars { bars }),
    })
}

/// A finite source over bars already in memory: one `New` event per bar.
#[derive(Debug, Clone)]
pub struct MemorySource {
    meta: SeriesMeta,
    bars: Vec<Bar>,
    pos: usize,
}

impl MemorySource {
    /// A source replaying `bars` in order.
    #[must_use]
    pub fn new(meta: SeriesMeta, bars: Vec<Bar>) -> Self {
        Self { meta, bars, pos: 0 }
    }

    /// From a `Series` with embedded bars.
    ///
    /// # Errors
    /// The series has no embedded bars, or they fail `qp_schema::validate_bars`.
    pub fn from_series(series: Series) -> Result<Self, SourceError> {
        let bars = series
            .bars()
            .ok_or_else(|| {
                SourceError::new(format!("series `{}` has no embedded bars", series.id))
            })?
            .to_vec();
        qp_schema::validate_bars(&bars).map_err(SourceError::new)?;
        Ok(Self::new(
            SeriesMeta {
                id: series.id,
                instrument: series.instrument,
                interval: series.interval,
            },
            bars,
        ))
    }
}

impl SeriesSource for MemorySource {
    fn meta(&self) -> &SeriesMeta {
        &self.meta
    }
    fn next_event(&mut self) -> Result<Option<BarEvent>, SourceError> {
        let ev = self.bars.get(self.pos).cloned().map(BarEvent::New);
        self.pos += usize::from(ev.is_some());
        Ok(ev)
    }
}

/// A file source: a bar series document (`fixtures/bars` format, a `Series` with
/// embedded bars), or one series of a `Document` picked by id.
#[derive(Debug, Clone)]
pub struct FileSource {
    path: PathBuf,
    inner: MemorySource,
}

impl FileSource {
    /// Opens a `Series` document with embedded bars.
    ///
    /// # Errors
    /// Unreadable file, invalid JSON, no embedded bars, invalid bars.
    pub fn open(path: &Path) -> Result<Self, SourceError> {
        let text = read(path)?;
        let series: Series = serde_json::from_str(&text).map_err(|e| {
            SourceError::new(format!("{}: invalid series JSON: {e}", path.display()))
        })?;
        Self::with(path, series)
    }

    /// Opens series `id` of a `Document` file; it must embed its bars.
    ///
    /// # Errors
    /// Unreadable file, invalid document, unknown id, no embedded bars, invalid bars.
    pub fn open_in_document(path: &Path, id: &str) -> Result<Self, SourceError> {
        let text = read(path)?;
        let doc: qp_schema::Document = serde_json::from_str(&text).map_err(|e| {
            SourceError::new(format!("{}: invalid document JSON: {e}", path.display()))
        })?;
        let series = doc
            .series
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| SourceError::new(format!("{}: no series `{id}`", path.display())))?;
        Self::with(path, series)
    }

    fn with(path: &Path, series: Series) -> Result<Self, SourceError> {
        Ok(Self {
            path: path.to_owned(),
            inner: MemorySource::from_series(series)
                .map_err(|e| SourceError::new(format!("{}: {e}", path.display())))?,
        })
    }

    /// The file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SeriesSource for FileSource {
    fn meta(&self) -> &SeriesMeta {
        self.inner.meta()
    }
    fn next_event(&mut self) -> Result<Option<BarEvent>, SourceError> {
        self.inner.next_event()
    }
}

/// Largest series file a source reads (bytes).
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

fn read(path: &Path) -> Result<String, SourceError> {
    use std::io::Read as _;
    let err = |e: std::io::Error| SourceError::new(format!("cannot read {}: {e}", path.display()));
    let meta = std::fs::metadata(path).map_err(err)?;
    if !meta.is_file() {
        return Err(SourceError::new(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(SourceError::new(format!(
            "{} is larger than {MAX_FILE_BYTES} bytes",
            path.display()
        )));
    }
    let mut text = String::new();
    std::fs::File::open(path)
        .map_err(err)?
        .take(MAX_FILE_BYTES)
        .read_to_string(&mut text)
        .map_err(err)?;
    Ok(text)
}

/// Resolves a document's relative `file` binding under `base_dir`. Documents can be
/// written by agents, so the path must stay inside `base_dir`: absolute paths and any
/// `..`, root or prefix component are rejected, and the canonical result (symlinks
/// resolved) must still lie under the canonical `base_dir`.
///
/// # Errors
/// A path that is not a plain relative path or that escapes `base_dir`.
pub fn resolve_binding_path(base_dir: &Path, rel: &str) -> Result<PathBuf, SourceError> {
    use std::path::Component;
    let p = Path::new(rel);
    if rel.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(SourceError::new(format!(
            "file binding path `{rel}` must be a plain relative path (no `..`, no root)"
        )));
    }
    let base = base_dir
        .canonicalize()
        .map_err(|e| SourceError::new(format!("cannot resolve {}: {e}", base_dir.display())))?;
    let full = base
        .join(p)
        .canonicalize()
        .map_err(|e| SourceError::new(format!("cannot resolve `{rel}`: {e}")))?;
    if !full.starts_with(&base) {
        return Err(SourceError::new(format!(
            "file binding path `{rel}` resolves outside {}",
            base.display()
        )));
    }
    Ok(full)
}

/// Opens the runtime source for a document's series binding. Supported in R1: `inline`
/// (embedded bars) and `file` (`path` relative to `base_dir`, confined to it, see
/// [`resolve_binding_path`]). `matriks`, `nats` and
/// `study` bindings are reported as not available yet (R6 / computed by the caller).
///
/// # Errors
/// Unsupported kind or the errors of the chosen source.
pub fn open_binding(
    series: &Series,
    base_dir: &Path,
) -> Result<Box<dyn SeriesSource>, SourceError> {
    match &series.source {
        qp_schema::SeriesSource::Inline {} => {
            Ok(Box::new(MemorySource::from_series(series.clone())?))
        }
        qp_schema::SeriesSource::File { path } => {
            let full = resolve_binding_path(base_dir, path)?;
            Ok(Box::new(FileSource::open(&full)?))
        }
        other => Err(SourceError::new(format!(
            "series `{}`: source kind `{}` is not available in this build",
            series.id,
            match other {
                qp_schema::SeriesSource::Matriks { .. } => "matriks",
                qp_schema::SeriesSource::Nats { .. } => "nats",
                qp_schema::SeriesSource::Study { .. } => "study",
                _ => "?",
            }
        ))),
    }
}
