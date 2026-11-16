//! Layer preparation: resolves a layer's data, channels, constants and colours into
//! value rows (LOD-aggregated when the frame says so). Pixel work happens in `draw`.

use std::collections::BTreeMap;

use qp_schema::capsule::GlyphSpec;
use qp_schema::grammar::{
    Cmp, ColorSpec, Layer, Mark, Operand, Origin, PanelPos, Position, SizeSpec,
};
use qp_schema::scene::{DataSource, ResolvedConstant, Rgba};

use crate::error::{ErrorKind, GrammarError};
use crate::frame::{Column, Frame, Row};
use crate::provider::SeriesKind;
use crate::theme::with_opacity;

/// What a layer draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Geom {
    Candle,
    Line,
    Area,
    Band,
    Bar,
    RuleH,
    RuleV,
    Point,
    Text,
    /// A rule segment from bar `x` to bar `x2` (R3 evidence geometry).
    Segment,
    /// A meaning glyph (R3).
    Glyph,
}

/// Panel-relative layout positions of a `text` or `glyph` layer (R3).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Anchor {
    pub x: Option<PanelPos>,
    pub y: Option<PanelPos>,
}

/// Default glyph size in pixels.
pub(crate) const GLYPH_SIZE: f64 = 24.0;

/// One drawn element's values.
#[derive(Debug, Clone)]
pub(crate) struct PRow {
    /// First slot.
    pub slot: u64,
    /// Bars aggregated (1 without LOD).
    pub n: u64,
    /// Time of the first slot.
    pub t: Option<i64>,
    /// Values; meaning per geometry (see `Prepared::keys`). For a LOD line:
    /// `[first, a, b, last]` with `a`, `b` the extremes in time order.
    pub v: [Option<f64>; 4],
    /// Final colour (opacity applied).
    pub color: Rgba,
    /// Colour-driving quantity: `left − right` of a condition, or the stepped field.
    pub ckey: Option<f64>,
    /// Point diameter / text size.
    pub size: f64,
    /// Text of a `text` mark.
    pub text: Option<String>,
}

/// How colour changes between two rows are split.
#[derive(Debug, Clone)]
pub(crate) enum Split {
    /// Constant colour (or per-row colours without interpolation, e.g. candles).
    None,
    /// Condition: split where `ckey` crosses zero.
    When,
    /// Steps: split where `ckey` crosses each threshold.
    Steps {
        thresholds: Vec<f64>,
        colors: Vec<Rgba>,
    },
}

/// One drawn sequence (a line per output; one for every other mark).
#[derive(Debug, Clone)]
pub(crate) struct Trace {
    pub rows: Vec<PRow>,
    pub split: Split,
    /// Colour for legends and tags (before opacity), at the last visible slot.
    pub base: Rgba,
    /// Rows hold `[first, a, b, last]` (LOD line).
    pub lod_line: bool,
}

/// A layer ready to draw.
#[derive(Debug, Clone)]
pub(crate) struct Prepared {
    pub id: String,
    pub mark: Mark,
    pub panel: u32,
    pub data: DataSource,
    pub fields: Vec<String>,
    pub constants: BTreeMap<String, ResolvedConstant>,
    pub label: Option<String>,
    pub geom: Geom,
    pub traces: Vec<Trace>,
    /// Item value keys per value index; for lines, one key per trace (index = trace).
    pub keys: Vec<String>,
    pub width: f64,
    /// Band edges are stroked only when `width` is given.
    pub edge: bool,
    pub skipped: u64,
    /// Whether the layer is bound to data (first data layer gets the last-value tag).
    pub has_data: bool,
    /// Panel-relative positions (`text`, `glyph`; R3).
    pub anchor: Anchor,
    /// Bar extent `x..=x2` of a segment (R3).
    pub extent: Option<(u64, u64)>,
    /// Glyph parameters, one per row of the first trace (`glyph`; R3).
    pub glyphs: Vec<GlyphSpec>,
}

/// A channel: a column by slot, a row field, or a constant.
enum Chan<'f> {
    Col(Column<'f>),
    Field(String),
    Const(f64),
}

impl Chan<'_> {
    fn at(&self, slot: u64, row: Option<&Row>) -> Option<f64> {
        match self {
            Chan::Col(c) => c.get(slot),
            Chan::Field(f) => row?.nums.get(f).copied().filter(|v| v.is_finite()),
            Chan::Const(v) => Some(*v),
        }
    }

    /// Whether the channel's source has an entry at `slot` (see `Column::has`).
    fn has(&self, slot: u64) -> bool {
        match self {
            Chan::Col(c) => c.has(slot),
            Chan::Field(_) | Chan::Const(_) => true,
        }
    }
}

enum ColorRule<'f> {
    Const(Rgba),
    Candle {
        up: Rgba,
        down: Rgba,
    },
    When {
        left: Chan<'f>,
        op: Cmp,
        right: Chan<'f>,
        then: Rgba,
        otherwise: Rgba,
    },
    Steps {
        field: Chan<'f>,
        thresholds: Vec<f64>,
        colors: Vec<Rgba>,
    },
}

impl ColorRule<'_> {
    /// Colour and colour key at a slot/row. `ohlc` is used by the candle default.
    fn at(&self, slot: u64, row: Option<&Row>, oc: Option<(f64, f64)>) -> (Rgba, Option<f64>) {
        match self {
            ColorRule::Const(c) => (*c, None),
            ColorRule::Candle { up, down } => match oc {
                Some((o, c)) if c >= o => (*up, None),
                _ => (*down, None),
            },
            ColorRule::When {
                left,
                op,
                right,
                then,
                otherwise,
            } => match (left.at(slot, row), right.at(slot, row)) {
                (Some(l), Some(r)) => {
                    let holds = match op {
                        Cmp::Lt => l < r,
                        Cmp::Le => l <= r,
                        Cmp::Gt => l > r,
                        Cmp::Ge => l >= r,
                    };
                    (if holds { *then } else { *otherwise }, Some(l - r))
                }
                _ => (*otherwise, None),
            },
            ColorRule::Steps {
                field,
                thresholds,
                colors,
            } => match field.at(slot, row) {
                Some(v) => (colors[step_index(thresholds, v)], Some(v)),
                None => (colors[0], None),
            },
        }
    }

    fn split(&self, opacity: f64) -> Split {
        match self {
            ColorRule::When { .. } => Split::When,
            ColorRule::Steps {
                thresholds, colors, ..
            } => Split::Steps {
                thresholds: thresholds.clone(),
                colors: colors.iter().map(|c| with_opacity(*c, opacity)).collect(),
            },
            _ => Split::None,
        }
    }
}

/// `i` such that `thresholds[i-1] <= v < thresholds[i]`.
pub(crate) fn step_index(thresholds: &[f64], v: f64) -> usize {
    thresholds.iter().take_while(|t| **t <= v).count()
}

