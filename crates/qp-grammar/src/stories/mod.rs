//! (R5/M18, K50, K54, K53) Story chart designs: what the chart depth draws for an
//! instrument's whole story list (`docs/PROTOCOL.md` §23), around the open (selected)
//! capsule's own chart.
//!
//! A design is chosen by name (`style` gesture `story`, `ViewState.story`, `?story=`);
//! the default (`r4`) draws nothing more than the open capsule's chart, so every existing
//! golden and transcript is unchanged. A design works in two steps both sides run alike
//! (the authority for `tap_at`/`hit`, the device for its pixels): the open capsule's chart
//! is compiled at [`StoryDesign::price_height`] of the frame, then
//! [`StoryDesign::extend`] adds the story part to that scene (pure: the scene, the rows,
//! the focus, the look). A tap on the story part is resolved by [`StoryDesign::tap`]
//! before the opening rule ([`crate::depth::tap_opens`]).
//!
//! (R5/S) A design may instead compile the whole scene itself from the view
//! ([`StoryDesign::compile`]; `korda`): then the chart is not compiled first and
//! `price_height`/`extend` are unused.

mod design;
pub mod korda;
pub mod pavka;

pub use design::{
    by_name, is_story, names, StampFn, StoryCompile, StoryDesign, StoryInput, StoryTap,
    DEFAULT_STORY, PRESENTED_STORY, STORIES,
};
