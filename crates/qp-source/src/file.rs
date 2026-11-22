//! File sources and document bindings: [`FileSource`], [`resolve_binding_path`],
//! [`open_binding`] (out of `lib.rs`).

use std::path::{Path, PathBuf};

use qp_schema::Series;

use crate::{BarEvent, MemorySource, SeriesMeta, SeriesSource, SourceError};

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

pub(crate) fn read(path: &Path) -> Result<String, SourceError> {
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
