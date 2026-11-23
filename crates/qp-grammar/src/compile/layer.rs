//! Layer preparation: resolves a layer's data, channels, constants and colours into
//! value rows (LOD-aggregated when the frame says so). Pixel work happens in `draw`.

use std::collections::BTreeMap;

use qp_schema::capsule::GlyphSpec;
use qp_schema::grammar::{
    Cmp, ColorSpec, Layer, Mark, Operand, Origin, PanelPos, Position, SizeSpec,
};
use qp_schema::scene::{DataSource, ResolvedConstant, Rgba};

use crate::frame::{Column, Frame, Row};
use crate::provider::SeriesKind;
use qp_look::error::{ErrorKind, GrammarError};
use qp_look::theme::with_opacity;

mod prepare;
mod resolve;
mod rows;

pub(crate) use prepare::prepare;
use resolve::resolve_constants;
use rows::{pair_rows, series_rows, text_field_of, RowAgg};

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
    /// (R5/M20) A panel-positioned text's right wrap edge (`x2`, panel-relative; it may
    /// reach past the panel's right edge into the plot margin). `None`: the panel's right.
    pub x2: Option<PanelPos>,
}

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
                    written.map_or_else(String::new, qp_look::detmath::num)
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
                format!("{} is not a bar of the view", qp_look::detmath::num(v)),
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
