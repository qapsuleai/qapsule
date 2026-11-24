//! Authority and command path: the session and the agent policy (command application is
//! `qp_ops::apply`; errors, suggestions and the values cache are in `qp-plane`).

pub mod agent;
pub(crate) mod session;
