//! `qp-live`: the live Matriks connection core (server side only; never in the WASM package).
//!
//! Token acquisition ([`auth`]), MQTT 3.1 / 3.1.1 over WebSocket + TLS ([`mqtt`], [`conn`]),
//! single-connection locks ([`lock`]), deterministic back-off ([`backoff`]) and secret masking
//! ([`redact`]). Configuration comes from the environment only ([`config`]). The crate prints
//! nothing itself: messages and events go to a [`LiveSink`]; every event / error text is
//! masked. See `README.md` for the protection rules and where each is enforced.

pub mod auth;
pub mod backoff;
pub mod config;
pub mod conn;
pub mod error;
pub mod lock;
#[cfg(any(test, feature = "mock"))]
pub mod mock;
pub mod mqtt;
pub mod redact;
mod transport;

pub use auth::{AccessToken, AuthClient, AuthPolicy};
pub use backoff::BackoffPolicy;
pub use config::{Endpoint, LiveConfig, Mode, MqttProtocol, MqttSettings};
pub use conn::{
    ConnState, ConnectOptions, DisconnectReason, LiveConnection, LiveEvent, LiveMessage, LiveSink,
    Notice, NoticeClassifier, TokenRenewal,
};
pub use error::{AuthError, HaltReason, LiveError};
pub use lock::{EndpointKey, LockError};
pub use redact::Redactor;
