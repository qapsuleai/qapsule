//! The replica consistency scenario (tests, `qp replay`): the authority with the standard
//! subsystems, the replica and the open side with the follower's (as the product builds
//! them; RF1 S3.2, audit-s32 B2).

pub use qp_authority::tape::*;

/// Runs the scenario with the replica's data `lag` events behind the authority's.
///
/// # Errors
/// The scenario cannot be set up.
pub fn run(lag: usize) -> Result<Outcome, String> {
    qp_authority::tape::run(lag, (crate::STANDARD, crate::FOLLOWER))
}

/// [`run`] with another replica provider.
///
/// # Errors
/// The scenario cannot be set up.
pub fn run_with(lag: usize, provider: &serde_json::Value) -> Result<Outcome, String> {
    qp_authority::tape::run_with(lag, provider, (crate::STANDARD, crate::FOLLOWER))
}

/// The consistency scenario for a viewer.
///
/// # Errors
/// The scenario cannot be set up.
pub fn run_viewer(lag: usize) -> Result<ViewerOutcome, String> {
    qp_authority::tape::run_viewer(lag, (crate::STANDARD, crate::FOLLOWER))
}
