//! (R5/M18, K50, K54, K53) Story chart designs: what chart depth draws when it shows
//! **every** story (capsule) of the open instrument in the visible range, not only the
//! open one. A design takes the presenter's chart view (`crate::depth::chart_view_in`:
//! candles, the open story's evidence geometry and glyph) and the instrument's story list
//! (`docs/PROTOCOL.md` §23, `StoryRow`) and compiles one scene (the single source of
//! truth: `ref` and `canvas` paint it, `screen` and `hit` read it).
//!
//! Selected by name like the face designs (`style` gesture / `ViewState.story`,
//! `?story=` on the page). The default (`r4`) is no story chart: today's chart of the
//! open capsule alone. The interaction a story chart offers is read back from the point
//! query by [`story_hit`] (the authority's `tap_at` applies it, the shell never decides).

pub mod korda;

use qp_schema::grammar::View;
use qp_schema::protocol::{HitItem, Language, StoryRow};
use qp_schema::scene::{DataSource, SceneDesc};

use crate::compile::CompileOptions;
use crate::error::GrammarError;
use crate::provider::DataProvider;

/// The default story chart setting: none (the chart of the open capsule alone, R4).
pub const DEFAULT_STORY: &str = "r4";

/// Layer id prefix of one story's lane (`story:<capsule id>`; data: its finding).
pub const STORY_LAYER: &str = "story:";
/// Layer id prefix of a cluster of stories (`story-cluster:<from bar>-<to bar>`).
pub const CLUSTER_LAYER: &str = "story-cluster:";
/// Layer id of the "closed stories shown / hidden" control.
pub const TOGGLE_LAYER: &str = "story-toggle";

/// What the story chart draws from: the instrument's story list as the reader received
/// it, the open capsule's finding and the reader's choices.
#[derive(Debug, Clone, Copy)]
pub struct StoryInput<'a> {
    /// The rows (any order; rows of another bar series are ignored).
    pub rows: &'a [StoryRow],
    /// Finding of the open (selected) capsule.
    pub selected: &'a str,
    /// Closed stories shown (K54: the default) or hidden.
    pub show_closed: bool,
    /// Language of the chart's own words (the rows carry theirs).
    pub lang: Language,
}

/// A story chart design.
pub struct StoryDesign {
    /// Setting name.
    pub name: &'static str,
    /// One line on the design's principle.
    pub note: &'static str,
    /// The scene: the chart view plus the stories, compiled.
    pub scene: fn(
        &View,
        &dyn DataProvider,
        &CompileOptions,
        &StoryInput<'_>,
    ) -> Result<SceneDesc, GrammarError>,
}

/// The registered designs (the default `r4` is none and has no entry).
pub static STORIES: &[StoryDesign] = &[StoryDesign {
    name: "korda",
    note: "lifelines: one time-aligned lane per story under the price, birth mark to outcome",
    scene: korda::scene,
}];

/// Setting names, the default first.
#[must_use]
pub fn names() -> Vec<&'static str> {
    std::iter::once(DEFAULT_STORY)
        .chain(STORIES.iter().map(|d| d.name))
        .collect()
}

/// Whether `name` is a known setting (the default included).
#[must_use]
pub fn is_story(name: &str) -> bool {
    names().contains(&name)
}

/// The design a setting names (`None`: the default, an unknown name or none set).
#[must_use]
pub fn design(name: Option<&str>) -> Option<&'static StoryDesign> {
    let n = name?;
    STORIES.iter().find(|d| d.name == n)
}

/// What a tap on a story chart asks for, read from the point query's item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryHit {
    /// Move the focus to this capsule (another story's lane or mark).
    Focus {
        /// Capsule id.
        capsule: String,
        /// Its finding.
        finding: String,
    },
    /// Zoom the chart to this bar range (a cluster).
    Zoom {
        /// First bar.
        from: u64,
        /// Last bar.
        to: u64,
    },
    /// Show or hide the closed stories.
    ToggleClosed,
}

/// The story chart's meaning of a hit item (`None`: not a story chart element).
#[must_use]
pub fn story_hit(item: &HitItem) -> Option<StoryHit> {
    if item.layer == TOGGLE_LAYER {
        return Some(StoryHit::ToggleClosed);
    }
    if let Some(rest) = item.layer.strip_prefix(CLUSTER_LAYER) {
        let (a, b) = rest.split_once('-')?;
        return Some(StoryHit::Zoom {
            from: a.parse().ok()?,
            to: b.parse().ok()?,
        });
    }
    let capsule = item.layer.strip_prefix(STORY_LAYER)?;
    match &item.data {
        DataSource::Finding { finding } => Some(StoryHit::Focus {
            capsule: capsule.to_owned(),
            finding: finding.clone(),
        }),
        _ => None,
    }
}
