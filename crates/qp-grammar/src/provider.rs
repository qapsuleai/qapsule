//! The data the compiler draws, supplied by the authority (`qp-session`).
//!
//! The grammar **refers** to data, it never carries or computes it (K23): study values
//! come from the authority's compute core. [`DataProvider`] is the whole contract; it is
//! object-safe (`&dyn DataProvider`) and read-only. [`MemProvider`] is an in-memory
//! implementation for tests and examples.

use std::borrow::Cow;
use std::collections::BTreeMap;

use qp_schema::{Bar, Fact, Finding, Point};

/// What kind of rows a series has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesKind {
    /// OHLCV bars: fields `o h l c v`, main field `c`, one row per bar slot.
    Bars,
    /// Per-bar values (a study output): named fields aligned to the bar slots of the
    /// view's first bar series, `None` during warmup.
    Values,
    /// Timed points `(t, v)`: field `v`; points after the last bar occupy the following
    /// slots (`docs/GRAMMAR.md` §3).
    Points,
}

/// Read-only access to the document's data for one compile.
///
/// Field lists, main field and default plot outputs are asked only for
/// [`SeriesKind::Values`] series; bar and point series have fixed fields.
pub trait DataProvider {
    /// Kind of the series `series`, or `None` if it does not exist.
    fn series_kind(&self, series: &str) -> Option<SeriesKind>;
    /// Bars of a bar series, ascending by time.
    fn bars(&self, series: &str) -> Option<Cow<'_, [Bar]>>;
    /// For a values series: the bar series it is computed on (its input).
    fn input_bars(&self, series: &str) -> Option<String>;
    /// Field names of a values series, e.g. `["upper","middle","lower","bandwidth"]`.
    fn fields(&self, series: &str) -> Vec<String>;
    /// Main field of a values series (the output named like the study, a chosen
    /// `source.output`, or the middle line of a band study).
    fn main_field(&self, series: &str) -> Option<String>;
    /// Default plot outputs of a values series (e.g. the three lines of a band study).
    fn plot_fields(&self, series: &str) -> Vec<String>;
    /// Values of `field` of a values series, one per bar slot (`None` = warmup).
    fn values(&self, series: &str, field: &str) -> Option<Cow<'_, [Option<f64>]>>;
    /// Points of a point series, ascending by time (may extend past the last bar).
    fn points(&self, series: &str) -> Option<Cow<'_, [Point]>>;
    /// Legend label of a series, e.g. a study name with its parameters.
    fn label(&self, series: &str) -> Option<String>;
    /// Instrument symbol of a bar series (title).
    fn symbol(&self, series: &str) -> Option<String>;
    /// Bar interval of a bar series (title).
    fn interval(&self, series: &str) -> Option<String>;
    /// A fact by id.
    fn fact(&self, id: &str) -> Option<Fact>;
    /// A finding by id.
    fn finding(&self, id: &str) -> Option<Finding>;
    /// Headline of the capsule presenting `finding`, if there is one.
    fn headline(&self, finding: &str) -> Option<String>;
    /// Value of a named threshold (`<study>.<name>`), e.g. the overbought level → 70.
    fn threshold(&self, name: &str) -> Option<f64>;
}