enum Src<'s> {
    Slots(&'s str),
    Rows,
    Nothing,
}

/// Default opacity per mark (`docs/GRAMMAR.md` §5).
fn default_opacity(mark: Mark) -> f64 {
    match mark {
        Mark::Area => 0.25,
        Mark::Band => 0.2,
        _ => 1.0,
    }
}

/// Resolves the layer's origins (`docs/GRAMMAR.md` §10).
fn resolve_constants(
    fr: &Frame<'_>,
    layer: &Layer,
    id: &str,
) -> Result<BTreeMap<String, ResolvedConstant>, GrammarError> {
    let mut out = BTreeMap::new();
    let n_thresholds = match &layer.encoding.color {
        Some(ColorSpec::Steps(s)) => s.thresholds.len(),
        _ => 0,
    };
    for (ptr, origin) in &layer.origin {
        let valid = matches!(
            ptr.as_str(),
            "/encoding/x"
                | "/encoding/x2"
                | "/encoding/y"
                | "/encoding/y2"
                | "/encoding/color/when/than"
        ) || ptr
            .strip_prefix("/encoding/color/thresholds/")
            .and_then(|i| i.parse::<usize>().ok())
            .is_some_and(|i| i < n_thresholds);
        if !valid {
            return Err(GrammarError::in_layer(
                ErrorKind::Invalid,
                id,
                format!("/origin/{}", ptr.replace('~', "~0").replace('/', "~1")),
                format!("origin pointer `{ptr}` does not name a numeric constant of the layer"),
            ));
        }
        let nf = |what: String| GrammarError::in_layer(ErrorKind::NotFound, id, ptr.as_str(), what);
        let value = match origin {
            Origin::Fact { fact, field } => {
                let f =
                    fr.p.fact(fact)
                        .ok_or_else(|| nf(format!("origin fact `{fact}` does not exist")))?;
                #[allow(clippy::cast_precision_loss)]
                let v = match field.as_str() {
                    "bar" => Some(f.bar as f64),
                    "t" => Some(f.t as f64),
                    "value" => fr.fact_value(&f, id, ptr)?,
                    k => f
                        .evidence
                        .get(k)
                        .copied()
                        .or_else(|| crate::depth::geometry_field(&f, k)),
                };
                v.ok_or_else(|| {
                    nf(format!(
                        "origin fact `{fact}` has no numeric field `{field}` (fields: bar, t, value{}{})",
                        if f.evidence.is_empty() { "" } else { ", " },
                        f.evidence.keys().cloned().collect::<Vec<_>>().join(", ")
                    ))
                })?
            }
            Origin::Threshold { name } => {
                fr.p.threshold(name)
                    .ok_or_else(|| nf(format!("unknown named threshold `{name}`")))?
            }
            Origin::Series { series, field, .. } => {
                let field = match field {
                    Some(f) => f.clone(),
                    None => fr
                        .main_field(series)
                        .ok_or_else(|| nf(format!("origin series `{series}` does not exist")))?,
                };
                let col = fr.column(series, &field, id, ptr)?;
                let last = match fr.kind(series) {
                    Some(SeriesKind::Points) => fr.total_slots(),
                    _ => fr.n_bars(),
                };
                (0..last).rev().find_map(|s| col.get(s)).ok_or_else(|| {
                    nf(format!(
                        "origin series `{series}` has no value for `{field}`"
                    ))
                })?
            }
        };
        out.insert(
            ptr.clone(),
            ResolvedConstant {
                origin: origin.clone(),
                value,
            },
        );
    }
    Ok(out)
}

struct Ctx<'f, 'l> {
    fr: &'f Frame<'f>,
    id: &'l str,
    layer: &'l Layer,
    src: Src<'l>,
    /// All rows of a fact/finding layer (for field validation).
    all_rows: Vec<Row>,
    constants: BTreeMap<String, ResolvedConstant>,
}

