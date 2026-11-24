//! The presenter's depth entries that only forward to the named designs (faces, story
//! charts): split out of `qp_grammar::depth` in RF1 (`rounds/RF1/PLAN.md` §3.1). The depth
//! rules themselves stay in `qp_grammar::depth`.

use qp_grammar::DataProvider;
use qp_look::error::GrammarError;
use qp_look::look::Look;
use qp_schema::View;

// ---------------------------------------------------------------------------
// R4/T5: capsule face designs (`docs/PROTOCOL.md` §4.6.2)
// ---------------------------------------------------------------------------

/// Whether `name` is a known face design (`crate::faces`, R4/H-M8910: the registry with
/// each design's height; `card` is the earlier name of `r3`).
#[must_use]
pub fn is_face(name: &str) -> bool {
    crate::faces::is_face(name)
}

/// The face of a capsule in the feed (open depth `sentence`) by face design `face`
/// (`None`: the default, `r3` = the §18.4 capsule card [`qp_grammar::depth::capsule_view`] at `max_points`).
/// The designs are registered in [`crate::faces::FACES`].
///
/// # Errors
/// An unknown face name (`invalid`, `/face`) or the design's own errors.
pub fn face_view(
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    crate::faces::face_view(face, finding, max_points, p, id)
}

/// (R5/T3) [`face_view`] in `look`.
///
/// # Errors
/// As [`face_view`].
pub fn face_view_in(
    look: &Look,
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    crate::faces::face_view_in(look, face, finding, max_points, p, id)
}

// ---------------------------------------------------------------------------
// R5/M18: the story chart (`crate::stories`)
// ---------------------------------------------------------------------------

/// (R5/M18, K50) The story chart design a connection chose (`ViewState.story`), drawn at
/// `chart`/`evidence` depth around the open capsule's chart; `None` (the default, `r4`):
/// the open capsule's chart alone. The designs are registered in
/// [`crate::stories::STORIES`].
#[must_use]
pub fn story_design(name: Option<&str>) -> Option<&'static crate::stories::StoryDesign> {
    crate::stories::by_name(name)
}
