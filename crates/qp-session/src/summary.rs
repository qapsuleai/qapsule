//! The document summary sent in `welcome` (`DocSummary`, `docs/GRAMMAR.md` §12): enough
//! for an agent to refer to series, views, panels, layers, facts and findings by id in
//! its first message. Deterministic and compact (lists cut at [`MAX_ITEMS`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qp_schema::grammar::{Layer, Mark, Position, ScaleKind, View};
use qp_schema::protocol::{
    DocSummary, FactRef, FactSel, FactSummary, FindingSel, FindingSummary, LayerSummary,
    PanelSummary, ProposalState, SelectHints, SeriesSummary, ViewSummary,
};
use qp_schema::{Document, Fact, Finding, Series, SeriesData, SeriesSource};

use crate::canon::{layer_data, position_constant, series_fields, series_main, Data};
use crate::error::num;
use crate::values::bars_of;

/// Resolver used for the `select` hints (§17): a selector that resolves to the item now.
fn hints(doc: &Document) -> crate::select::Resolver<'_> {
    crate::select::Resolver { doc, root: None }
}

/// Lists longer than this are cut (`truncated: true`).
pub(crate) const MAX_ITEMS: usize = 50;

/// Longest text (fact text, finding claim) kept, in characters.
const MAX_TEXT: usize = 120;

fn short(s: &str) -> String {
    if s.chars().count() <= MAX_TEXT {
        s.to_owned()
    } else {
        let mut t: String = s.chars().take(MAX_TEXT - 1).collect();
        t.push('…');
        t
    }
}

/// Legend text of a series: `RSI14`, `SMA20`, `MACD(12,26,9)`, `BOLLINGER(20,2)`; for
/// bars the symbol and interval.
pub(crate) fn series_label(s: &Series) -> String {
    match &s.source {
        SeriesSource::Study { study, params, .. } => {
            let up = study.to_uppercase();
            let p = |k: &str| params.get(k).map(|v| num(*v)).unwrap_or_default();
            match study.as_str() {
                "macd" => format!("{up}({},{},{})", p("fast"), p("slow"), p("signal")),
                "bollinger" => format!("{up}({},{})", p("period"), p("k")),
                _ if params.len() == 1 && params.contains_key("period") => {
                    format!("{up}{}", p("period"))
                }
                _ => format!(
                    "{up}({})",
                    params
                        .values()
                        .map(|v| num(*v))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            }
        }
        _ => match (s.symbol(), &s.interval) {
            (Some(sym), Some(i)) => format!("{sym} {i}"),
            (Some(sym), None) => sym.to_owned(),
            _ => s.id.clone(),
        },
    }
}

fn series_summary(doc: &Document, s: &Series) -> SeriesSummary {
    let (kind, study, params, inputs) = match &s.source {
        SeriesSource::Study {
            study,
            inputs,
            params,
            ..
        } => ("study", Some(study.clone()), params.clone(), inputs.clone()),
        _ => match &s.data {
            Some(SeriesData::Points { .. }) => ("points", None, Default::default(), Vec::new()),
            _ => ("bars", None, Default::default(), Vec::new()),
        },
    };
    let len = match &s.data {
        Some(SeriesData::Points { points }) => Some(points.len() as u64),
        _ => bars_of(doc, &s.id).map(|b| b.len() as u64),
    };
    SeriesSummary {
        id: s.id.clone(),
        kind: kind.to_owned(),
        study,
        params,
        inputs,
        fields: summary_fields(s),
        symbol: s.symbol().map(str::to_owned),
        interval: s.interval.clone(),
        len,
        label: series_label(s),
        binding: None,
    }
}

/// Fields of a series in the order `docs/PROTOCOL.md` documents them: the engine lists
/// Bollinger outputs as `middle, upper, lower, bandwidth`; the summary gives
/// `upper, middle, lower, bandwidth` (as §7 and the scene do).
fn summary_fields(s: &Series) -> Vec<String> {
    let mut f = series_fields(s);
    if matches!(&s.source, SeriesSource::Study { study, .. } if study == "bollinger") {
        const ORDER: [&str; 4] = ["upper", "middle", "lower", "bandwidth"];
        f.sort_by_key(|x| ORDER.iter().position(|o| o == x).unwrap_or(ORDER.len()));
    }
    f
}

/// Outputs a `line` draws by default (`docs/GRAMMAR.md` §5).
pub(crate) fn drawing_outputs(s: &Series) -> Vec<String> {
    match &s.source {
        SeriesSource::Study { study, output, .. } if output.is_none() => match study.as_str() {
            "bollinger" => vec!["upper".into(), "middle".into(), "lower".into()],
            "macd" => vec!["macd".into(), "signal".into()],
            _ => vec![series_main(s)],
        },
        _ => vec![series_main(s)],
    }
}

fn position_text(p: &Position) -> String {
    match p {
        Position::Number(v) => num(*v),
        Position::Name(s) => s.clone(),
        Position::Ref(r) => match (&r.series, &r.field) {
            (Some(s), Some(f)) => format!("{s}.{f}"),
            (Some(s), None) => s.clone(),
            (None, Some(f)) => f.clone(),
            (None, None) => String::new(),
        },
        Position::Panel(pp) => format!("panel {}{:+}px", num(pp.panel), pp.px),
    }
}

fn mark_name(m: Mark) -> &'static str {
    match m {
        Mark::Candle => "candles",
        Mark::Line => "line",
        Mark::Area => "area",
        Mark::Bar => "bar",
        Mark::Rule => "rule",
        Mark::Point => "point",
        Mark::Text => "text",
        Mark::Band => "band",
        Mark::Glyph => "glyph",
    }
}

