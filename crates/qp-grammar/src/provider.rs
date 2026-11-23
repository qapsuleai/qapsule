//! The data the compiler draws, supplied by the authority (`qp-session`).
//!
//! The grammar **refers** to data, it never carries or computes it (K23): study values
//! come from the authority's compute core. [`DataProvider`] is the whole contract; it is
//! object-safe (`&dyn DataProvider`) and read-only. [`MemProvider`] is an in-memory
//! implementation for tests and examples.

use std::borrow::Cow;
use std::collections::BTreeMap;

use qp_schema::capsule::{GlyphSpec, Stage, Validity};
use qp_schema::protocol::Language;
use qp_schema::{Bar, Capsule, Fact, Finding, Point};

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

/// (R4/H-FP) The pieces of a capsule's short headline for one reader, from which a face
/// design's headline template ([`crate::faces::HeadlineFn`]) builds its sentence (M8;
/// `docs/CONCEPTS.md` "Başlık yapı taşları").
///
/// Plain texts in the reader's language ([`HeadlineParts::lang`]) plus a few flags. The
/// authority fills them (and the shell's replica, for the same reader) with
/// `qp_capsule::realize::parts`, by the language and audience rule of the reader's
/// sentence: no text says "bar", "aile" or "teyit"; for a redacted reader (no data
/// entitlement, K33) **no text holds a digit** and [`HeadlineParts::ledger_n`] is
/// `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlineParts {
    /// The language every text below is in.
    pub lang: Language,
    /// Instrument label (its symbol, e.g. `SYN03`).
    pub instrument: String,
    /// What happened, a short noun phrase: "yukarı kırılım" / "upside breakout".
    pub event: String,
    /// The direction as one word: "yukarı" / "up", "aşağı" / "down", "yönsüz" / "no
    /// direction".
    pub direction_word: String,
    /// When it was born, by the bar period and the exchange calendar: "bugün", "dün",
    /// "cuma günü", "3 işlem günü önce", "2 saat önce" / "today", "3 trading days ago".
    /// Redacted: the same without digits ("birkaç gün önce").
    pub when: String,
    /// When it last moved (its last transition), the same kind of phrase.
    pub moved: String,
    /// Whether it still holds, in words: "hâlâ geçerli" / "still holds", "artık geçerli
    /// değil" / "no longer holds"; closed: how it ended ("gerçekleşti", "tutmadı",
    /// "süresi doldu", "geri çekildi" / "played out", "did not play out", "ran out of
    /// time", "withdrawn").
    pub validity_phrase: String,
    /// The ledger at birth, shortened but honest: "7'de 3 tuttu (az örnek)" / "3 of 7
    /// played out (few cases)", "karnesi yok" / "no record yet"; redacted: qualitative
    /// ("az örnek", "çoğunlukla tuttu" / "few cases", "mostly played out").
    pub ledger_short: String,
    /// The story's stage now.
    pub stage: Stage,
    /// Validity now: the same value as the capsule item's and the glyph's (K42/A28).
    pub validity: Validity,
    /// The reader gets no digits (K33): every text above is digit-free.
    pub redacted: bool,
    /// Closed ledger rows at birth (`n`); `None` for a redacted reader.
    pub ledger_n: Option<u32>,
    /// Few cases: `n < `[`crate::depth::GLYPH_MIN_LEDGER_N`] (also `n = 0`), the
    /// product's "henüz az örnekli" threshold. Given to a redacted reader too (its ledger
    /// text says "az örnek" anyway).
    pub few_cases: bool,
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
    /// A fact by id (borrowed when the provider holds it).
    fn fact(&self, id: &str) -> Option<Cow<'_, Fact>>;
    /// A finding by id (borrowed when the provider holds it).
    fn finding(&self, id: &str) -> Option<Cow<'_, Finding>>;
    /// Headline of the capsule presenting `finding`, if there is one.
    fn headline(&self, finding: &str) -> Option<String>;
    /// Value of a named threshold (`<study>.<name>`), e.g. the overbought level → 70.
    fn threshold(&self, name: &str) -> Option<f64>;
    /// The capsule `capsule_or_finding` names: a capsule id, or the id of the finding a
    /// capsule presents (R3). Default: none (a provider without capsules).
    fn capsule(&self, capsule_or_finding: &str) -> Option<Cow<'_, Capsule>> {
        let _ = capsule_or_finding;
        None
    }
    /// The meaning-glyph parameters of the capsule `capsule_or_finding` names (R3, the
    /// `glyph` mark; engine-derived, see [`crate::depth::story_glyph`]). Default: none.
    fn glyph(&self, capsule_or_finding: &str) -> Option<Cow<'_, GlyphSpec>> {
        let _ = capsule_or_finding;
        None
    }
    /// (R4/H-FP) The headline pieces of the capsule presenting `finding` for the reader
    /// this provider speaks for (its language and entitlement, like [`Self::headline`]).
    /// Read only by a face design with a headline template. Default: none (the face then
    /// shows [`Self::headline`]).
    fn headline_parts(&self, finding: &str) -> Option<HeadlineParts> {
        let _ = finding;
        None
    }
    /// (R5/m17-int, `docs/LANGUAGE.md`) The surfaces of the capsule presenting `finding`
    /// in the voice the reader's connection chose (its language and entitlement, like
    /// [`Self::headline`]). When given, every capsule scene speaks from it: the face's
    /// headline and ledger, the open capsule's detail and ledger, and the glossary terms
    /// marked in those texts. Default: none (today's text, the `r4` voice).
    fn voice(&self, finding: &str) -> Option<qp_schema::protocol::CapsuleVoice> {
        let _ = finding;
        None
    }
    /// (R5/M20) The language of the reader this provider speaks for: an open capsule
    /// design's own labels follow it. Default: none (Turkish).
    fn reader_lang(&self) -> Option<Language> {
        None
    }
    /// (R5/M20, K48) The device's own check of the capsule presenting `finding` (a viewer
    /// that checked it on its own data). Default: none (the authority).
    fn device_check(&self, finding: &str) -> Option<crate::designs::DeviceCheck> {
        let _ = finding;
        None
    }
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
    capsules: Vec<Capsule>,
    glyphs: BTreeMap<String, GlyphSpec>,
    parts: BTreeMap<String, HeadlineParts>,
    voices: BTreeMap<String, qp_schema::protocol::CapsuleVoice>,
    lang: Option<Language>,
    checks: BTreeMap<String, crate::designs::DeviceCheck>,
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
            capsules: Vec::new(),
            glyphs: BTreeMap::new(),
            parts: BTreeMap::new(),
            voices: BTreeMap::new(),
            lang: None,
            checks: BTreeMap::new(),
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

    /// Adds a capsule (R3). Its headline becomes the finding's headline; its glyph is
    /// derived from its story ([`crate::depth::story_glyph`] at the last bar of the story's
    /// series) unless one is set with [`MemProvider::with_glyph`].
    #[must_use]
    pub fn with_capsule(mut self, capsule: Capsule) -> Self {
        self.headlines
            .insert(capsule.finding.clone(), capsule.headline.clone());
        self.capsules.push(capsule);
        self
    }

    /// Sets the glyph parameters of a capsule or finding id.
    #[must_use]
    pub fn with_glyph(mut self, capsule_or_finding: &str, spec: GlyphSpec) -> Self {
        self.glyphs.insert(capsule_or_finding.to_owned(), spec);
        self
    }

    /// Sets the headline pieces of a finding (R4/H-FP).
    #[must_use]
    pub fn with_headline_parts(mut self, finding: &str, parts: HeadlineParts) -> Self {
        self.parts.insert(finding.to_owned(), parts);
        self
    }

    /// (R5/m17-int) Sets the voice surfaces of the capsule presenting `finding`.
    #[must_use]
    pub fn with_voice(mut self, finding: &str, voice: qp_schema::protocol::CapsuleVoice) -> Self {
        self.voices.insert(finding.to_owned(), voice);
        self
    }

    /// (R5/M20) Sets the reader's language.
    #[must_use]
    pub fn with_lang(mut self, lang: Language) -> Self {
        self.lang = Some(lang);
        self
    }

    /// (R5/M20) Sets the device's check of the capsule presenting `finding`.
    #[must_use]
    pub fn with_check(mut self, finding: &str, check: crate::designs::DeviceCheck) -> Self {
        self.checks.insert(finding.to_owned(), check);
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

    fn fact(&self, id: &str) -> Option<Cow<'_, Fact>> {
        self.facts.get(id).map(Cow::Borrowed)
    }

    fn finding(&self, id: &str) -> Option<Cow<'_, Finding>> {
        self.findings.get(id).map(Cow::Borrowed)
    }

    fn headline(&self, finding: &str) -> Option<String> {
        self.headlines.get(finding).cloned()
    }

    fn threshold(&self, name: &str) -> Option<f64> {
        self.thresholds.get(name).copied()
    }

    fn capsule(&self, capsule_or_finding: &str) -> Option<Cow<'_, Capsule>> {
        self.capsules
            .iter()
            .find(|c| c.id == capsule_or_finding)
            .or_else(|| {
                self.capsules
                    .iter()
                    .find(|c| c.finding == capsule_or_finding)
            })
            .map(Cow::Borrowed)
    }

    fn glyph(&self, capsule_or_finding: &str) -> Option<Cow<'_, GlyphSpec>> {
        if let Some(g) = self.glyphs.get(capsule_or_finding) {
            return Some(Cow::Borrowed(g));
        }
        let c = self.capsule(capsule_or_finding)?;
        let story = c.story.as_ref()?;
        let n = self.bars.get(&story.series)?.bars.len() as u64;
        Some(Cow::Owned(crate::depth::story_glyph(
            story,
            n.saturating_sub(1),
        )))
    }

    fn headline_parts(&self, finding: &str) -> Option<HeadlineParts> {
        self.parts.get(finding).cloned()
    }

    fn voice(&self, finding: &str) -> Option<qp_schema::protocol::CapsuleVoice> {
        self.voices.get(finding).cloned()
    }
    fn reader_lang(&self) -> Option<Language> {
        self.lang
    }
    fn device_check(&self, finding: &str) -> Option<crate::designs::DeviceCheck> {
        self.checks.get(finding).copied()
    }
}
