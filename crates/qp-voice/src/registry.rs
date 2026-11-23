//! The voice registry (R5/m17-int): the names a connection may choose and the defaults.

/// Voice names a connection may choose; the first is the default.
pub const VOICES: [&str; 3] = ["r4", "marla", "selvi"];

/// The default voice: today's text (no voice module).
pub const DEFAULT_VOICE: &str = "r4";

/// The voice the presentation shell asks for when the reader chose none (M17's pick,
/// `rounds/R5/experiments/M17/RESULTS.md` §6). Like `qp_designs::faces::PRESENTED_FACE`
/// it is sent by the shell core with the `style` gesture; the protocol default
/// ([`DEFAULT_VOICE`]) does not change.
pub const PRESENTED_VOICE: &str = "marla";

/// Whether `name` is a known voice.
#[must_use]
pub fn is_voice(name: &str) -> bool {
    VOICES.contains(&name)
}

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    VOICES.to_vec()
}
