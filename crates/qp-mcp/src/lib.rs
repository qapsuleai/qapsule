//! MCP server adapter over the `qp/0` reference authority (`rounds/RD1/EMBEDDING.md` §1).
//!
//! - [`Hub`]: one [`qp_session::Session`] shared by any number of adapters, with one
//!   queue of pending `notify` messages per protocol connection.
//! - [`McpServer`]: one MCP endpoint bound to one principal, one device surface and one
//!   tool surface ([`surfaces`]). It sends the `hello` itself at [`McpServer::open`],
//!   then answers JSON-RPC 2.0: `initialize`, `server/discover`, `ping`, `tools/list`,
//!   `tools/call`, `resources/list`, `resources/read` and (R5B, [`NotifyMode::Spec`])
//!   `subscriptions/listen` (and ignores other notifications).
//! - [`tools`]: the protocol tool list, generated from the protocol JSON Schema.
//! - [`surfaces`]: (R5B, M28) the tool surfaces `qp mcp serve --surface` chooses from;
//!   [`server_tools`]: the tools every served surface gets (`render`, `notifications`).
//! - [`calllog`]: (R5B) the measurement log of tool calls.
//! - [`conform`]: the conformance transcript runner over MCP.
//!
//! The thin surface adds no state to the protocol: every tool call becomes exactly one
//! `qp/0` client message handed to [`qp_session::Session::handle_raw`], and the reply
//! message is the tool result.

pub mod calllog;
mod config;
pub mod conform;
mod hub;
mod notify;
mod rpc;
mod server;
pub mod server_tools;
pub mod surfaces;
pub mod tools;

pub use config::{Config, Options};
pub use hub::{Hub, Queued, SharedHub};
pub use notify::{Delivery, Inbox, NotifyMode, INBOX_CAP, NOTIFY_URI};
pub use rpc::{
    notification, tool_result, RpcError, LEGACY_VERSIONS, LIST_TTL_MS, META_ORDER,
    META_REQUEST_ID, META_WELCOME, MODERN_VERSIONS,
};
pub(crate) use rpc::{check_version, error_response};
pub use server::McpServer;
pub use surfaces::{Ctx, ToolDef, ToolSurface};
