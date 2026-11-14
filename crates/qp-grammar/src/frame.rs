//! The compile frame: the view's first bar series, its slots (including slots after the
//! last bar for point series), the visible slot range, LOD bucketing, and data columns.

use std::borrow::Cow;
use std::collections::BTreeMap;

use qp_schema::{Bar, Fact};

use crate::error::{ErrorKind, GrammarError};
use crate::provider::{DataProvider, SeriesKind};
use crate::theme::Theme;

/// Bar fields of a bar series, in the order of [`Column::Bars`]' index.
pub(crate) const BAR_FIELDS: [&str; 5] = ["o", "h", "l", "c", "v"];

/// A slot-indexed data column.
pub(crate) enum Column<'f> {
    /// A field of a bar series (index into [`BAR_FIELDS`]).
    Bars(Cow<'f, [Bar]>, usize),
    /// Values aligned to slots.
    Values(Cow<'f, [Option<f64>]>),
    /// Values by slot built from a point series.
    Dense(Vec<Option<f64>>),
}

impl Column<'_> {
    /// Value at `slot`; non-finite values read as `None`.
    pub(crate) fn get(&self, slot: u64) -> Option<f64> {
        let i = usize::try_from(slot).ok()?;
        let v = match self {
            Column::Bars(b, f) => {
                let b = b.get(i)?;
                match f {
                    0 => b.o,
                    1 => b.h,
                    2 => b.l,
                    3 => b.c,
                    _ => b.v,
                }
            }
            Column::Values(v) => (*v.get(i)?)?,
            Column::Dense(v) => (*v.get(i)?)?,
        };
        v.is_finite().then_some(v)
    }
}

/// One data row of a fact or finding layer.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub slot: u64,
    pub nums: BTreeMap<String, f64>,
    pub texts: BTreeMap<String, String>,
}

/// Number of LOD buckets for `count` slots of `k` (the remainder joins the last one).
pub(crate) fn bucket_count(count: u64, k: u64) -> u64 {
    if k <= 1 {
        count
    } else {
        (count / k).max(1)
    }
}

pub(crate) struct Frame<'a> {
    pub p: &'a dyn DataProvider,
    pub theme: Theme,
    pub bar_series: String,
    pub bars: Cow<'a, [Bar]>,
    /// Times of the slots after the last bar (`n_bars + k` has `ext_times[k]`).
    pub ext_times: Vec<i64>,
    /// First visible slot.
    pub from: u64,
    /// Last visible slot (inclusive).
    pub to: u64,
    /// Bars per LOD bucket (1 = no aggregation).
    pub k: u64,
}

impl<'a> Frame<'a> {
    pub(crate) fn n_bars(&self) -> u64 {
        self.bars.len() as u64
    }

    pub(crate) fn total_slots(&self) -> u64 {
        self.n_bars() + self.ext_times.len() as u64
    }

    /// Time of a slot (bar time, or the time of a future point).
    pub(crate) fn slot_time(&self, slot: u64) -> Option<i64> {
        let i = usize::try_from(slot).ok()?;
        if let Some(b) = self.bars.get(i) {
            return Some(b.t);
        }
        self.ext_times.get(i - self.bars.len()).copied()
    }

    /// Slot of the last slot whose time is `<= t`, or `None` before the first bar.
    pub(crate) fn time_slot(&self, t: f64) -> Option<u64> {
        #[allow(clippy::cast_precision_loss)]
        let le = |x: i64| (x as f64) <= t;
        let nb = self.bars.partition_point(|b| le(b.t));
        let ne = self.ext_times.partition_point(|x| le(*x));
        if ne > 0 {
            Some((self.bars.len() + ne - 1) as u64)
        } else if nb > 0 {
            Some((nb - 1) as u64)
        } else {
            None
        }
    }

    /// Visible LOD buckets: `(first slot, slot count)`. Buckets hold `k` slots; the
    /// remainder joins the last bucket, so every bucket is at least one pixel wide and
    /// its centre column lies inside it.
    pub(crate) fn buckets(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        let k = self.k.max(1);
        let end = self.to + 1;
        let n = bucket_count(end - self.from, k);
        (0..n).map(move |b| {
            let s = self.from + b * k;
            (s, if b + 1 == n { end - s } else { k })
        })
    }

    pub(crate) fn kind(&self, series: &str) -> Option<SeriesKind> {
        self.p.series_kind(series)
    }

    /// Field names of a series.
    pub(crate) fn fields(&self, series: &str) -> Vec<String> {
        match self.kind(series) {
            Some(SeriesKind::Bars) => BAR_FIELDS.iter().map(|s| (*s).to_owned()).collect(),
            Some(SeriesKind::Points) => vec!["v".to_owned()],
            Some(SeriesKind::Values) => self.p.fields(series),
            None => Vec::new(),
        }
    }