impl<'f> Ctx<'f, '_> {
    fn err(&self, kind: ErrorKind, ptr: &str, why: impl Into<String>) -> GrammarError {
        GrammarError::in_layer(kind, self.id, ptr, why)
    }

    /// A constant at `ptr` (written value `written`, if any), checked against its origin.
    fn constant(&self, ptr: &str, written: Option<f64>) -> Result<f64, GrammarError> {
        match self.constants.get(ptr) {
            Some(rc) => {
                if let Some(w) = written {
                    let live = matches!(rc.origin, Origin::Series { .. });
                    if !live && w.to_bits() != rc.value.to_bits() {
                        return Err(self.err(
                            ErrorKind::NoOrigin,
                            ptr,
                            format!(
                                "constant {w} does not match its origin's value {}",
                                rc.value
                            ),
                        ));
                    }
                }
                Ok(rc.value)
            }
            None => Err(self.err(
                ErrorKind::NoOrigin,
                ptr,
                format!(
                    "constant {} has no origin (bind it to a fact or a named threshold via the layer's `origin`)",
                    written.map_or_else(String::new, crate::detmath::num)
                ),
            )),
        }
    }

    /// A field of the layer's own data.
    fn field(&self, name: &str, ptr: &str) -> Result<Chan<'f>, GrammarError> {
        match &self.src {
            Src::Slots(series) => Ok(Chan::Col(self.fr.column(series, name, self.id, ptr)?)),
            Src::Rows => {
                if self.all_rows.iter().any(|r| r.nums.contains_key(name)) {
                    Ok(Chan::Field(name.to_owned()))
                } else {
                    let mut keys: Vec<String> = Vec::new();
                    for r in &self.all_rows {
                        for k in r.nums.keys() {
                            if !keys.contains(k) {
                                keys.push(k.clone());
                            }
                        }
                    }
                    Err(self.err(
                        ErrorKind::NotFound,
                        ptr,
                        format!(
                            "field `{name}` not found in the layer's data (fields: {})",
                            keys.join(", ")
                        ),
                    ))
                }
            }
            Src::Nothing => Err(self.err(
                ErrorKind::Invalid,
                ptr,
                format!("the layer has no data (series, fact or finding), so `{name}` cannot be a field"),
            )),
        }
    }

    /// Main field of the layer's own data.
    fn main(&self) -> Option<String> {
        match &self.src {
            Src::Slots(series) => self.fr.main_field(series),
            Src::Rows => Some(
                if self.all_rows.iter().any(|r| r.nums.contains_key("level")) {
                    "level".to_owned()
                } else {
                    "value".to_owned()
                },
            ),
            Src::Nothing => None,
        }
    }

    /// A data reference (`{"series","field"}`) → channel and item key.
    fn data_ref(
        &self,
        series: Option<&str>,
        field: Option<&str>,
        ptr: &str,
    ) -> Result<(Chan<'f>, String), GrammarError> {
        match series {
            None => {
                let f = match field {
                    Some(f) => f.to_owned(),
                    None => self.main().ok_or_else(|| {
                        self.err(
                            ErrorKind::Invalid,
                            ptr,
                            "no field and the layer has no data",
                        )
                    })?,
                };
                Ok((self.field(&f, ptr)?, f))
            }
            Some(s) => {
                let f = match field {
                    Some(f) => f.to_owned(),
                    None => self.fr.main_field(s).ok_or_else(|| {
                        self.err(
                            ErrorKind::NotFound,
                            ptr,
                            format!("series `{s}` does not exist"),
                        )
                    })?,
                };
                let key = if matches!(&self.src, Src::Slots(own) if *own == s) {
                    f.clone()
                } else {
                    format!("{s}.{f}")
                };
                Ok((Chan::Col(self.fr.column(s, &f, self.id, ptr)?), key))
            }
        }
    }

    /// A position channel → channel and item key (`None` when absent and no origin).
    fn position(
        &self,
        pos: Option<&Position>,
        ptr: &str,
        key: &str,
    ) -> Result<Option<(Chan<'f>, String)>, GrammarError> {
        match pos {
            None => {
                if self.constants.contains_key(ptr) {
                    Ok(Some((
                        Chan::Const(self.constant(ptr, None)?),
                        key.to_owned(),
                    )))
                } else {
                    Ok(None)
                }
            }
            Some(Position::Number(n)) => Ok(Some((
                Chan::Const(self.constant(ptr, Some(*n))?),
                key.to_owned(),
            ))),
            Some(Position::Name(s)) => match s.trim().parse::<f64>() {
                Ok(n) if n.is_finite() => Ok(Some((
                    Chan::Const(self.constant(ptr, Some(n))?),
                    key.to_owned(),
                ))),
                _ => Ok(Some((self.field(s, ptr)?, s.clone()))),
            },
            Some(Position::Ref(r)) => self
                .data_ref(r.series.as_deref(), r.field.as_deref(), ptr)
                .map(Some),
            Some(Position::Panel(_)) => Err(self.err(
                ErrorKind::Invalid,
                ptr,
                "a panel-relative position is only for `text` and `glyph`",
            )),
        }
    }

    /// A bar constant (`x`, `x2` of a segment, band extent or point; R3): a number with an
    /// origin. A bar-valued origin (a fact's `bar`, a geometry bar) reads as a bar index,
    /// anything else as a time (like a vertical rule's `x`).
    fn bar_const(&self, ptr: &str, pos: Option<&Position>) -> Result<Option<u64>, GrammarError> {
        let v = match pos {
            None if self.constants.contains_key(ptr) => self.constant(ptr, None)?,
            None => return Ok(None),
            Some(Position::Number(n)) => self.constant(ptr, Some(*n))?,
            Some(Position::Name(s)) if s.trim().parse::<f64>().is_ok_and(f64::is_finite) => {
                self.constant(ptr, s.trim().parse::<f64>().ok())?
            }
            Some(_) => {
                return Err(self.err(
                    ErrorKind::Invalid,
                    ptr,
                    "here a bar: a numeric constant with an origin (e.g. a fact's `bar` or a geometry bar)",
                ))
            }
        };
        let by_bar = matches!(
            self.constants.get(ptr).map(|c| &c.origin),
            Some(Origin::Fact { field, .. }) if is_bar_field(field)
        );
        let slot = if by_bar {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            (v >= 0.0 && v.fract() == 0.0).then_some(v as u64)
        } else {
            self.fr.time_slot(v)
        };
        slot.map(Some).ok_or_else(|| {
            self.err(
                ErrorKind::Invalid,
                ptr,
                format!("{} is not a bar of the view", crate::detmath::num(v)),
            )
        })
    }

    fn color(&self, spec: &str, ptr: &str) -> Result<Rgba, GrammarError> {
        self.fr
            .theme
            .color(spec)
            .map_err(|e| self.err(ErrorKind::Invalid, ptr, e))
    }

    fn color_rule(&self, default: ColorRule<'f>) -> Result<ColorRule<'f>, GrammarError> {
        let Some(spec) = &self.layer.encoding.color else {
            return Ok(default);
        };
        Ok(match spec {
            ColorSpec::Constant(c) => ColorRule::Const(self.color(c, "/encoding/color")?),
            ColorSpec::When(w) => {
                let left = self.field(&w.when.field, "/encoding/color/when/field")?;
                let ptr = "/encoding/color/when/than";
                let right = match &w.when.than {
                    Operand::Number(n) => Chan::Const(self.constant(ptr, Some(*n))?),
                    Operand::Ref(r) => {
                        self.data_ref(r.series.as_deref(), r.field.as_deref(), ptr)?
                            .0
                    }
                };
                ColorRule::When {
                    left,
                    op: w.when.op,
                    right,
                    then: self.color(&w.then, "/encoding/color/then")?,
                    otherwise: self.color(&w.otherwise, "/encoding/color/else")?,
                }
            }
            ColorSpec::Steps(s) => {
                let field = self.field(&s.field, "/encoding/color/field")?;
                if s.colors.len() != s.thresholds.len() + 1 {
                    return Err(self.err(
                        ErrorKind::Invalid,
                        "/encoding/color/colors",
                        format!(
                            "{} colours for {} thresholds (need thresholds + 1)",
                            s.colors.len(),
                            s.thresholds.len()
                        ),
                    ));
                }
                let mut thresholds = Vec::with_capacity(s.thresholds.len());
                for (i, t) in s.thresholds.iter().enumerate() {
                    let v = self.constant(&format!("/encoding/color/thresholds/{i}"), Some(*t))?;
                    if thresholds.last().is_some_and(|p| *p >= v) {
                        return Err(self.err(
                            ErrorKind::Invalid,
                            "/encoding/color/thresholds",
                            "thresholds must be strictly ascending",
                        ));
                    }
                    thresholds.push(v);
                }
                let colors = s
                    .colors
                    .iter()
                    .enumerate()
                    .map(|(i, c)| self.color(c, &format!("/encoding/color/colors/{i}")))
                    .collect::<Result<Vec<_>, _>>()?;
                ColorRule::Steps {
                    field,
                    thresholds,
                    colors,
                }
            }
        })
    }

    fn visible_rows(&self) -> impl Iterator<Item = &Row> {
        let (from, to) = (self.fr.from, self.fr.to);
        self.all_rows
            .iter()
            .filter(move |r| r.slot >= from && r.slot <= to)
    }
}

/// Whether a fact field names a bar (`bar`, or a geometry bar; R3).
pub(crate) fn is_bar_field(field: &str) -> bool {
    field == "bar"
        || (field.starts_with("geometry.")
            && (field.ends_with(".from_bar")
                || field.ends_with(".to_bar")
                || field.ends_with(".bar")))
}

