//! (R5/T3) The embedded font files, by asset id (`qp_grammar::look::FONTS` names the same
//! ids, with each file's family, weight and licence). A scene's text names its asset in
//! `Prim::Text::font` (absent: the default, index 0); the reference renderer, the canvas
//! paint path (`FontFace` from these bytes) and the GPU path all draw with these files,
//! so no output depends on a system font and the page asks the network for none.
//!
//! **Adding a file:** put it and its licence in `assets/fonts/`, add one line below and
//! one [`qp_grammar::look::FontAsset`] line (`look::FONTS`); `tests/fonts.rs` checks the
//! two lists agree.

use std::sync::OnceLock;

use crate::RenderError;

/// `(asset id, file bytes)`; index 0 is the default font.
pub const FILES: [(&str, &[u8]); 1] = [(
    "plex-mono",
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
)];

/// The index of asset `id` (`None` or an unknown id: 0, the default).
#[must_use]
pub fn index(id: Option<&str>) -> usize {
    id.and_then(|id| FILES.iter().position(|(n, _)| *n == id))
        .unwrap_or(0)
}

/// The bytes of asset `id`, if embedded.
#[must_use]
pub fn bytes(id: &str) -> Option<&'static [u8]> {
    FILES.iter().find(|(n, _)| *n == id).map(|(_, b)| *b)
}

/// The parsed font at `index` (out of range: the default), parsed once.
///
/// # Errors
/// Only if an embedded file is corrupt (a build-time invariant; `tests/fonts.rs`).
pub fn by_index(index: usize) -> Result<&'static fontdue::Font, RenderError> {
    static CACHE: [OnceLock<Result<fontdue::Font, String>>; FILES.len()] =
        [const { OnceLock::new() }; FILES.len()];
    let i = if index < FILES.len() { index } else { 0 };
    CACHE[i]
        .get_or_init(|| {
            fontdue::Font::from_bytes(FILES[i].1, fontdue::FontSettings::default())
                .map_err(str::to_owned)
        })
        .as_ref()
        .map_err(|e| RenderError::new(format!("embedded font `{}`: {e}", FILES[i].0)))
}

/// The font a text prim with `font` field `id` is drawn with, and its index: `default`
/// (index 0) for `None`, an unknown id falls back to it too.
///
/// # Errors
/// As [`by_index`].
pub fn for_text<'f>(
    default: &'f fontdue::Font,
    id: Option<&str>,
) -> Result<(usize, &'f fontdue::Font), RenderError> {
    match index(id) {
        0 => Ok((0, default)),
        i => Ok((i, by_index(i)?)),
    }
}
