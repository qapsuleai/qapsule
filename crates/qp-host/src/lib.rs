//! Qapsule hosting ("barındırma"; RG1/M1b, moved out of `qp-cli`): the live host, the
//! Matriks record and inspection, the trade comparison, the accelerated simulation, the
//! demo HTTP server and the MCP server's wiring. `qp-cli` binds its commands to these.

pub mod demo;
mod demo_static;
pub mod live;
mod live_trades;
mod livehost;
pub mod mcp_serve;
pub mod sim;