/// Whether a position is panel-relative.
fn is_panel(p: Option<&Position>) -> bool {
    matches!(p, Some(Position::Panel(_)))
}

fn panel_pos(p: Option<&Position>) -> Option<PanelPos> {
    match p {
        Some(Position::Panel(pp)) => Some(*pp),
        _ => None,
    }
}

/// Prepares one layer. `palette` counts uncoloured line/area/band traces in the panel.
#[allow(clippy::too_many_lines)]
pub(crate) fn prepare(
    fr: &Frame<'_>,
    layer: &Layer,
    id: &str,
    palette: &mut usize,
) -> Result<Prepared, GrammarError> {
    let n_data = [&layer.series, &layer.fact, &layer.finding]
        .iter()
        .filter(|d| d.is_some())
        .count();
    if n_data > 1 {
        return Err(GrammarError::in_layer(
            ErrorKind::Invalid,
            id,
            "",
            "a layer has at most one of `series`, `fact`, `finding`",
        ));
    }
    let constants = resolve_constants(fr, layer, id)?;
    let mut all_rows = Vec::new();
    let (src, data) = if let Some(s) = &layer.series {
        if fr.kind(s).is_none() {
            return Err(GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/series",
                format!("series `{s}` does not exist"),
            ));
        }
        (
            Src::Slots(s.as_str()),
            DataSource::Series { series: s.clone() },
        )
    } else if let Some(f) = &layer.fact {
        let fact = fr.p.fact(f).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/fact",
                format!("fact `{f}` does not exist"),
            )
        })?;
        all_rows.push(fr.fact_row(&fact, id, "/fact")?);
        (Src::Rows, DataSource::Fact { fact: f.clone() })
    } else if let Some(f) = &layer.finding {
        let finding = fr.p.finding(f).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/finding",
                format!("finding `{f}` does not exist"),
            )
        })?;
        for fid in &finding.facts {
            let fact = fr.p.fact(fid).ok_or_else(|| {
                GrammarError::in_layer(
                    ErrorKind::NotFound,
                    id,
                    "/finding",
                    format!("fact `{fid}` of finding `{f}` does not exist"),
                )
            })?;
            let mut row = fr.fact_row(&fact, id, "/finding")?;
            row.nums.insert("confidence".to_owned(), finding.confidence);
            row.texts.insert("claim".to_owned(), finding.claim.clone());
            all_rows.push(row);
        }
        // R3 capsule texts (the capsule card): its sentence and its ledger line. (R4/H-FP)
        // In a face scene whose design has a headline template, the template's sentence.
        let headline = crate::faces::headline_text(fr.headline, f, fr.p);
        let ledger = fr.p.capsule(f).and_then(|c| {
            c.story
                .as_ref()
                .map(|s| crate::depth::ledger_text(&s.ledger_at_birth))
        });
        for row in &mut all_rows {
            if let Some(h) = &headline {
                row.texts.insert("headline".to_owned(), h.clone());
            }
            if let Some(l) = &ledger {
                row.texts.insert("ledger".to_owned(), l.clone());
            }
        }
        (Src::Rows, DataSource::Finding { finding: f.clone() })
    } else {
        (Src::Nothing, DataSource::Constant {})
    };
    let cx = Ctx {
        fr,
        id,
        layer,
        src,
        all_rows,
        constants,
    };
    let enc = &layer.encoding;
    let opacity = enc.opacity.unwrap_or_else(|| default_opacity(layer.mark));
    if !(0.0..=1.0).contains(&opacity) {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/opacity",
            "opacity must be in 0..=1",
        ));
    }
    if let Some(w) = enc.width {
        if !(w.is_finite() && w > 0.0) {
            return Err(cx.err(ErrorKind::Invalid, "/encoding/width", "width must be > 0"));
        }
    }
    let x2_given = enc.x2.is_some() || cx.constants.contains_key("/encoding/x2");
    let x_ok = match layer.mark {
        Mark::Rule => true,
        Mark::Band | Mark::Area => x2_given,
        Mark::Point => !matches!(cx.src, Src::Rows | Src::Slots(_)),
        Mark::Text | Mark::Glyph => is_panel(enc.x.as_ref()),
        _ => false,
    };
    if enc.x.is_some() && !x_ok {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/x",
            "`x` is used by `rule` (vertical rule, or with `x2` a segment), by `band`/`area` with `x2` (a bar extent), by a data-less `point` (its bar) and, panel-relative, by `text` and `glyph`; other marks are placed at their row's bar",
        ));
    }
    if x2_given && !matches!(layer.mark, Mark::Rule | Mark::Band | Mark::Area) {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/x2",
            "`x2` (the last bar of an extent) is used by `rule`, `band` and `area`",
        ));
    }
    let extent = if x2_given && layer.mark != Mark::Rule {
        let a = cx
            .bar_const("/encoding/x", enc.x.as_ref())?
            .ok_or_else(|| {
                cx.err(
                    ErrorKind::Invalid,
                    "/encoding/x",
                    "`x2` needs `x` (the first bar)",
                )
            })?;
        let b = cx.bar_const("/encoding/x2", enc.x2.as_ref())?.unwrap_or(a);
        Some((a.min(b), a.max(b)))
    } else {
        None
    };
    let mut anchor = Anchor::default();
    let mut glyphs: Vec<GlyphSpec> = Vec::new();
    let mut seg_extent: Option<(u64, u64)> = None;
    let label = layer.label.clone().or_else(|| {
        layer
            .series
            .as_ref()
            .map(|s| fr.p.label(s).unwrap_or_else(|| s.clone()))
    });
    let theme = &fr.theme;
    let mut skipped = 0u64;
    let mut keys: Vec<String> = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut traces: Vec<Trace> = Vec::new();
    let geom;
    let mut width = enc.width.unwrap_or(1.0);

    fn main_or_err<'f>(cx: &Ctx<'f, '_>, what: &str) -> Result<(Chan<'f>, String), GrammarError> {
        match cx.position(cx.layer.encoding.y.as_ref(), "/encoding/y", "y")? {
            Some(c) => Ok(c),
            None => {
                let m = cx.main().ok_or_else(|| {
                    cx.err(
                        ErrorKind::Invalid,
                        "/encoding/y",
                        format!("`{what}` needs `y`"),
                    )
                })?;
                Ok((cx.field(&m, "/encoding/y")?, m))
            }
        }
    }
    let mut next_palette = |rule_given: bool| {
        if rule_given {
            None
        } else {
            let c = theme.palette(*palette);
            *palette += 1;
            Some(c)
        }
    };

    match layer.mark {
        Mark::Candle => {
            geom = Geom::Candle;
            let Src::Slots(series) = cx.src else {
                return Err(cx.err(ErrorKind::Invalid, "/mark", "`candle` needs a bar series"));
            };
            if fr.kind(series) != Some(SeriesKind::Bars) {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    "/series",
                    format!("`candle` needs a bar series; `{series}` is not one"),
                ));
            }
            let cols: Vec<Column<'_>> = ["o", "h", "l", "c"]
                .iter()
                .map(|f| fr.column(series, f, id, "/series"))
                .collect::<Result<_, _>>()?;
            let rule = cx.color_rule(ColorRule::Candle {
                up: theme.up,
                down: theme.down,
            })?;
            let mut rows = Vec::new();
            for (s0, n) in fr.buckets() {
                let (mut o, mut h, mut l, mut c) = (None, None::<f64>, None::<f64>, None);
                for s in s0..s0 + n {
                    let (vo, vh, vl, vc) = (
                        cols[0].get(s),
                        cols[1].get(s),
                        cols[2].get(s),
                        cols[3].get(s),
                    );
                    if let (Some(vo), Some(vh), Some(vl), Some(vc)) = (vo, vh, vl, vc) {
                        if o.is_none() {
                            o = Some(vo);
                        }
                        h = Some(h.map_or(vh, |x| x.max(vh)));
                        l = Some(l.map_or(vl, |x| x.min(vl)));
                        c = Some(vc);
                    } else if s < fr.n_bars() {
                        skipped += 1;
                    }
                }
                let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) else {
                    continue;
                };
                let last = s0 + n - 1;
                let (color, ckey) = rule.at(last, None, Some((o, c)));
                rows.push(PRow {
                    slot: s0,
                    n,
                    t: fr.slot_time(s0),
                    v: [Some(o), Some(h), Some(l), Some(c)],
                    color: with_opacity(color, opacity),
                    ckey,
                    size: 0.0,
                    text: None,
                });
            }
            keys = vec!["o".into(), "h".into(), "l".into(), "c".into()];
            fields.clone_from(&keys);
            traces.push(Trace {
                rows,
                split: Split::None,
                base: with_opacity(theme.up, opacity),
                lod_line: false,
            });
        }
        Mark::Line => {
            geom = Geom::Line;
            width = enc.width.unwrap_or(1.5);
            let chans: Vec<(Chan<'_>, String)> =
                match cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
                    Some(c) => vec![c],
                    None => match cx.src {
                        Src::Slots(series) => {
                            let pf = fr.plot_fields(series);
                            pf.into_iter()
                                .map(|f| Ok((cx.field(&f, "/encoding/y")?, f)))
                                .collect::<Result<_, GrammarError>>()?
                        }
                        _ => vec![main_or_err(&cx, "line")?],
                    },
                };
            for (chan, key) in chans {
                let default = match next_palette(enc.color.is_some()) {
                    Some(c) => ColorRule::Const(c),
                    None => ColorRule::Const(theme.accent),
                };
                let rule = cx.color_rule(default)?;
                let (rows, sk, lod_line) = series_rows(&cx, &chan, &rule, opacity, RowAgg::Line);
                skipped += sk;
                let base = rule.at(fr.to, None, None).0;
                traces.push(Trace {
                    rows,
                    split: rule.split(opacity),
                    base,
                    lod_line,
                });
                fields.push(key.clone());
                keys.push(key);
            }
        }
        Mark::Area | Mark::Band | Mark::Bar => {
            let (y, ykey) = if layer.mark == Mark::Band {
                cx.position(enc.y.as_ref(), "/encoding/y", "y")?
                    .ok_or_else(|| cx.err(ErrorKind::Invalid, "/encoding/y", "`band` needs `y`"))?
            } else {
                main_or_err(
                    &cx,
                    if layer.mark == Mark::Bar {
                        "bar"
                    } else {
                        "area"
                    },
                )?
            };
            let y2 = cx.position(enc.y2.as_ref(), "/encoding/y2", "y2")?;
            if layer.mark == Mark::Band && y2.is_none() {
                return Err(cx.err(ErrorKind::Invalid, "/encoding/y2", "`band` needs `y2`"));
            }
            let (y2, y2key, base_default) = match y2 {
                Some((c, k)) => (Some(c), Some(k), false),
                None if layer.mark == Mark::Bar => {
                    (Some(Chan::Const(0.0)), Some("base".into()), true)
                }
                None => (None, None, false),
            };
            let _ = base_default;
            geom = match layer.mark {
                Mark::Area => Geom::Area,
                Mark::Band => Geom::Band,
                _ => Geom::Bar,
            };
            let default = if layer.mark == Mark::Bar {
                ColorRule::Const(theme.muted)
            } else {
                ColorRule::Const(next_palette(enc.color.is_some()).unwrap_or(theme.accent))
            };
            let rule = cx.color_rule(default)?;
            let agg = match layer.mark {
                Mark::Bar => RowAgg::MaxAbs,
                _ => RowAgg::Envelope,
            };
            let (rows, sk) = pair_rows(&cx, &y, y2.as_ref(), &rule, opacity, agg, extent);
            skipped += sk;
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: if layer.mark == Mark::Bar {
                    Split::None
                } else {
                    rule.split(opacity)
                },
                base,
                lod_line: false,
            });
            fields.push(ykey.clone());
            keys.push(ykey);
            if let Some(k) = y2key {
                if !(layer.mark == Mark::Bar && k == "base") {
                    fields.push(k.clone());
                }
                keys.push(k);
            }
        }
        Mark::Rule => {
            let rule = cx.color_rule(ColorRule::Const(theme.muted))?;
            let mut rows = Vec::new();
            let mk = |slot: u64, v: Option<f64>, row: Option<&Row>| {
                let (color, ckey) = rule.at(slot, row, None);
                PRow {
                    slot,
                    n: 1,
                    t: fr.slot_time(slot),
                    v: [v, None, None, None],
                    color: with_opacity(color, opacity),
                    ckey,
                    size: 0.0,
                    text: None,
                }
            };
            if x2_given {
                // A segment from bar `x` to bar `x2` at `y` (sloped to `y2`): evidence
                // geometry (R3). Every value is a constant with an origin.
                geom = Geom::Segment;
                let a = cx
                    .bar_const("/encoding/x", enc.x.as_ref())?
                    .ok_or_else(|| {
                        cx.err(
                            ErrorKind::Invalid,
                            "/encoding/x",
                            "`x2` needs `x` (the first bar)",
                        )
                    })?;
                let b = cx.bar_const("/encoding/x2", enc.x2.as_ref())?.unwrap_or(a);
                let konst = |ptr: &str, pos: Option<&Position>, key: &str| match cx
                    .position(pos, ptr, key)?
                {
                    Some((Chan::Const(v), _)) => Ok(Some(v)),
                    None => Ok(None),
                    Some(_) => Err(cx.err(
                        ErrorKind::Invalid,
                        ptr,
                        "a segment's `y`/`y2` are constants with an origin",
                    )),
                };
                let y0 = konst("/encoding/y", enc.y.as_ref(), "y")?.ok_or_else(|| {
                    cx.err(ErrorKind::Invalid, "/encoding/y", "a segment needs `y`")
                })?;
                let y1 = konst("/encoding/y2", enc.y2.as_ref(), "y2")?;
                let mut row = mk(a, Some(y0), None);
                row.v[1] = y1;
                rows.push(row);
                seg_extent = Some((a, b));
                keys.push("y".into());
                if y1.is_some() {
                    keys.push("y2".into());
                }
            } else if let Some((y, key)) = cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
                geom = Geom::RuleH;
                match (&y, &cx.src) {
                    (Chan::Const(v), _) => rows.push(mk(fr.to, Some(*v), None)),
                    (_, Src::Slots(_)) => {
                        // A rule over series data: one rule at the last visible value.
                        if let Some((s, v)) = (fr.from..=fr.to)
                            .rev()
                            .find_map(|s| y.at(s, None).map(|v| (s, v)))
                        {
                            rows.push(mk(s, Some(v), None));
                        }
                    }
                    _ => {
                        for r in cx.visible_rows() {
                            match y.at(r.slot, Some(r)) {
                                Some(v) => rows.push(mk(r.slot, Some(v), Some(r))),
                                None => skipped += 1,
                            }
                        }
                    }
                }
                if !matches!(y, Chan::Const(_)) {
                    fields.push(key.clone());
                }
                keys.push(key);
            } else if let Some((x, key)) = cx.position(enc.x.as_ref(), "/encoding/x", "x")? {
                geom = Geom::RuleV;
                let by_bar = matches!(
                    cx.constants.get("/encoding/x").map(|c| &c.origin),
                    Some(Origin::Fact { field, .. }) if field == "bar"
                ) || key == "bar";
                let to_slot = |v: f64| -> Option<u64> {
                    if by_bar {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        (v >= 0.0).then(|| v.round() as u64)
                    } else {
                        fr.time_slot(v)
                    }
                };
                match (&x, &cx.src) {
                    (Chan::Const(v), _) => {
                        if let Some(s) = to_slot(*v).filter(|s| *s >= fr.from && *s <= fr.to) {
                            rows.push(mk(s, Some(*v), None));
                        }
                    }
                    (_, Src::Rows) => {
                        for r in cx.visible_rows() {
                            if let Some(s) = x.at(r.slot, Some(r)).and_then(to_slot) {
                                if s >= fr.from && s <= fr.to {
                                    rows.push(mk(s, x.at(r.slot, Some(r)), Some(r)));
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(cx.err(
                            ErrorKind::Invalid,
                            "/encoding/x",
                            "a vertical rule's `x` is a constant or a fact/finding field",
                        ))
                    }
                }
                keys.push(key);
            } else if matches!(cx.src, Src::Rows) {
                geom = Geom::RuleV;
                for r in cx.visible_rows() {
                    rows.push(mk(r.slot, None, Some(r)));
                }
                keys.push("x".into());
            } else {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    "/encoding",
                    "`rule` needs `y` (horizontal) or `x` (vertical), or fact/finding data",
                ));
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: Split::None,
                base,
                lod_line: false,
            });
        }
        Mark::Text if is_panel(enc.x.as_ref()) || is_panel(enc.y.as_ref()) => {
            geom = Geom::Text;
            anchor = Anchor {
                x: panel_pos(enc.x.as_ref()),
                y: panel_pos(enc.y.as_ref()),
            };
            if !matches!(cx.src, Src::Rows) {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    "/encoding/text",
                    "`text` reads a text field of a fact or finding",
                ));
            }
            let f = text_field_of(&cx)?;
            let y = if anchor.y.is_some() {
                None
            } else {
                Some(main_or_err(&cx, "text")?)
            };
            let size = match &enc.size {
                None => 11.0,
                Some(SizeSpec::Constant(s)) if s.is_finite() && *s > 0.0 => *s,
                Some(_) => {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        "/encoding/size",
                        "a panel-positioned `text` takes a constant size > 0",
                    ))
                }
            };
            let rule = cx.color_rule(ColorRule::Const(theme.text))?;
            let mut rows = Vec::new();
            // A panel-relative `x` has no bar: the layer draws its first row only.
            let src_rows: Vec<&Row> = if anchor.x.is_some() {
                cx.all_rows.iter().take(1).collect()
            } else {
                cx.visible_rows().collect()
            };
            for r in src_rows {
                let v = match &y {
                    Some((c, _)) => match c.at(r.slot, Some(r)) {
                        Some(v) => Some(v),
                        None => {
                            skipped += 1;
                            continue;
                        }
                    },
                    None => None,
                };
                let (color, ckey) = rule.at(r.slot, Some(r), None);
                rows.push(PRow {
                    slot: r.slot,
                    n: 1,
                    t: fr.slot_time(r.slot),
                    v: [v, None, None, None],
                    color: with_opacity(color, opacity),
                    ckey,
                    size,
                    text: r.texts.get(&f).cloned(),
                });
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: Split::None,
                base,
                lod_line: false,
            });
            if let Some((_, k)) = y {
                fields.push(k.clone());
                keys.push(k);
            }
            fields.push(f);
        }
        Mark::Glyph => {
            geom = Geom::Glyph;
            let Some(finding) = &layer.finding else {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    if layer.fact.is_some() || layer.series.is_some() {
                        "/mark"
                    } else {
                        "/finding"
                    },
                    "`glyph` draws the capsule of a finding: bind the layer to its `finding`",
                ));
            };
            for (bad, set) in [
                ("/encoding/color", enc.color.is_some()),
                ("/encoding/opacity", enc.opacity.is_some()),
                ("/encoding/width", enc.width.is_some()),
                ("/encoding/y2", enc.y2.is_some()),
                ("/encoding/text", enc.text.is_some()),
            ] {
                if set {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        bad,
                        "a glyph takes only `x`, `y` and `size`; its colours come from the theme and its other channels from the capsule",
                    ));
                }
            }
            let spec = fr.p.glyph(finding).ok_or_else(|| {
                cx.err(
                    ErrorKind::NotFound,
                    "/finding",
                    format!("finding `{finding}` has no meaning glyph (no R3 capsule with a story presents it)"),
                )
            })?;
            let size = match &enc.size {
                None => GLYPH_SIZE,
                Some(SizeSpec::Constant(s)) if s.is_finite() && *s > 0.0 => *s,
                Some(_) => {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        "/encoding/size",
                        "a glyph takes a constant size > 0 (pixels)",
                    ))
                }
            };
            // `y` absent: at the top of the panel (a glyph strip over the chart).
            anchor = Anchor {
                x: panel_pos(enc.x.as_ref()),
                y: match &enc.y {
                    None => Some(PanelPos {
                        panel: 0.0,
                        px: 2.0,
                    }),
                    p => panel_pos(p.as_ref()),
                },
            };
            let y = if anchor.y.is_some() {
                None
            } else {
                cx.position(enc.y.as_ref(), "/encoding/y", "y")?
            };
            let mut rows = Vec::new();
            if let Some(r) = cx.all_rows.first() {
                let visible = anchor.x.is_some() || (r.slot >= fr.from && r.slot <= fr.to);
                let v = y.as_ref().and_then(|(c, _)| c.at(r.slot, Some(r)));
                if visible && (y.is_none() || v.is_some()) {
                    rows.push(PRow {
                        slot: r.slot,
                        n: 1,
                        t: fr.slot_time(r.slot),
                        v: [v, None, None, None],
                        color: [0; 4],
                        ckey: None,
                        size,
                        text: None,
                    });
                    glyphs.push(spec.into_owned());
                } else {
                    skipped += 1;
                }
            }
            traces.push(Trace {
                rows,
                split: Split::None,
                base: theme.text,
                lod_line: false,
            });
            fields.push("glyph".to_owned());
            if let Some((_, k)) = y {
                fields.push(k.clone());
                keys.push(k);
            }
        }
        Mark::Point | Mark::Text => {
            geom = if layer.mark == Mark::Point {
                Geom::Point
            } else {
                Geom::Text
            };
            let (y, ykey) = main_or_err(
                &cx,
                if layer.mark == Mark::Point {
                    "point"
                } else {
                    "text"
                },
            )?;
            let text_field = if layer.mark == Mark::Text {
                let f = enc.text.clone().or_else(|| match &layer.finding {
                    Some(_) => Some("claim".to_owned()),
                    None => layer.fact.as_ref().map(|_| "text".to_owned()),
                });
                let f = f.ok_or_else(|| {
                    cx.err(
                        ErrorKind::Invalid,
                        "/encoding/text",
                        "`text` needs the `text` channel",
                    )
                })?;
                if !matches!(cx.src, Src::Rows) {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        "/encoding/text",
                        "in v0 `text` reads a text field of a fact or finding",
                    ));
                }
                if !cx.all_rows.iter().any(|r| r.texts.contains_key(&f)) {
                    return Err(cx.err(
                        ErrorKind::NotFound,
                        "/encoding/text",
                        format!(
                            "text field `{f}` not found (fields: text{})",
                            if layer.finding.is_some() {
                                ", claim"
                            } else {
                                ""
                            }
                        ),
                    ));
                }
                Some(f)
            } else {
                None
            };
            let default = if layer.mark == Mark::Point {
                theme.accent
            } else {
                theme.text
            };
            let rule = cx.color_rule(ColorRule::Const(default))?;
            let default_size = if layer.mark == Mark::Point { 6.0 } else { 11.0 };
            let size_chan = match &enc.size {
                None => None,
                Some(SizeSpec::Constant(s)) => {
                    if !(s.is_finite() && *s > 0.0) {
                        return Err(cx.err(
                            ErrorKind::Invalid,
                            "/encoding/size",
                            "size must be > 0",
                        ));
                    }
                    None
                }
                Some(SizeSpec::ByField(b)) => {
                    Some((cx.field(&b.field, "/encoding/size/field")?, b.range))
                }
            };
            let const_size = match &enc.size {
                Some(SizeSpec::Constant(s)) => *s,
                _ => default_size,
            };
            let mut rows = Vec::new();
            let mut push = |slot: u64, n: u64, row: Option<&Row>, skipped: &mut u64| {
                let Some(v) = y.at(slot, row) else {
                    *skipped += 1;
                    return;
                };
                let (color, ckey) = rule.at(slot, row, None);
                let sv = size_chan.as_ref().and_then(|(c, _)| c.at(slot, row));
                rows.push((
                    PRow {
                        slot,
                        n,
                        t: fr.slot_time(slot),
                        v: [Some(v), None, None, None],
                        color: with_opacity(color, opacity),
                        ckey,
                        size: const_size,
                        text: text_field
                            .as_ref()
                            .and_then(|f| row.and_then(|r| r.texts.get(f).cloned())),
                    },
                    sv,
                ));
            };
            match cx.src {
                Src::Slots(_) => {
                    for (s0, n) in fr.buckets() {
                        // Last valid value of the bucket.
                        let s = (s0..s0 + n).rev().find(|s| y.at(*s, None).is_some());
                        match s {
                            Some(s) => push(s, n, None, &mut skipped),
                            None => skipped += n.min(fr.n_bars().saturating_sub(s0)),
                        }
                    }
                }
                Src::Rows => {
                    for r in cx.visible_rows() {
                        push(r.slot, 1, Some(r), &mut skipped);
                    }
                }
                Src::Nothing => {
                    // A data-less point sits on the bar its `x` constant names (R3
                    // evidence geometry: a swing point).
                    match cx.bar_const("/encoding/x", enc.x.as_ref())? {
                        Some(s) if layer.mark == Mark::Point => {
                            if s >= fr.from && s <= fr.to {
                                push(s, 1, None, &mut skipped);
                            }
                        }
                        _ => {
                            if let Chan::Const(_) = y {
                                return Err(cx.err(
                                    ErrorKind::Invalid,
                                    "",
                                    "`point`/`text` without data has no bar to sit on (a `point` takes its bar from an `x` constant)",
                                ));
                            }
                        }
                    }
                }
            }
            if let Some((_, range)) = &size_chan {
                let vals: Vec<f64> = rows.iter().filter_map(|(_, s)| *s).collect();
                let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                for (r, s) in &mut rows {
                    r.size = match *s {
                        Some(s) if hi > lo => {
                            range[0] + (s - lo) / (hi - lo) * (range[1] - range[0])
                        }
                        Some(_) => range[0],
                        None => const_size,
                    }
                    .max(1.0);
                }
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows: rows.into_iter().map(|(r, _)| r).collect(),
                split: Split::None,
                base,
                lod_line: false,
            });
            fields.push(ykey.clone());
            keys.push(ykey);
            if let Some(f) = text_field {
                fields.push(f);
            }
        }
    }
    Ok(Prepared {
        id: id.to_owned(),
        mark: layer.mark,
        panel: layer.panel,
        data,
        fields,
        constants: cx.constants,
        label,
        geom,
        traces,
        keys,
        width,
        edge: enc.width.is_some(),
        skipped,
        has_data: n_data == 1,
        anchor,
        extent: seg_extent,
        glyphs,
    })
}

