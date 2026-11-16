//! R4/H-M8910: capsule face designs (M8; `docs/PROTOCOL.md` §4.6.2, the `style` gesture's
//! `face`).
//!
//! A face design is a grammar rule that builds the [`View`] of a capsule's face in the feed
//! (open depth `sentence`) and **states its own height** (CSS px at the phone width): the
//! shell lays faces out at the height the core reads from here, it never computes one.
//! The authority (`qp-session` `interact.rs`, hit testing and openings) and the shell's
//! replica (`qp-wasm` `shell.rs` `face_scene`) both call [`face_view`] by name, so they
//! compile the same scene.
//!
//! The reference `r3` is the R3 capsule card (§18.4, [`depth::capsule_view`]) at 240 px,
//! the T5 shell's face. `card` is its earlier name (accepted, means `r3`).
//!
//! **Adding an alternative (M8):** in this file only — write the view function below the
//! `M8 alternatives` marker (or declare `mod <name>;` there and put it in
//! `src/faces/<name>.rs`), then add one line to [`FACES`]. Nothing else changes; the
//! settings page and the harness list the names from here.

use qp_schema::View;

use crate::depth;
use crate::error::{ErrorKind, GrammarError};
use crate::provider::DataProvider;

/// Builds a face's view: `(finding, max_points, provider, view id) -> View`. `max_points`
/// is the connection profile's point budget; the view id is `capsule-<finding>`.
pub type FaceViewFn = fn(
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError>;

/// One face design.
#[derive(Debug, Clone, Copy)]
pub struct FaceDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`; not an ordinary word, it must not appear in a
    /// reader's prompt).
    pub name: &'static str,
    /// Face height in CSS px at the phone width (390); 120..=844.
    pub height: u32,
    /// The view rule.
    pub view: FaceViewFn,
}

/// The default face design.
pub const DEFAULT_FACE: &str = "r3";

/// Earlier names still accepted: `(alias, name)`.
pub const ALIASES: [(&str, &str); 1] = [("card", "r3")];

// --- M8 alternatives: view functions (or `mod <name>;`) go below this line -------------

/// Face designs a connection may choose; the first is the default.
pub static FACES: &[FaceDesign] = &[
    FaceDesign {
        name: "r3",
        height: 240,
        view: depth::capsule_view,
    },
    // --- M8 alternatives: one line each
];

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    FACES.iter().map(|f| f.name).collect()
}

/// The canonical name of `name` (an alias resolves), if known.
#[must_use]
pub fn canonical(name: &str) -> Option<&'static str> {
    let n = ALIASES
        .iter()
        .find(|(a, _)| *a == name)
        .map_or(name, |(_, n)| *n);
    FACES.iter().find(|f| f.name == n).map(|f| f.name)
}

/// Whether `name` is a known face design (or alias).
#[must_use]
pub fn is_face(name: &str) -> bool {
    canonical(name).is_some()
}

/// The design named `name` (`None`: the default).
///
/// # Errors
/// An unknown name (`invalid`, `/face`).
pub fn by_name(name: Option<&str>) -> Result<&'static FaceDesign, GrammarError> {
    let n = name.unwrap_or(DEFAULT_FACE);
    let c = canonical(n).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "/face",
            format!("unknown face `{n}` (known: {})", names().join(", ")),
        )
    })?;
    Ok(FACES.iter().find(|f| f.name == c).unwrap_or(&FACES[0]))
}

/// The face height of design `name` (`None` or unknown: the default's).
#[must_use]
pub fn height(name: Option<&str>) -> u32 {
    by_name(name).map_or(FACES[0].height, |f| f.height)
}

/// The face view of `finding` by design `face` (`None`: the default).
///
/// # Errors
/// An unknown face name or the design's own errors.
pub fn face_view(
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    (by_name(face)?.view)(finding, max_points, p, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(FACES[0].name, DEFAULT_FACE);
        let mut seen = Vec::new();
        for f in FACES {
            assert!(!seen.contains(&f.name), "duplicate face {}", f.name);
            seen.push(f.name);
            assert!(f.name.starts_with(|c: char| c.is_ascii_lowercase()));
            assert!(f
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
            assert!((120..=844).contains(&f.height), "{} height", f.name);
        }
        assert_eq!(canonical("card"), Some("r3"));
        assert_eq!(height(None), 240);
        assert_eq!(height(Some("card")), 240);
        assert!(by_name(Some("nope")).is_err());
    }
}