#[derive(Debug, Clone, Default)]
struct BarData {
    bars: Vec<Bar>,
    symbol: Option<String>,
    interval: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ValuesData {
    input: Option<String>,
    fields: Vec<String>,
    values: BTreeMap<String, Vec<Option<f64>>>,
    main: Option<String>,
    plot: Vec<String>,
}

/// In-memory [`DataProvider`] (tests, examples, benchmarks).
#[derive(Debug, Clone)]
pub struct MemProvider {
    bars: BTreeMap<String, BarData>,
    values: BTreeMap<String, ValuesData>,
    points: BTreeMap<String, Vec<Point>>,
    labels: BTreeMap<String, String>,
    facts: BTreeMap<String, Fact>,
    findings: BTreeMap<String, Finding>,
    headlines: BTreeMap<String, String>,
    thresholds: BTreeMap<String, f64>,
}

impl Default for MemProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MemProvider {
    /// Empty provider (no series, no named thresholds: the threshold list is the
    /// engine's, see `with_threshold`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            bars: BTreeMap::new(),
            values: BTreeMap::new(),
            points: BTreeMap::new(),
            labels: BTreeMap::new(),
            facts: BTreeMap::new(),
            findings: BTreeMap::new(),
            headlines: BTreeMap::new(),
            thresholds: BTreeMap::new(),
        }
    }

    /// Adds a bar series.
    #[must_use]
    pub fn with_bars(
        mut self,
        id: &str,
        bars: Vec<Bar>,
        symbol: Option<&str>,
        interval: Option<&str>,
    ) -> Self {
        self.bars.insert(
            id.to_owned(),
            BarData {
                bars,
                symbol: symbol.map(str::to_owned),
                interval: interval.map(str::to_owned),
            },
        );
        self
    }

    /// Adds a values series computed on bar series `input`. `fields` are in output order;
    /// `main` is the main field, `plot` the default plot outputs.
    #[must_use]
    pub fn with_values(
        mut self,
        id: &str,
        input: &str,
        fields: Vec<(&str, Vec<Option<f64>>)>,
        main: &str,
        plot: &[&str],
    ) -> Self {
        let names = fields.iter().map(|(n, _)| (*n).to_owned()).collect();
        let values = fields.into_iter().map(|(n, v)| (n.to_owned(), v)).collect();
        self.values.insert(
            id.to_owned(),
            ValuesData {
                input: Some(input.to_owned()),
                fields: names,
                values,
                main: Some(main.to_owned()),
                plot: plot.iter().map(|s| (*s).to_owned()).collect(),
            },
        );
        self
    }

    /// Adds a point series.
    #[must_use]
    pub fn with_points(mut self, id: &str, points: Vec<Point>) -> Self {
        self.points.insert(id.to_owned(), points);
        self
    }

    /// Sets the legend label of a series.
    #[must_use]
    pub fn with_label(mut self, id: &str, label: &str) -> Self {
        self.labels.insert(id.to_owned(), label.to_owned());
        self
    }

    /// Adds a fact.
    #[must_use]
    pub fn with_fact(mut self, fact: Fact) -> Self {
        self.facts.insert(fact.id.clone(), fact);
        self
    }

    /// Adds a finding.
    #[must_use]
    pub fn with_finding(mut self, finding: Finding) -> Self {
        self.findings.insert(finding.id.clone(), finding);
        self
    }

    /// Sets the capsule headline of a finding.
    #[must_use]
    pub fn with_headline(mut self, finding: &str, headline: &str) -> Self {
        self.headlines
            .insert(finding.to_owned(), headline.to_owned());
        self
    }

    /// Sets a named threshold.
    #[must_use]
    pub fn with_threshold(mut self, name: &str, value: f64) -> Self {
        self.thresholds.insert(name.to_owned(), value);
        self
    }
}

impl DataProvider for MemProvider {
    fn series_kind(&self, series: &str) -> Option<SeriesKind> {
        if self.bars.contains_key(series) {
            Some(SeriesKind::Bars)
        } else if self.values.contains_key(series) {
            Some(SeriesKind::Values)
        } else if self.points.contains_key(series) {
            Some(SeriesKind::Points)
        } else {
            None
        }
    }

    fn bars(&self, series: &str) -> Option<Cow<'_, [Bar]>> {
        self.bars
            .get(series)
            .map(|b| Cow::Borrowed(b.bars.as_slice()))
    }

    fn input_bars(&self, series: &str) -> Option<String> {
        self.values.get(series).and_then(|v| v.input.clone())
    }

    fn fields(&self, series: &str) -> Vec<String> {
        self.values
            .get(series)
            .map(|v| v.fields.clone())
            .unwrap_or_default()
    }

    fn main_field(&self, series: &str) -> Option<String> {
        self.values.get(series).and_then(|v| v.main.clone())
    }

    fn plot_fields(&self, series: &str) -> Vec<String> {
        self.values
            .get(series)
            .map(|v| v.plot.clone())
            .unwrap_or_default()
    }

    fn values(&self, series: &str, field: &str) -> Option<Cow<'_, [Option<f64>]>> {
        self.values
            .get(series)
            .and_then(|v| v.values.get(field))
            .map(|v| Cow::Borrowed(v.as_slice()))
    }

    fn points(&self, series: &str) -> Option<Cow<'_, [Point]>> {
        self.points.get(series).map(|p| Cow::Borrowed(p.as_slice()))
    }

    fn label(&self, series: &str) -> Option<String> {
        self.labels.get(series).cloned()
    }

    fn symbol(&self, series: &str) -> Option<String> {
        self.bars.get(series).and_then(|b| b.symbol.clone())
    }

    fn interval(&self, series: &str) -> Option<String> {
        self.bars.get(series).and_then(|b| b.interval.clone())
    }

    fn fact(&self, id: &str) -> Option<Fact> {
        self.facts.get(id).cloned()
    }

    fn finding(&self, id: &str) -> Option<Finding> {
        self.findings.get(id).cloned()
    }

    fn headline(&self, finding: &str) -> Option<String> {
        self.headlines.get(finding).cloned()
    }

    fn threshold(&self, name: &str) -> Option<f64> {
        self.thresholds.get(name).copied()
    }
}