/// What a layer draws, in a few words: `px candles`, `rsi14.rsi line`,
/// `rule 70 (rsi.overbought)`.
pub(crate) fn layer_about(doc: &Document, l: &Layer) -> String {
    let d = layer_data(doc, l);
    if l.mark == Mark::Rule {
        for (k, p) in [
            ("/encoding/y", &l.encoding.y),
            ("/encoding/x", &l.encoding.x),
        ] {
            let prefix = if k == "/encoding/x" {
                "vertical rule"
            } else {
                "rule"
            };
            let origin = l.origin.get(k).map(crate::canon::origin_text);
            match (p.as_ref().and_then(position_constant), origin) {
                (Some(v), Some(o)) => return format!("{prefix} {} ({o})", num(v)),
                (Some(v), None) => return format!("{prefix} {}", num(v)),
                (None, Some(o)) => return format!("{prefix} ({o})"),
                (None, None) => {}
            }
        }
        return "rule".to_owned();
    }
    let data = match d {
        Data::None => String::new(),
        Data::Series(s) => s.id.clone(),
        Data::Fact(f) => f.id.clone(),
        Data::Finding(f) => f.id.clone(),
    };
    if l.mark == Mark::Candle {
        return format!("{data} candles");
    }
    let field = match (&l.encoding.y, d) {
        (Some(p), _) => position_text(p),
        (None, Data::Series(s)) if l.mark == Mark::Line => drawing_outputs(s).join("/"),
        (None, _) => crate::canon::data_main(doc, d).unwrap_or_default(),
    };
    // `data.field`, unless one side is empty or the field already names its series.
    let mut what = if data.is_empty() || field.contains('.') {
        field
    } else if field.is_empty() {
        data
    } else {
        format!("{data}.{field}")
    };
    if let Some(y2) = &l.encoding.y2 {
        what = format!("{what}..{}", position_text(y2));
    }
    // (R3) A segment or bar-range band: its horizontal extent.
    if let Some(x2) = &l.encoding.x2 {
        let x = l
            .encoding
            .x
            .as_ref()
            .map_or_else(String::new, position_text);
        what = format!("{what} x {x}..{}", position_text(x2));
    }
    format!("{what} {}", mark_name(l.mark))
}

