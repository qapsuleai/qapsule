//! The story design contract ([`StoryDesign`], [`StoryInput`], [`StoryTap`]) and the
//! registry of designs ([`STORIES`]).

use super::{korda, pavka};

use qp_schema::protocol::{HitItem, Language, StoryRow};
use qp_schema::scene::SceneDesc;

use qp_schema::grammar::View;

use crate::compile::CompileOptions;
use crate::provider::DataProvider;
use qp_look::error::GrammarError;
use qp_look::look::Look;

/// (RP4/T7) A story row's time stamp, if it has one ([`StoryInput::stamp_of`]).
pub type StampFn<'a> = dyn Fn(&StoryRow) -> Option<String> + 'a;

/// What a story design draws from.
pub struct StoryInput<'a> {
    /// The instrument's story rows as the reader received them (birth order).
    pub rows: &'a [StoryRow],
    /// The selected story: the open capsule's finding.
    pub focus: Option<&'a str>,
    /// (K54) Closed stories hidden (the selected one stays).
    pub hide_closed: bool,
    /// The reader's language (the design's own words; the rows' face texts are already
    /// in it).
    pub lang: Language,
    /// The first fact of a finding (the origin of a cluster's constants), if known.
    pub fact_of: &'a dyn Fn(&str) -> Option<String>,
    /// (RP4/T7) A row's time stamp ("14:35", "12 Kas"), when the connection asked rows to
    /// carry their time (`ViewState.clock`; the core computes it from the exchange
    /// calendar, `qp_capsule::realize::stamp`); `None`: rows as before.
    pub stamp_of: Option<&'a StampFn<'a>>,
}

/// What a tap on the story part of a scene does (resolved in the core).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryTap {
    /// Move the focus to this story (its finding): it becomes the open capsule.
    Focus(String),
    /// Show these bars (a cluster opened): the visible range becomes `[from, to]`.
    Zoom(u64, u64),
    /// Show or hide the closed stories (K54).
    ToggleClosed,
}

/// One story chart design.
pub struct StoryDesign {
    /// Name (`[a-z][a-z0-9]*`).
    pub name: &'static str,
    /// One line on the idea.
    pub note: &'static str,
    /// The height the open capsule's chart is compiled at, for a frame `h` high.
    pub price_height: fn(&Look, u32) -> u32,
    /// Adds the story part below/around the chart scene, making it `h` high.
    pub extend: fn(&Look, Option<&str>, SceneDesc, u32, &StoryInput<'_>) -> SceneDesc,
    /// The tap rule of the story part (`None`: not a story element; the opening rule
    /// applies).
    pub tap: fn(Option<&HitItem>, Option<&str>) -> Option<StoryTap>,
    /// (R5/S) The whole scene compiled by the design from the chart view (`None`: the
    /// chart is compiled at `price_height`, then `extend`ed).
    pub compile: Option<StoryCompile>,
}

/// (R5/S) A design that compiles the chart view and its stories into one scene.
pub type StoryCompile = fn(
    &View,
    &dyn DataProvider,
    &CompileOptions,
    &StoryInput<'_>,
) -> Result<SceneDesc, GrammarError>;

/// The default: no story chart (the open capsule's chart alone, as in R4).
pub const DEFAULT_STORY: &str = "r4";

/// (R5/S) The story chart the presentation shell asks for; the protocol default stays
/// [`DEFAULT_STORY`]. (R6/T0, K57) Arkin's pick `korda` (M18 had picked `pavka`;
/// `rounds/R5/experiments/M18/RESULTS.md`); `pavka` stays selectable (`?story=pavka`).
pub const PRESENTED_STORY: &str = "korda";

/// Registered designs (the default is not one: it draws nothing more).
pub static STORIES: &[StoryDesign] = &[pavka::DESIGN, korda::DESIGN];

/// Every name `story` accepts, the default first.
#[must_use]
pub fn names() -> Vec<&'static str> {
    std::iter::once(DEFAULT_STORY)
        .chain(STORIES.iter().map(|d| d.name))
        .collect()
}

/// Whether `name` is accepted.
#[must_use]
pub fn is_story(name: &str) -> bool {
    names().contains(&name)
}

/// The design named `name` (`None`, the default or an unknown name: none).
#[must_use]
pub fn by_name(name: Option<&str>) -> Option<&'static StoryDesign> {
    let n = name?;
    STORIES.iter().find(|d| d.name == n)
}
