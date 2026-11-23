//! The presenter's side of the grammar: depth rules ([`depth`]) and pixel gestures on a
//! compiled scene ([`gesture`]). Grouped here for the folder budget; the public paths stay
//! `qp_grammar::depth`, `qp_grammar::gesture`. (RF1: the motion designs and the depth
//! entries that only forward to a design are in `qp-designs`.)

pub mod depth;
pub mod gesture;