fn scale_text(v: &View, id: u32) -> String {
    let Some(p) = v.panel(id) else {
        return format!("y{id}");
    };
    let kind = match p.y.kind {
        ScaleKind::Time => "time",
        ScaleKind::Linear => "linear",
        ScaleKind::Log => "log",
    };
    match p.y.domain {
        Some([a, b]) => format!("y{id} {kind} [{}, {}]", num(a), num(b)),
        None => format!("y{id} {kind}"),
    }
}

fn view_summary(doc: &Document, v: &View, truncated: &mut bool) -> ViewSummary {
    let panels = v
        .panels
        .iter()
        .filter_map(|p| p.id)
        .map(|id| {
            let mut layers: Vec<LayerSummary> = v
                .layers
                .iter()
                .filter(|l| l.panel == id)
                .map(|l| LayerSummary {
                    id: l.id.clone().unwrap_or_default(),
                    mark: l.mark,
                    about: layer_about(doc, l),
                })
                .collect();
            if layers.len() > MAX_ITEMS {
                layers.truncate(MAX_ITEMS);
                *truncated = true;
            }
            PanelSummary {
                id,
                name: v.panel(id).and_then(|p| p.name.clone()),
                scale: scale_text(v, id),
                layers,
            }
        })
        .collect();
    ViewSummary {
        id: v.id.clone(),
        title: v.title.clone(),
        panels,
        emphasis: u32::try_from(v.emphasis.len()).unwrap_or(u32::MAX),
    }
}

fn state_name<T: serde::Serialize>(s: &T) -> String {
    serde_json::to_value(s)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A selector per listed item that resolves to it now (§17).
fn select_hints(
    doc: &Document,
    series: &[SeriesSummary],
    views: &[ViewSummary],
    facts: &[FactSummary],
    findings: &[FindingSummary],
) -> Option<SelectHints> {
    let r = hints(doc);
    let mut h = SelectHints::default();
    for s in series {
        if let Some(x) = doc.series.iter().find(|x| x.id == s.id) {
            if let Some(sel) = r.series_hint(x) {
                h.series.insert(s.id.clone(), sel);
            }
        }
    }
    for vs in views {
        let Some(v) = doc.views.iter().find(|x| x.id == vs.id) else {
            continue;
        };
        if let Some(sel) = r.view_hint(v) {
            h.views.insert(v.id.clone(), sel);
        }
        for p in &vs.panels {
            if let Some(sel) = r.panel_hint(v, p.id) {
                h.panels.insert(format!("{}/{}", v.id, p.id), sel);
            }
            for l in &p.layers {
                if let Some(sel) = v.layer(&l.id).and_then(|x| r.layer_hint(v, x)) {
                    h.layers.insert(format!("{}/{}", v.id, l.id), sel);
                }
            }
        }
    }
    let fh = FastHints::new(doc);
    for f in facts {
        if let Some(sel) = fh.fact_pos(&f.id).and_then(|i| fh.fact_hint(&doc.facts[i])) {
            h.facts.insert(f.id.clone(), sel);
        }
    }
    for f in findings {
        if let Some(sel) = fh
            .finding_pos(&f.id)
            .and_then(|i| fh.finding_hint(&doc.findings[i]))
        {
            h.findings.insert(f.id.clone(), sel);
        }
    }
    (h != SelectHints::default()).then_some(h)
}

/// (R3/perf, S-D §6.3 item 4) The fact and finding hints of the summary
/// ([`crate::select::Resolver::fact_hint`], [`crate::select::Resolver::finding_hint`]),
/// computed from indexes built once instead of resolving every candidate selector over
/// the whole document. The candidates, their order, the rows' order and the matching
/// rules are the resolver's; a test checks both give the same hints.
pub(crate) struct FastHints<'a> {
    doc: &'a Document,
    /// Fact id → its first position.
    fact_at: HashMap<&'a str, usize>,
    /// Finding id → its first position.
    finding_at: HashMap<&'a str, usize>,
    /// Finding positions in the resolver's row order (finding bar, position).
    gorder: Vec<usize>,
    /// Lower-case fact kind → fact positions, in row order (bar, position); built whole
    /// on first use.
    facts_of_kind: RefCell<HashMap<String, Rc<Vec<usize>>>>,
    /// Lower-case fact kind → finding positions with a fact of that kind, in row order;
    /// built whole on first use.
    findings_of_kind: RefCell<HashMap<String, Rc<Vec<usize>>>>,
    /// Fact id → finding positions resting on it, in row order; built whole on first use
    /// (R4/T6: one pass over the findings instead of one per asked fact).
    findings_of_fact: RefCell<Option<HashMap<&'a str, Rc<Vec<usize>>>>>,
}

