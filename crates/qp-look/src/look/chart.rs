//! (RD2/S1) A look's profile, its origin colour roles and its chart conventions
//! (`docs/design/CONTRACT.md`). Legacy looks carry values that change no scene.

use super::*;

/// The density profile a look is designed for (`docs/design/CONTRACT.md` "Yoğunluk").
/// A design direction registers two looks: `<name>` ([`Profile::Dense`], desktop) and
/// `<name>-tel` ([`Profile::Touch`], phone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// A look from before RD2 (`r4`, `ornek`, `oltan`, `zemre`): `scripts/design-check`
    /// reports it for comparison and never fails on it.
    Legacy,
    /// Desktop: body 13–14 px, chart figures 11–12 px, pointer targets.
    Dense,
    /// Phone: body ≥ 15 px, smallest text ≥ 13 px, 44 px touch targets.
    Touch,
}

impl Profile {
    /// The name (`legacy`, `dense`, `touch`; `--qp-profile`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Profile::Legacy => "legacy",
            Profile::Dense => "dense",
            Profile::Touch => "touch",
        }
    }
}

/// Who wrote a mark, as a colour each (köken: insan / ajan / motor), in one scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginRoles {
    /// A mark a person drew or approved.
    pub human: Rgba,
    /// A mark an agent proposed.
    pub agent: Rgba,
    /// A mark the engine computed (a fact, a study).
    pub engine: Rgba,
}

/// A look's own origin roles; a look without them ([`Look::origin`] `None`) derives them
/// from its scheme ([`Look::origin_roles`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginLook {
    /// Dark scheme.
    pub dark: OriginRoles,
    /// Light scheme.
    pub light: OriginRoles,
    /// Dash and gap of an agent-authored line (px); `None`: solid.
    pub agent_dash: Option<[f64; 2]>,
}

/// The value line at the last value across the plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLine {
    /// None (legacy).
    None,
    /// A dotted line in the last value's colour.
    Dotted,
}

/// (RD2/S1) The chart's familiar conventions (the "alphabet": right price axis, two-level
/// time axis, last-value tag, value line, crosshair and axis pills, OHLC legend, volume
/// pane). Recorded here; the compiler reads them from RD2/C2 on. [`ChartLook::legacy`]
/// is today's behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartLook {
    /// Target spacing of price-axis labels (CSS px; legacy 90: `height / 90` divisions).
    pub price_label_px: f64,
    /// Two-level time axis (minor unit, major unit emphasised); legacy: months only.
    pub two_level_time: bool,
    /// Last-value tag on the price axis (legacy: yes).
    pub last_value_tag: bool,
    /// Value line at the last value.
    pub value_line: ValueLine,
    /// Crosshair with axis pills (date and price) under the pointer.
    pub crosshair_pills: bool,
    /// OHLC legend in the title row (legacy: the close only).
    pub ohlc_legend: bool,
    /// Share of the chart height of a volume sub-pane (0: none; the view decides).
    pub volume_share: f64,
    /// Greatest height of a semantic mark on the plot (px; legacy: the chart glyph).
    pub mark_max_px: f64,
    /// Candle body width as a share of the slot (legacy 0.7).
    pub body_ratio: f64,
    /// Width of the right axis gutter (px; legacy: the right plot margin).
    pub axis_gutter_px: f64,
    /// (RD2/C2b) A horizontal level takes its origin's colour and stroke (human solid,
    /// agent dashed, engine dotted) and ends in a price-axis tag (legacy: no).
    pub origin_levels: bool,
}

impl ChartLook {
    /// Today's chart for a look whose right plot margin is `gutter` and whose chart glyph
    /// is `mark_px` tall.
    #[must_use]
    pub const fn legacy(gutter: f64, mark_px: f64) -> Self {
        ChartLook {
            price_label_px: 90.0,
            two_level_time: false,
            last_value_tag: true,
            value_line: ValueLine::None,
            crosshair_pills: false,
            ohlc_legend: false,
            volume_share: 0.0,
            mark_max_px: mark_px,
            body_ratio: 0.7,
            axis_gutter_px: gutter,
            origin_levels: false,
        }
    }

    pub(super) fn to_json(self) -> Value {
        json!({"price_label_px": self.price_label_px, "two_level_time": self.two_level_time,
            "last_value_tag": self.last_value_tag,
            "value_line": match self.value_line { ValueLine::None => "none",
                ValueLine::Dotted => "dotted" },
            "crosshair_pills": self.crosshair_pills, "ohlc_legend": self.ohlc_legend,
            "volume_share": self.volume_share, "mark_max_px": self.mark_max_px,
            "body_ratio": self.body_ratio, "axis_gutter_px": self.axis_gutter_px,
            "origin_levels": self.origin_levels})
    }
}

impl Look {
    /// (RD2/C2a) Whether the compiler draws the [`ChartLook`] (non-legacy looks only).
    #[must_use]
    pub fn familiar(&self) -> bool {
        self.profile != Profile::Legacy
    }

    /// (RD2/C2a, P8) An on-plot mark asked `px` tall, capped at `chart.mark_max_px`.
    #[must_use]
    pub fn mark_px(&self, px: f64) -> f64 {
        if self.familiar() {
            px.min(self.chart.mark_max_px)
        } else {
            px
        }
    }

    /// The origin roles of `theme` (one of this look's schemes): the look's own, or
    /// derived (human = `text`, agent = `accent`, engine = `muted`).
    #[must_use]
    pub fn origin_roles(&self, theme: &Theme) -> OriginRoles {
        match self.origin {
            Some(o) if theme.name == "light" => o.light,
            Some(o) => o.dark,
            None => OriginRoles {
                human: theme.text,
                agent: theme.accent,
                engine: theme.muted,
            },
        }
    }

    /// Dash and gap of an agent-authored line: the look's own, or derived (the story
    /// chart's dash); `None`: solid.
    #[must_use]
    pub fn agent_dash(&self) -> Option<[f64; 2]> {
        match self.origin {
            Some(o) => o.agent_dash,
            None => Some([f64::from(self.story.dash.0), f64::from(self.story.dash.1)]),
        }
    }

    pub(super) fn origin_json(&self) -> Value {
        let h = |c: Rgba| format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3]);
        let roles = |t: &Theme| {
            let o = self.origin_roles(t);
            json!({"human": h(o.human), "agent": h(o.agent), "engine": h(o.engine)})
        };
        json!({"dark": roles(&self.dark), "light": roles(&self.light),
            "agent_dash": self.agent_dash(), "derived": self.origin.is_none()})
    }
}
