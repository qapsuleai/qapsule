//! The look registry: [`LOOKS`], the default and the presented look, companion faces and
//! lookup by name.

use super::*;

/// Looks a connection may choose; the first is the default.
pub static LOOKS: &[Look] = &[R4, ORNEK, OLTAN, ZEMRE, TAVEK, TAVEK_TEL];

/// The default look's name.
pub const DEFAULT_LOOK: &str = "r4";

/// (R5/S) The look the presentation shell asks for when the reader chose none (M16's pick,
/// `rounds/R5/experiments/M16/RESULTS.md`); the protocol default ([`DEFAULT_LOOK`]) does
/// not change, so bare connections, transcripts and the `v1` goldens stay byte-identical.
pub const PRESENTED_LOOK: &str = "oltan";

/// (R5/S) Each look's companion face design (`qp_grammar::faces`): choosing a look in the
/// presentation chooses its face too, unless the reader names a face (`?face=`). `r4`:
/// R4's presented face. A look without one keeps the face as it is. (RD2/C3) `tavek`:
/// `zemre`'s sentence, track record and price strip (`docs/FACE.md`).
pub const COMPANION_FACES: &[(&str, &str)] = &[
    ("r4", "vadrel"),
    ("oltan", "oltan"),
    ("zemre", "zemre"),
    ("tavek", "zemre"),
    ("tavek-tel", "zemre"),
];

/// (R5/S) The companion face of look `look` ([`COMPANION_FACES`]), if it has one.
#[must_use]
pub fn companion_face(look: &str) -> Option<&'static str> {
    COMPANION_FACES
        .iter()
        .find(|(l, _)| *l == look)
        .map(|(_, f)| *f)
}

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    LOOKS.iter().map(|l| l.name).collect()
}

/// Whether `name` is a registered look.
#[must_use]
pub fn is_look(name: &str) -> bool {
    LOOKS.iter().any(|l| l.name == name)
}

/// The look named `name` (`None`: the default).
///
/// # Errors
/// An unknown name (`invalid`, `/look`).
pub fn by_name(name: Option<&str>) -> Result<&'static Look, GrammarError> {
    let n = name.unwrap_or(DEFAULT_LOOK);
    LOOKS.iter().find(|l| l.name == n).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "/look",
            format!("unknown look `{n}` (known: {})", names().join(", ")),
        )
    })
}

/// The look named `name`, the default for `None` or an unknown name.
#[must_use]
pub fn or_default(name: Option<&str>) -> &'static Look {
    by_name(name).unwrap_or(&LOOKS[0])
}