/// The text field of a `text` layer: the `text` channel, else `claim` (finding) / `text`
/// (fact); it must exist in the layer's rows.
fn text_field_of(cx: &Ctx<'_, '_>) -> Result<String, GrammarError> {
    let layer = cx.layer;
    let f = layer
        .encoding
        .text
        .clone()
        .or_else(|| match &layer.finding {
            Some(_) => Some("claim".to_owned()),
            None => layer.fact.as_ref().map(|_| "text".to_owned()),
        });
    let f = f.ok_or_else(|| {
        cx.err(
            ErrorKind::Invalid,
            "/encoding/text",
            "`text` needs the `text` channel",
        )
    })?;
    if !cx.all_rows.iter().any(|r| r.texts.contains_key(&f)) {
        let mut names: Vec<&str> = Vec::new();
        for r in &cx.all_rows {
            for k in r.texts.keys() {
                if !names.contains(&k.as_str()) {
                    names.push(k);
                }
            }
        }
        return Err(cx.err(
            ErrorKind::NotFound,
            "/encoding/text",
            format!("text field `{f}` not found (fields: {})", names.join(", ")),
        ));
    }
    Ok(f)
}

#[derive(Clone, Copy)]
enum RowAgg {
    Line,
    Envelope,
    MaxAbs,
}

