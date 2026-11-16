//! bs2 ablation: settled-bar hygiene only, no disputes (R3/M7; see `bs2.rs`).

use crate::adapter::Approach;

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "bs2h",
        summary: "bs2 ablation: settled-bar hygiene only, no disputes",
        authority: super::bs2::authority::<true>,
        replica: super::bs2::replica::<false>,
    }
}
