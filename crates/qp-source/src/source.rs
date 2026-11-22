//! The [`SeriesSource`] trait, its [`BarEvent`]s and error, event application and the
//! in-memory source (out of `lib.rs`).

use std::fmt;

use qp_schema::{Bar, Instrument, Series, SeriesData};

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
