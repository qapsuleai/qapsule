//! Who an endpoint speaks as ([`Config`], its `hello`) and how it serves ([`Options`]).

use qp_schema::protocol::Surface;
use serde_json::{json, Value};

use crate::{calllog, NotifyMode};

/// Who the adapter speaks as; becomes its `hello`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Principal id.
    pub principal: String,
    /// Surface.
    pub surface: Surface,
    /// Protocol version string sent in `hello` (normally `"qp/0"`).
    pub protocol: String,
    /// Protocol features requested in `hello`.
    pub features: Vec<String>,
}

impl Config {
    /// `qp/0`, no features.
    #[must_use]
    pub fn new(principal: &str, surface: Surface) -> Self {
        Self {
            principal: principal.to_owned(),
            surface,
            protocol: "qp/0".to_owned(),
            features: Vec::new(),
        }
    }

    pub(crate) fn hello(&self) -> Value {
        let mut h = json!({
            "type": "hello",
            "protocol": self.protocol,
            "principal": self.principal,
            "surface": self.surface,
        });
        if !self.features.is_empty() {
            h["features"] = json!(self.features);
        }
        h
    }
}

/// Options of an endpoint.
#[derive(Clone, Default)]
pub struct Options {
    /// Notification delivery.
    pub notify: NotifyMode,
    /// Offer the common tools ([`common`]: `render`, `notifications`).
    pub common_tools: bool,
    /// Measurement log (one line per tool call).
    pub log: Option<calllog::SharedLog>,
    /// (R5B/M29, measurement) The capsule format a tool gets when its input schema has a
    /// `format` property and the call gives none (`card`, `json`, `layered`). `None`: the
    /// surface's own default.
    pub capsule_format: Option<String>,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("notify", &self.notify)
            .field("common_tools", &self.common_tools)
            .field("log", &self.log.is_some())
            .finish()
    }
}
