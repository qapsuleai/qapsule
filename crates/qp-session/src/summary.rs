//! The document summary sent in `welcome` (`DocSummary`, `docs/GRAMMAR.md` §12): enough
//! for an agent to refer to series, views, panels, layers, facts and findings by id in
//! its first message. Deterministic and compact (lists cut at [`MAX_ITEMS`]).

use qp_schema::grammar::{Layer, Mark, Position, ScaleKind, View};
use qp_schema::protocol::{
    DocSummary, FactSummary, FindingSummary, LayerSummary, PanelSummary, ProposalState,
    SeriesSummary, ViewSummary,
};
use qp_schema::{Document, Series, SeriesData, SeriesSource};

use crate::canon::{layer_data, position_constant, series_fields, series_main, Data};
use crate::error::num;
use crate::values::bars_of;

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
        fields: series_fields(s),
        symbol: s.symbol().map(str::to_owned),
        interval: s.interval.clone(),
        len,
        label: series_label(s),
    }
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
    let views = doc
        .views
        .iter()
        .take(MAX_ITEMS)
        .map(|v| view_summary(doc, v, &mut truncated))
        .collect();
    let mut facts: Vec<FactSummary> = doc
        .facts
        .iter()
        .map(|f| FactSummary {
            id: f.id.clone(),
            kind: f.kind.clone(),
            series: f.series.clone(),
            bar: f.bar,
            text: short(&f.text),
        })
        .collect();
    facts.sort_by_key(|f| f.bar);
    if facts.len() > MAX_ITEMS {
        truncated = true;
        facts.drain(..facts.len() - MAX_ITEMS);
    }
    let mut findings: Vec<FindingSummary> = doc
        .findings
        .iter()
        .map(|f| FindingSummary {
            id: f.id.clone(),
            claim: short(&f.claim),
            facts: f.facts.clone(),
            bar: f
                .facts
                .iter()
                .filter_map(|id| doc.facts.iter().find(|x| &x.id == id))
                .map(|x| x.bar)
                .max(),
            capsule: doc
                .capsules
                .iter()
                .find(|c| c.finding == f.id)
                .map(|c| format!("{} {}", c.id, state_name(&c.state))),
        })
        .collect();
    findings.sort_by_key(|f| f.bar);
    if findings.len() > MAX_ITEMS {
        truncated = true;
        findings.drain(..findings.len() - MAX_ITEMS);
    }
    let pending = doc
        .proposals
        .iter()
        .filter(|p| p.state == ProposalState::Pending)
        .count();
    DocSummary {
        series,
        views,
        facts,
        findings,
        pending_proposals: Some(u32::try_from(pending).unwrap_or(u32::MAX)),
        truncated,
    }
}
