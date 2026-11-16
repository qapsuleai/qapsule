//! bs2 ablation: the real authority and replica without §19.9 (R3/M7; see `bs2.rs`).

use crate::adapter::Approach;

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "bs2n",
        summary: "bs2 ablation: the real authority and replica without §19.9",
        authority: super::bs2::authority::<false>,
        replica: super::bs2::replica::<false>,
    }
}