    /// Main field of a series.
    pub(crate) fn main_field(&self, series: &str) -> Option<String> {
        match self.kind(series)? {
            SeriesKind::Bars => Some("c".to_owned()),
            SeriesKind::Points => Some("v".to_owned()),
            SeriesKind::Values => self
                .p
                .main_field(series)
                .or_else(|| self.p.fields(series).into_iter().next()),
        }
    }

    /// Default plot fields of a series (`line` without `y`).
    pub(crate) fn plot_fields(&self, series: &str) -> Vec<String> {
        match self.kind(series) {
            Some(SeriesKind::Values) => {
                let p = self.p.plot_fields(series);
                if p.is_empty() {
                    self.main_field(series).into_iter().collect()
                } else {
                    p
                }
            }
            Some(_) => self.main_field(series).into_iter().collect(),
            None => Vec::new(),
        }
    }

    /// The column `field` of `series`. `layer`/`ptr` locate errors.
    pub(crate) fn column(
        &self,
        series: &str,
        field: &str,
        layer: &str,
        ptr: &str,
    ) -> Result<Column<'_>, GrammarError> {
        let not_found =
            |what: String| GrammarError::in_layer(ErrorKind::NotFound, layer, ptr, what);
        let kind = self
            .kind(series)
            .ok_or_else(|| not_found(format!("series `{series}` does not exist")))?;
        let missing = || {
            not_found(format!(
                "field `{field}` not found in series `{series}` (fields: {})",
                self.fields(series).join(", ")
            ))
        };
        match kind {
            SeriesKind::Bars => {
                let i = BAR_FIELDS
                    .iter()
                    .position(|f| *f == field)
                    .ok_or_else(missing)?;
                let bars = if series == self.bar_series {
                    Cow::Borrowed(&self.bars[..])
                } else {
                    self.p
                        .bars(series)
                        .ok_or_else(|| not_found(format!("series `{series}` has no bars")))?
                };
                Ok(Column::Bars(bars, i))
            }
            SeriesKind::Values => {
                if !self.p.fields(series).iter().any(|f| f == field) {
                    return Err(missing());
                }
                self.p
                    .values(series, field)
                    .map(Column::Values)
                    .ok_or_else(missing)
            }
            SeriesKind::Points => {
                if field != "v" {
                    return Err(missing());
                }
                let pts = self
                    .p
                    .points(series)
                    .ok_or_else(|| not_found(format!("series `{series}` has no points")))?;
                let total = usize::try_from(self.total_slots()).unwrap_or(0);
                let mut dense = vec![None; total];
                let last_t = self.bars.last().map(|b| b.t);
                let mut k = 0usize;
                for pt in pts.iter() {
                    let slot = match last_t {
                        Some(lt) if pt.t > lt => {
                            k += 1;
                            Some(self.bars.len() + k - 1)
                        }
                        #[allow(clippy::cast_precision_loss)]
                        _ => self
                            .time_slot(pt.t as f64)
                            .and_then(|s| usize::try_from(s).ok()),
                    };
                    if let Some(s) = slot {
                        if let Some(d) = dense.get_mut(s) {
                            *d = pt.v;
                        }
                    }
                }
                Ok(Column::Dense(dense))
            }
        }
    }

    /// One row for a fact: `bar t value text` and its evidence keys.
    pub(crate) fn fact_row(
        &self,
        fact: &Fact,
        layer: &str,
        ptr: &str,
    ) -> Result<Row, GrammarError> {
        let mut nums = BTreeMap::new();
        #[allow(clippy::cast_precision_loss)]
        {
            nums.insert("bar".to_owned(), fact.bar as f64);
            nums.insert("t".to_owned(), fact.t as f64);
        }
        if let Some(v) = self.fact_value(fact, layer, ptr)? {
            nums.insert("value".to_owned(), v);
        }
        for (k, v) in &fact.evidence {
            nums.insert(k.clone(), *v);
        }
        let mut texts = BTreeMap::new();
        texts.insert("text".to_owned(), fact.text.clone());
        Ok(Row {
            slot: fact.bar,
            nums,
            texts,
        })
    }

    /// A fact's `value`: the main output of its series at its bar.
    pub(crate) fn fact_value(
        &self,
        fact: &Fact,
        layer: &str,
        ptr: &str,
    ) -> Result<Option<f64>, GrammarError> {
        if self.kind(&fact.series).is_none() {
            return Ok(None);
        }
        let Some(main) = self.main_field(&fact.series) else {
            return Ok(None);
        };
        Ok(self.column(&fact.series, &main, layer, ptr)?.get(fact.bar))
    }
}