impl<'a> FastHints<'a> {
    pub(crate) fn new(doc: &'a Document) -> Self {
        let mut fact_at: HashMap<&str, usize> = HashMap::with_capacity(doc.facts.len());
        for (i, f) in doc.facts.iter().enumerate() {
            fact_at.entry(f.id.as_str()).or_insert(i);
        }
        let mut finding_at: HashMap<&str, usize> = HashMap::with_capacity(doc.findings.len());
        for (i, f) in doc.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(i);
        }
        let mut gorder: Vec<(Option<u64>, usize)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.facts
                        .iter()
                        .filter_map(|id| fact_at.get(id.as_str()))
                        .map(|&j| doc.facts[j].bar)
                        .max(),
                    i,
                )
            })
            .collect();
        gorder.sort_unstable();
        Self {
            doc,
            fact_at,
            finding_at,
            gorder: gorder.into_iter().map(|(_, i)| i).collect(),
            facts_of_kind: RefCell::default(),
            findings_of_kind: RefCell::default(),
            findings_of_fact: RefCell::default(),
        }
    }

    /// Lower-case kind of every fact (a copy only per distinct raw kind).
    fn lower_kinds(&self) -> Vec<Rc<str>> {
        let mut seen: HashMap<&str, Rc<str>> = HashMap::new();
        self.doc
            .facts
            .iter()
            .map(|f| {
                Rc::clone(
                    seen.entry(f.kind.as_str())
                        .or_insert_with(|| Rc::from(f.kind.to_lowercase())),
                )
            })
            .collect()
    }

    /// (R4/T6) Both kind indexes in one pass each, on the first kind asked (one pass per
    /// asked kind before). Grouping by `to_lowercase` is the resolver's case-insensitive kind match.
    fn build_kind_indexes(&self) {
        if !self.facts_of_kind.borrow().is_empty() || self.doc.facts.is_empty() {
            return;
        }
        let lower = self.lower_kinds();
        let mut facts: HashMap<Rc<str>, Vec<(u64, usize)>> = HashMap::new();
        for (i, f) in self.doc.facts.iter().enumerate() {
            facts
                .entry(Rc::clone(&lower[i]))
                .or_default()
                .push((f.bar, i));
        }
        let mut findings: HashMap<Rc<str>, Vec<usize>> = HashMap::new();
        let mut kinds: Vec<&Rc<str>> = Vec::new();
        for &i in &self.gorder {
            kinds.clear();
            for id in &self.doc.findings[i].facts {
                if let Some(&j) = self.fact_at.get(id.as_str()) {
                    if !kinds.contains(&&lower[j]) {
                        kinds.push(&lower[j]);
                    }
                }
            }
            for k in &kinds {
                findings.entry(Rc::clone(k)).or_default().push(i);
            }
        }
        let mut fk = self.facts_of_kind.borrow_mut();
        for (k, mut v) in facts {
            v.sort_unstable();
            fk.insert(
                k.to_string(),
                Rc::new(v.into_iter().map(|(_, i)| i).collect()),
            );
        }
        let mut gk = self.findings_of_kind.borrow_mut();
        for (k, v) in findings {
            gk.insert(k.to_string(), Rc::new(v));
        }
    }

    fn kind_facts(&self, lower: &str) -> Rc<Vec<usize>> {
        self.build_kind_indexes();
        self.facts_of_kind
            .borrow()
            .get(lower)
            .map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    fn kind_findings(&self, lower: &str) -> Rc<Vec<usize>> {
        self.build_kind_indexes();
        self.findings_of_kind
            .borrow()
            .get(lower)
            .map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    fn fact_findings(&self, id: &str) -> Rc<Vec<usize>> {
        let mut cache = self.findings_of_fact.borrow_mut();
        let index = cache.get_or_insert_with(|| {
            let mut m: HashMap<&'a str, Vec<usize>> = HashMap::new();
            for &i in &self.gorder {
                let facts = &self.doc.findings[i].facts;
                for (k, x) in facts.iter().enumerate() {
                    // A finding counts once per fact id, however often it names it.
                    if facts[..k].iter().all(|y| y != x) {
                        m.entry(x.as_str()).or_default().push(i);
                    }
                }
            }
            m.into_iter().map(|(k, v)| (k, Rc::new(v))).collect()
        });
        index.get(id).map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    pub(crate) fn fact_pos(&self, id: &str) -> Option<usize> {
        self.fact_at.get(id).copied()
    }

    pub(crate) fn finding_pos(&self, id: &str) -> Option<usize> {
        self.finding_at.get(id).copied()
    }

    /// The ids a fact selector made of `kind`, `bar`, `first`, `latest` resolves to.
    fn fact_ids(&self, s: &FactSel) -> Vec<&'a str> {
        let doc = self.doc;
        if s.first && s.latest {
            return Vec::new();
        }
        let Some(k) = &s.kind else {
            unreachable!("hint candidates name a kind");
        };
        let rows = self.kind_facts(&k.to_lowercase());
        let mut hits = rows
            .iter()
            .filter(|&&i| s.bar.is_none_or(|b| doc.facts[i].bar == b))
            .map(|&i| doc.facts[i].id.as_str());
        // (R4/T6) `pick` without collecting every row for `first` / `latest`.
        if s.first {
            hits.next().into_iter().collect()
        } else if s.latest {
            hits.last().into_iter().collect()
        } else {
            hits.collect()
        }
    }

    /// The ids a finding selector made of `kind` or `fact`, `first`, `latest` resolves to.
    fn finding_ids(&self, s: &FindingSel) -> Vec<&'a str> {
        let doc = self.doc;
        if s.first && s.latest {
            return Vec::new();
        }
        let rows: Rc<Vec<usize>> = match (&s.kind, &s.fact) {
            (Some(k), None) => self.kind_findings(&k.to_lowercase()),
            (None, Some(r)) => {
                let FactRef::Select(h) = r.as_ref() else {
                    unreachable!("hint candidates select their fact");
                };
                let set = self.fact_ids(h);
                match set.len() {
                    0 => return Vec::new(),
                    1 => self.fact_findings(set[0]),
                    _ => {
                        let mut m = self.finding_ids_slow(&set);
                        pick(&mut m, s.first, s.latest);
                        return m;
                    }
                }
            }
            _ => unreachable!("hint candidates: kind or fact"),
        };
        let mut m: Vec<&str> = rows.iter().map(|&i| doc.findings[i].id.as_str()).collect();
        pick(&mut m, s.first, s.latest);
        m
    }

    /// Findings resting on any of `set`, in row order (several facts: never for a hint,
    /// which resolves to one fact).
    fn finding_ids_slow(&self, set: &[&str]) -> Vec<&'a str> {
        let doc = self.doc;
        let mut order: Vec<(Option<u64>, usize)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.facts
                        .iter()
                        .filter_map(|id| self.fact_at.get(id.as_str()))
                        .map(|&j| doc.facts[j].bar)
                        .max(),
                    i,
                )
            })
            .collect();
        order.sort_unstable();
        order
            .into_iter()
            .map(|(_, i)| &doc.findings[i])
            .filter(|f| f.facts.iter().any(|x| set.contains(&x.as_str())))
            .map(|f| f.id.as_str())
            .collect()
    }

    /// [`crate::select::Resolver::fact_hint`].
    pub(crate) fn fact_hint(&self, f: &Fact) -> Option<FactSel> {
        let kind = FactSel {
            kind: Some(f.kind.clone()),
            ..FactSel::default()
        };
        let cands = vec![
            kind.clone(),
            FactSel {
                latest: true,
                ..kind.clone()
            },
            FactSel {
                first: true,
                ..kind.clone()
            },
            FactSel {
                bar: Some(f.bar),
                ..kind.clone()
            },
        ];
        choose(
            cands,
            &f.id,
            |c| self.fact_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// [`crate::select::Resolver::finding_hint`].
    pub(crate) fn finding_hint(&self, f: &Finding) -> Option<FindingSel> {
        let doc = self.doc;
        let latest = f
            .facts
            .iter()
            .filter_map(|id| self.fact_at.get(id.as_str()).map(|&i| &doc.facts[i]))
            .max_by_key(|x| x.bar)?;
        let kind = FindingSel {
            kind: Some(latest.kind.clone()),
            ..FindingSel::default()
        };
        let mut cands = vec![
            kind.clone(),
            FindingSel {
                latest: true,
                ..kind.clone()
            },
            FindingSel {
                first: true,
                ..kind.clone()
            },
        ];
        if let Some(h) = self.fact_hint(latest) {
            cands.push(FindingSel {
                fact: Some(Box::new(FactRef::Select(h))),
                ..FindingSel::default()
            });
        }
        choose(
            cands,
            &f.id,
            |c| self.finding_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }
}

/// `select::pick` without the error (both flags are handled by the caller).
fn pick(m: &mut Vec<&str>, first: bool, latest: bool) {
    if first {
        m.truncate(1);
    } else if latest && m.len() > 1 {
        m.drain(..m.len() - 1);
    }
}

/// `select::Resolver::choose`.
fn choose<'x, S: Clone>(
    cands: Vec<S>,
    target: &str,
    ids: impl Fn(&S) -> Vec<&'x str>,
    set_first: impl Fn(&mut S),
    set_latest: impl Fn(&mut S),
) -> Option<S> {
    let found: Vec<Vec<&'x str>> = cands.iter().map(&ids).collect();
    for (c, m) in cands.iter().zip(&found) {
        if m.len() == 1 && m[0] == target {
            return Some(c.clone());
        }
    }
    for (c, m) in cands.iter().zip(&found) {
        if m.first().is_some_and(|x| *x == target) {
            let mut c = c.clone();
            set_first(&mut c);
            return Some(c);
        }
        if m.last().is_some_and(|x| *x == target) {
            let mut c = c.clone();
            set_latest(&mut c);
            return Some(c);
        }
    }
    None
}

/// The summary of a document.
#[must_use]
pub(crate) fn summarize_doc(doc: &Document) -> DocSummary {
    let mut truncated = false;
    let mut series: Vec<SeriesSummary> = doc
        .series
        .iter()
        .take(MAX_ITEMS)
        .map(|s| series_summary(doc, s))
        .collect();
    if doc.series.len() > MAX_ITEMS {
        truncated = true;
        series.truncate(MAX_ITEMS);
    }
    if doc.views.len() > MAX_ITEMS {
        truncated = true;
    }
    let views: Vec<ViewSummary> = doc
        .views
        .iter()
        .take(MAX_ITEMS)
        .map(|v| view_summary(doc, v, &mut truncated))
        .collect();
    // (R3/perf) The last `MAX_ITEMS` by bar (stable order), chosen before their
    // summaries are written.
    let mut order: Vec<usize> = (0..doc.facts.len()).collect();
    order.sort_by_key(|&i| doc.facts[i].bar);
    if order.len() > MAX_ITEMS {
        truncated = true;
        order.drain(..order.len() - MAX_ITEMS);
    }
    let facts: Vec<FactSummary> = order
        .into_iter()
        .map(|i| &doc.facts[i])
        .map(|f| FactSummary {
            id: f.id.clone(),
            kind: f.kind.clone(),
            series: f.series.clone(),
            bar: f.bar,
            text: short(&f.text),
        })
        .collect();
    // First positions (as `iter().find` gives them).
    let mut fact_at: HashMap<&str, usize> = HashMap::new();
    for (i, f) in doc.facts.iter().enumerate() {
        fact_at.entry(f.id.as_str()).or_insert(i);
    }
    let mut capsule_of: HashMap<&str, usize> = HashMap::new();
    for (i, c) in doc.capsules.iter().enumerate() {
        capsule_of.entry(c.finding.as_str()).or_insert(i);
    }
    let finding_bar = |f: &Finding| {
        f.facts
            .iter()
            .filter_map(|id| fact_at.get(id.as_str()).map(|&i| &doc.facts[i]))
            .map(|x| x.bar)
            .max()
    };
    let mut order: Vec<(Option<u64>, usize)> = doc
        .findings
        .iter()
        .enumerate()
        .map(|(i, f)| (finding_bar(f), i))
        .collect();
    order.sort_by_key(|(b, _)| *b);
    if order.len() > MAX_ITEMS {
        truncated = true;
        order.drain(..order.len() - MAX_ITEMS);
    }
    let findings: Vec<FindingSummary> = order
        .into_iter()
        .map(|(bar, i)| {
            let f = &doc.findings[i];
            FindingSummary {
                id: f.id.clone(),
                claim: short(&f.claim),
                facts: f.facts.clone(),
                bar,
                capsule: capsule_of
                    .get(f.id.as_str())
                    .map(|&i| &doc.capsules[i])
                    .map(|c| format!("{} {}", c.id, state_name(&c.state))),
            }
        })
        .collect();
    let pending = doc
        .proposals
        .iter()
        .filter(|p| p.state == ProposalState::Pending)
        .count();
    let select = select_hints(doc, &series, &views, &facts, &findings);
    DocSummary {
        series,
        views,
        facts,
        findings,
        pending_proposals: Some(u32::try_from(pending).unwrap_or(u32::MAX)),
        truncated,
        select,
    }
}

#[cfg(test)]
mod tests {
    use super::FastHints;
    use crate::select::Resolver;

    /// The summary's indexed hints are the resolver's, for every fact and finding of a
    /// document with an engine history (many facts per kind, findings sharing facts).
    #[test]
    fn fast_hints_equal_the_resolvers() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let t: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                root.join("fixtures/protocol/conformance/38-capsule-lifecycle.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut init = t["init"].clone();
        let bars = qp_source::synthetic_market(1, 260, 1)[0].bars.clone();
        init["data"]["feeds"][0]["bars"] = serde_json::json!(bars);
        let s = crate::Session::new(serde_json::from_value(init).unwrap(), &root).unwrap();
        let doc = s.document();
        assert!(
            doc.facts.len() > 50 && doc.findings.len() > 20,
            "a real history"
        );
        let fast = FastHints::new(doc);
        let r = Resolver { doc, root: None };
        for f in &doc.facts {
            assert_eq!(fast.fact_hint(f), r.fact_hint(f), "fact {}", f.id);
        }
        for f in &doc.findings {
            assert_eq!(fast.finding_hint(f), r.finding_hint(f), "finding {}", f.id);
        }
    }
}
