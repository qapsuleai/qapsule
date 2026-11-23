//! Qapsule named designs (RF1; `rounds/RF1/PLAN.md` §3.1): capsule faces ([`faces`]), open
//! capsule designs ([`open`]), story charts ([`stories`]), feed layouts ([`feeds`]), depth
//! transitions ([`transitions`]), motion designs ([`motion`]) and the depth entries that
//! forward to them ([`depth`]). The grammar compiler reaches them only through the design
//! registry ([`DESIGNS`], handed to it by [`options`]).

pub mod depth;
pub mod faces;
pub mod feeds;
pub mod motion;
pub mod open;
mod registry;
pub mod stories;
pub mod transitions;

pub use registry::{options, DESIGNS};
