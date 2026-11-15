//! The authority as the grammar compiler's data source (`docs/GRAMMAR.md` §6, §14):
//! [`DocProvider`] implements [`qp_grammar::DataProvider`] over the document and the
//! values cache. Study values come from `qp-compute` through the cache (K23); the
//! provider itself is read-only, so the cache is warmed before a compile
//! ([`warm`]).

use std::borrow::Cow;

use qp_grammar::{DataProvider, SeriesKind};
use qp_schema::{Bar, Document, Fact, Finding, Point, SeriesData, SeriesSource};

use crate::canon::{series_fields, series_main, threshold};
use crate::summary::{drawing_outputs, series_label};
use crate::values::{bar_series, find_series, values, Cache};

/// Read-only view of a document and its computed study values, for one compile or one
/// presenter call. Obtain it with [`crate::Session::provider`].
pub struct DocProvider<'a> {
    doc: &'a Document,
    cache: &'a Cache,
}

impl<'a> DocProvider<'a> {
    pub(crate) fn new(doc: &'a Document, cache: &'a Cache) -> Self {
        Self { doc, cache }
    }
}

/// Computes (and caches) the values of every study series of the document, so that a
/// [`DocProvider`] built afterwards can lend them out. Studies that cannot be computed
/// are skipped (the compiler then reports the missing values with the layer's path).
pub(crate) fn warm(doc: &Document, cache: &mut Cache) {
    for s in &doc.series {
        if matches!(s.source, SeriesSource::Study { .. }) && s.data.is_none() {
            let _ = values(doc, cache, &s.id);
        }
    }
}

impl DataProvider for DocProvider<'_> {
    fn series_kind(&self, series: &str) -> Option<SeriesKind> {
        let s = find_series(self.doc, series)?;
        match (&s.data, &s.source) {
            (Some(SeriesData::Bars { .. }), _) => Some(SeriesKind::Bars),
            (Some(SeriesData::Points { .. }), _) => Some(SeriesKind::Points),
            (None, SeriesSource::Study { .. }) => Some(SeriesKind::Values),
            _ => None,
        }
    }

    fn bars(&self, series: &str) -> Option<Cow<'_, [Bar]>> {
        find_series(self.doc, series)?.bars().map(Cow::Borrowed)
    }

    fn input_bars(&self, series: &str) -> Option<String> {
        let s = find_series(self.doc, series)?;
        if !matches!(s.source, SeriesSource::Study { .. }) {
            return None;
        }
        bar_series(self.doc, series).map(|b| b.id.clone())
    }

    fn fields(&self, series: &str) -> Vec<String> {
        find_series(self.doc, series).map_or_else(Vec::new, series_fields)
    }

    fn main_field(&self, series: &str) -> Option<String> {
        find_series(self.doc, series).map(series_main)
    }

    fn plot_fields(&self, series: &str) -> Vec<String> {
        find_series(self.doc, series).map_or_else(Vec::new, drawing_outputs)
    }

    fn values(&self, series: &str, field: &str) -> Option<Cow<'_, [Option<f64>]>> {
        self.cache
            .get(series)?
            .outputs
            .get(field)
            .map(|v| Cow::Borrowed(v.as_slice()))
    }

    fn points(&self, series: &str) -> Option<Cow<'_, [Point]>> {
        match &find_series(self.doc, series)?.data {
            Some(SeriesData::Points { points }) => Some(Cow::Borrowed(points.as_slice())),
            _ => None,
        }
    }

    fn label(&self, series: &str) -> Option<String> {
        find_series(self.doc, series).map(series_label)
    }

    fn symbol(&self, series: &str) -> Option<String> {
        find_series(self.doc, series)?.symbol().map(str::to_owned)
    }

    fn interval(&self, series: &str) -> Option<String> {
        find_series(self.doc, series)?.interval.clone()
    }

    fn fact(&self, id: &str) -> Option<Cow<'_, Fact>> {
        self.doc
            .facts
            .iter()
            .find(|f| f.id == id)
            .map(Cow::Borrowed)
    }

    fn finding(&self, id: &str) -> Option<Cow<'_, Finding>> {
        self.doc
            .findings
            .iter()
            .find(|f| f.id == id)
            .map(Cow::Borrowed)
    }

    fn headline(&self, finding: &str) -> Option<String> {
        self.doc
            .capsules
            .iter()
            .find(|c| c.finding == finding)
            .map(|c| c.headline.clone())
    }

    fn threshold(&self, name: &str) -> Option<f64> {
        threshold(name)
    }
}