/// Rows of a single-valued channel (lines). Returns rows, skipped count, LOD-line flag.
fn series_rows(
    cx: &Ctx<'_, '_>,
    y: &Chan<'_>,
    rule: &ColorRule<'_>,
    opacity: f64,
    _agg: RowAgg,
) -> (Vec<PRow>, u64, bool) {
    let fr = cx.fr;
    let mut rows = Vec::new();
    let mut skipped = 0u64;
    let mk = |slot: u64, n: u64, v: [Option<f64>; 4], at: u64, row: Option<&Row>| {
        let (color, ckey) = rule.at(at, row, None);
        PRow {
            slot,
            n,
            t: fr.slot_time(slot),
            v,
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        }
    };
    match cx.src {
        Src::Rows => {
            for r in cx.visible_rows() {
                match y.at(r.slot, Some(r)) {
                    Some(v) => {
                        rows.push(mk(r.slot, 1, [Some(v), None, None, None], r.slot, Some(r)))
                    }
                    None => skipped += 1,
                }
            }
            (rows, skipped, false)
        }
        _ if fr.k <= 1 => {
            for s in fr.from..=fr.to {
                match y.at(s, None) {
                    Some(v) => rows.push(mk(s, 1, [Some(v), None, None, None], s, None)),
                    None => {
                        if y.has(s) {
                            skipped += 1;
                        }
                        rows.push(mk(s, 1, [None; 4], s, None));
                    }
                }
            }
            (rows, skipped, false)
        }
        _ => {
            for (s0, n) in fr.buckets() {
                let mut first = None;
                let mut last = None;
                let mut lo: Option<(u64, f64)> = None;
                let mut hi: Option<(u64, f64)> = None;
                let mut last_slot = s0;
                for s in s0..s0 + n {
                    match y.at(s, None) {
                        Some(v) => {
                            if first.is_none() {
                                first = Some(v);
                            }
                            last = Some(v);
                            last_slot = s;
                            if lo.is_none_or(|(_, x)| v < x) {
                                lo = Some((s, v));
                            }
                            if hi.is_none_or(|(_, x)| v > x) {
                                hi = Some((s, v));
                            }
                        }
                        None => {
                            if y.has(s) {
                                skipped += 1;
                            }
                        }
                    }
                }
                let v = match (first, lo, hi, last) {
                    (Some(f), Some((ls, l)), Some((hs, h)), Some(z)) => {
                        let (a, b) = if ls <= hs { (l, h) } else { (h, l) };
                        [Some(f), Some(a), Some(b), Some(z)]
                    }
                    _ => [None; 4],
                };
                rows.push(mk(s0, n, v, last_slot, None));
            }
            (rows, skipped, true)
        }
    }
}

