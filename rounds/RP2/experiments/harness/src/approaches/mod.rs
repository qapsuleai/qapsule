//! Approach registry.
//!
//! An approach is a module `src/approaches/<name>.rs` that implements
//! [`crate::adapter::AuthorityAdapter`] and [`crate::adapter::ReplicaAdapter`] and exposes
//! `pub fn register() -> crate::adapter::Approach`. Adding one is a one-line change:
//! append its module name to the list in `approaches!(...)` below.

macro_rules! approaches {
    ($($m:ident),* $(,)?) => {
        $(pub mod $m;)*

        /// All registered approaches, in registration order.
        #[must_use]
        pub fn registry() -> Vec<crate::adapter::Approach> {
            vec![$($m::register()),*]
        }
    };
}

// One line per approach. T7 branches append their module here (e.g. `n, a`).
approaches!(n, c);

/// Finds an approach by name.
#[must_use]
pub fn find(name: &str) -> Option<crate::adapter::Approach> {
    registry().into_iter().find(|a| a.name == name)
}
