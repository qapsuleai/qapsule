//! The authority as the grammar compiler's data source (`docs/GRAMMAR.md` §6, §14):
//! [`DocProvider`] implements [`qp_grammar::DataProvider`] over the document and the
//! values cache. Study values come from `qp-compute` through the cache (K23); the
//! provider itself is read-only, so the cache is warmed before a compile
//! ([`warm`]).

use std::borrow::Cow;

use qp_grammar::{DataProvider, SeriesKind};
use qp_schema::capsule::GlyphSpec;
use qp_schema::{Bar, Capsule, Document, Fact, Finding, Point, SeriesData, SeriesSource};

use crate::canon::{series_fields, series_main, threshold};
use crate::summary::{drawing_outputs, series_label};
use crate::values::{bar_series, find_series, values, Cache};

/// Read-only view of a document and its computed study values, for one compile or one
/// presenter call. Obtain it with [`crate::Session::provider`].
pub struct DocProvider<'a> {
    doc: &'a Document,
    cache: &'a Cache,
    /// (R3) Finding id → the sentence written for this reader (language, entitlement),
    /// used instead of the stored capsule headline.
    headlines: Option<&'a std::collections::BTreeMap<String, String>>,
    /// The data plane during a command (§18): a bound bar series' bars come from the
    /// reading document when the scratch document `doc` has none ([`crate::plane::bars_for`]).
    reader: Option<crate::plane::Reader<'a>>,
    /// (R4/H-FP) The session and reader language the headline pieces are written for
    /// (inside [`crate::Session::as_reader`]: with the reader's projection).
    parts: Option<(&'a crate::Session, qp_schema::protocol::Language)>,
    /// (R5/m17-int) The voice the reader's connection chose (with [`Self::parts`]'s
    /// session and language); `None`: `r4`.
    voice: Option<&'a str>,
}

impl<'a> DocProvider<'a> {
    pub(crate) fn new(doc: &'a Document, cache: &'a Cache) -> Self {
        Self {
            doc,
            cache,
            headlines: None,
            reader: None,
            parts: None,
            voice: None,
        }
    }

    /// The same provider reading bound bars through the data plane's `reader` (command
    /// expansion: `doc` is the command's scratch document, without bound bars).
    pub(crate) fn with_reader(mut self, reader: Option<crate::plane::Reader<'a>>) -> Self {
        self.reader = reader;
        self
    }

    /// The same provider speaking with `headlines` (finding → sentence for the reader).
    pub(crate) fn with_headlines(
        mut self,
        headlines: &'a std::collections::BTreeMap<String, String>,
    ) -> Self {
        self.headlines = Some(headlines);
        self
    }

    /// (R4/H-FP) The same provider giving the headline pieces of session `s`'s reader in
    /// `lang` ([`crate::Session::reader_parts`]; the reader's projection inside
    /// [`crate::Session::as_reader`]), alongside the headlines of that reader.
    pub(crate) fn with_parts(
        mut self,
        s: &'a crate::Session,
        lang: qp_schema::protocol::Language,
    ) -> Self {
        self.parts = Some((s, lang));
        self
    }

    /// (R5/m17-int) The same provider speaking in voice `voice` (with the session and
    /// language of [`Self::with_parts`]); `None`: `r4`.
    pub(crate) fn with_voice(mut self, voice: Option<&'a str>) -> Self {
        self.voice = voice;
        self
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
            (None, SeriesSource::Bind { .. }) if self.reader.is_some() => Some(SeriesKind::Bars),
            _ => None,
        }
    }

    fn bars(&self, series: &str) -> Option<Cow<'_, [Bar]>> {
        let s = find_series(self.doc, series)?;
        match (s.bars(), &s.source, &self.reader) {
            (Some(b), _, _) => Some(Cow::Borrowed(b)),
            (None, SeriesSource::Bind { .. }, Some(r)) => {
                crate::plane::bars_for(self.doc, Some(r), series).map(|(_, b)| Cow::Borrowed(b))
            }
            _ => None,
        }
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
        if let Some(h) = self.headlines.and_then(|m| m.get(finding)) {
            return Some(h.clone());
        }
        self.doc
            .capsules
            .iter()
            .find(|c| c.finding == finding)
            .map(|c| c.headline.clone())
    }

    fn threshold(&self, name: &str) -> Option<f64> {
        threshold(name)
    }

    fn capsule(&self, capsule_or_finding: &str) -> Option<Cow<'_, Capsule>> {
        let caps = &self.doc.capsules;
        caps.iter()
            .find(|c| c.id == capsule_or_finding)
            .or_else(|| caps.iter().find(|c| c.finding == capsule_or_finding))
            .map(Cow::Borrowed)
    }

    /// R3 (`docs/GRAMMAR.md` §18, `docs/PROTOCOL.md` §20): the glyph of a capsule with a
    /// story, by the capsule engine's own rule (`qp_capsule::glyph_spec`, confidence
    /// shown from [`qp_grammar::depth::GLYPH_MIN_LEDGER_N`] closed rows on) at the last
    /// bar of the story's bar series.
    ///
    /// (R4, K42/A28) With the story's validity at that bar (from its facts in the
    /// document and the bars, the rule the sentence uses), so a capsule that no longer
    /// holds looks so in its glyph too.
    fn glyph(&self, capsule_or_finding: &str) -> Option<Cow<'_, GlyphSpec>> {
        let c = self.capsule(capsule_or_finding)?;
        let story = c.story.as_ref()?;
        let bars = self.bars(&story.series)?;
        let now = (bars.len() as u64).saturating_sub(1);
        let facts: Vec<&qp_schema::Fact> = crate::capsule::story_fact_ids(story)
            .iter()
            .filter_map(|i| self.doc.facts.iter().find(|f| &f.id == i))
            .collect();
        let validity = crate::capsule::story_validity(
            story,
            &facts,
            &bars,
            now,
            &mut qp_capsule::sentence::StudyMemo::default(),
        );
        Some(Cow::Owned(qp_capsule::glyph_spec_at(
            story,
            now,
            qp_grammar::depth::GLYPH_MIN_LEDGER_N,
            validity,
        )))
    }

    /// (R4/H-FP) The reader's headline pieces (see [`DocProvider::with_parts`]); `None`
    /// without them.
    fn headline_parts(&self, finding: &str) -> Option<qp_grammar::HeadlineParts> {
        let (s, lang) = self.parts?;
        let c = s.doc.capsules.iter().find(|c| c.finding == finding)?;
        s.reader_parts(
            &s.doc,
            c,
            s.proj.as_deref(),
            lang,
            &mut qp_capsule::sentence::StudyMemo::default(),
        )
    }

    /// (R5/M20) The reader's language (see [`DocProvider::with_parts`]).
    fn reader_lang(&self) -> Option<qp_schema::protocol::Language> {
        self.parts.map(|(_, l)| l)
    }

    /// (R5/m17-int) The reader's voice surfaces (see [`DocProvider::with_voice`]).
    fn voice(&self, finding: &str) -> Option<qp_schema::protocol::CapsuleVoice> {
        let (s, lang) = self.parts?;
        let v = self.voice?;
        let c = s.doc.capsules.iter().find(|c| c.finding == finding)?;
        s.reader_voice(
            &s.doc,
            c,
            s.proj.as_deref(),
            lang,
            v,
            &mut qp_capsule::sentence::StudyMemo::default(),
        )
    }
}