/// Rows of a two-valued mark (area, band, bar): `v = [y, y2]`.
fn pair_rows(
    cx: &Ctx<'_, '_>,
    y: &Chan<'_>,
    y2: Option<&Chan<'_>>,
    rule: &ColorRule<'_>,
    opacity: f64,
    agg: RowAgg,
    extent: Option<(u64, u64)>,
) -> (Vec<PRow>, u64) {
    let inside = |s: u64| extent.is_none_or(|(a, b)| s >= a && s <= b);
    let fr = cx.fr;
    let mut rows = Vec::new();
    let mut skipped = 0u64;
    let both = |s: u64, row: Option<&Row>| -> Option<(f64, Option<f64>)> {
        let a = y.at(s, row)?;
        match y2 {
            Some(c) => c.at(s, row).map(|b| (a, Some(b))),
            None => Some((a, None)),
        }
    };
    let mk = |slot: u64, n: u64, v: Option<(f64, Option<f64>)>, at: u64, row: Option<&Row>| {
        let (color, ckey) = rule.at(at, row, None);
        PRow {
            slot,
            n,
            t: fr.slot_time(slot),
            v: match v {
                Some((a, b)) => [Some(a), b, None, None],
                None => [None; 4],
            },
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        }
    };
    let is_bar = matches!(agg, RowAgg::MaxAbs);
    match cx.src {
        Src::Rows => {
            for r in cx.visible_rows().filter(|r| inside(r.slot)) {
                match both(r.slot, Some(r)) {
                    Some(v) => rows.push(mk(r.slot, 1, Some(v), r.slot, Some(r))),
                    None => skipped += 1,
                }
            }
        }
        _ => {
            for (s0, n) in fr.buckets() {
                let mut acc: Option<(f64, Option<f64>)> = None;
                let mut at = s0;
                if extent.is_some() && !(s0..s0 + n).any(inside) {
                    continue;
                }
                for s in (s0..s0 + n).filter(|s| inside(*s)) {
                    match both(s, None) {
                        Some((a, b)) => {
                            acc = Some(match (acc, agg) {
                                (None, _) => (a, b),
                                (Some((pa, pb)), RowAgg::MaxAbs) => {
                                    if a.abs() > pa.abs() {
                                        at = s;
                                        (a, b)
                                    } else {
                                        (pa, pb)
                                    }
                                }
                                (Some((pa, pb)), _) => {
                                    at = s;
                                    (
                                        pa.max(a),
                                        match (pb, b) {
                                            (Some(x), Some(y)) => Some(x.min(y)),
                                            _ => None,
                                        },
                                    )
                                }
                            });
                            if acc.is_some_and(|_| n == 1) {
                                at = s;
                            }
                        }
                        None => {
                            if y.has(s) || y2.is_some_and(|c| c.has(s)) {
                                skipped += 1;
                            }
                        }
                    }
                }
                if acc.is_none() && is_bar {
                    continue;
                }
                rows.push(mk(s0, n, acc, at, None));
            }
        }
    }
    (rows, skipped)
}
