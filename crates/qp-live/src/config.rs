//! Configuration read from the process environment only (no files).
//!
//! Error messages name the missing or invalid variable, never its value. `Debug` shows no
//! secret. Every secret and address read here is registered in the configuration's
//! [`Redactor`], so later error and event text is masked.

use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use crate::lock::EndpointKey;
use crate::redact::{host_of, Redactor, Secret};

pub const VAR_TOKEN_SERVICE_URL: &str = "MATRIKS_TOKEN_SERVICE_URL";
pub const VAR_API_USERNAME: &str = "MATRIKS_API_USERNAME";
pub const VAR_API_PASSWORD: &str = "MATRIKS_API_PASSWORD";
pub const VAR_HARDWARE_ID: &str = "MATRIKS_HARDWARE_ID";
pub const VAR_SERVICE_TYPE_CODE: &str = "MATRIKS_SERVICE_TYPE_CODE";
pub const VAR_MQTT_RT_HOST: &str = "MATRIKS_MQTT_RT_HOST";
pub const VAR_MQTT_DL_HOST: &str = "MATRIKS_MQTT_DL_HOST";
pub const VAR_MQTT_MODE: &str = "MATRIKS_MQTT_MODE";
pub const VAR_MQTT_STATS_HOST: &str = "MATRIKS_MQTT_STATS_HOST";
pub const VAR_MQTT_NEWS_HOST: &str = "MATRIKS_MQTT_NEWS_HOST";
/// Optional pre-issued token (listed by `scripts/live-env.sh`); only registered for masking.
pub const VAR_PRESET_TOKEN: &str = "MATRIKS_TOKEN";
/// Optional: `3.1` (default, `MQIsdp`) or `3.1.1` (`MQTT`).
pub const VAR_MQTT_PROTOCOL: &str = "QP_LIVE_MQTT_PROTOCOL";
/// Optional: WebSocket subprotocol (default `mqttv3.1`; `none` sends no header).
pub const VAR_WS_SUBPROTOCOL: &str = "QP_LIVE_WS_SUBPROTOCOL";
/// Optional: MQTT keepalive in seconds (default 30).
pub const VAR_KEEPALIVE_SECS: &str = "QP_LIVE_MQTT_KEEPALIVE_SECS";

/// Every hardware id this crate derives starts with this.
pub const HARDWARE_ID_PREFIX: &str = "qapsule-";

/// Market data mode; chosen by `MATRIKS_MQTT_MODE`, never changed by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    Delayed,
    Realtime,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Delayed => "delayed",
            Mode::Realtime => "realtime",
        }
    }

    /// Variable holding the market endpoint for this mode.
    pub fn host_var(self) -> &'static str {
        match self {
            Mode::Delayed => VAR_MQTT_DL_HOST,
            Mode::Realtime => VAR_MQTT_RT_HOST,
        }
    }
}

/// MQTT protocol flavour on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MqttProtocol {
    /// MQTT 3.1: protocol name `MQIsdp`, level 3 (what the reference adapter uses).
    V31,
    /// MQTT 3.1.1: protocol name `MQTT`, level 4.
    V311,
}

impl MqttProtocol {
    pub fn name(self) -> &'static str {
        match self {
            MqttProtocol::V31 => "MQIsdp",
            MqttProtocol::V311 => "MQTT",
        }
    }

    pub fn level(self) -> u8 {
        match self {
            MqttProtocol::V31 => 3,
            MqttProtocol::V311 => 4,
        }
    }
}

/// Wire settings of the MQTT session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MqttSettings {
    pub protocol: MqttProtocol,
    /// `Sec-WebSocket-Protocol` value; empty = no header.
    pub ws_subprotocol: String,
    pub keepalive_secs: u16,
    /// TCP connect, TLS + WebSocket handshake and CONNACK wait, each.
    pub connect_timeout: Duration,
}

impl Default for MqttSettings {
    fn default() -> Self {
        Self {
            protocol: MqttProtocol::V31,
            ws_subprotocol: "mqttv3.1".to_string(),
            keepalive_secs: 30,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareIdSource {
    /// `MATRIKS_HARDWARE_ID` was set.
    Explicit,
    /// Derived as `qapsule-<host name>`.
    DerivedFromHostName,
}

/// Configuration error. Names the variable only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(&'static str),
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing(name) => write!(f, "missing environment variable {name}"),
            ConfigError::Invalid { name, reason } => {
                write!(f, "invalid value in environment variable {name}: {reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// One MQTT-over-WebSocket endpoint (mode + path). The address itself is secret.
#[derive(Clone)]
pub struct Endpoint {
    mode: Mode,
    path: String,
    url: Secret,
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("mode", &self.mode)
            .field("path", &self.path)
            .field("url", &self.url)
            .finish()
    }
}

impl Endpoint {
    /// Accepts a full URL (`wss://host[:port]/path`) or a bare host (`host[:port]`).
    /// No scheme → `wss://`; no path → `default_path`. Plain `ws://` is accepted only for
    /// loopback hosts (tests). `var` names the source variable for errors.
    pub fn parse(
        mode: Mode,
        raw: &str,
        default_path: &str,
        var: &'static str,
    ) -> Result<Self, ConfigError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(ConfigError::Missing(var));
        }
        let invalid = |reason| ConfigError::Invalid { name: var, reason };
        let (scheme, rest) = match raw.split_once("://") {
            Some((s, r)) => (s.to_ascii_lowercase(), r),
            None => ("wss".to_string(), raw),
        };
        if scheme != "wss" && scheme != "ws" {
            return Err(invalid("scheme must be wss (ws only on loopback)"));
        }
        let (authority, path) = match rest.find(['/', '?', '#']) {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        if authority.is_empty() || authority.contains('@') {
            return Err(invalid("no host, or user info in address"));
        }
        if scheme == "ws" && !is_loopback(host_of(authority)) {
            return Err(invalid("plain ws is only allowed on a loopback host"));
        }
        let path = if path.is_empty() || path == "/" {
            default_path.to_string()
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            return Err(invalid("address has a query but no path"));
        };
        let url = format!("{scheme}://{authority}{path}");
        let bare_path = path
            .split(['?', '#'])
            .next()
            .unwrap_or("")
            .trim_matches('/')
            .to_string();
        if bare_path.is_empty() {
            return Err(invalid("empty path"));
        }
        Ok(Self {
            mode,
            path: bare_path,
            url: Secret::new(url),
        })
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Path without slashes (`market`). Not secret.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Lock / ban key: `<mode>-<path>`.
    pub fn key(&self) -> EndpointKey {
        EndpointKey::new(&format!("{}-{}", self.mode.as_str(), self.path))
    }

    pub(crate) fn url(&self) -> &str {
        self.url.expose()
    }
}

fn is_loopback(host: &str) -> bool {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    bare.eq_ignore_ascii_case("localhost")
        || bare.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// Live connection configuration.
#[derive(Clone)]
pub struct LiveConfig {
    token_service_url: Secret,
    username: Secret,
    password: Secret,
    type_code: String,
    hardware_id: String,
    hardware_id_source: HardwareIdSource,
    mode: Mode,
    market: Endpoint,
    /// Wire settings; may be changed by the caller after loading.
    pub mqtt: MqttSettings,
    redactor: Redactor,
}

impl fmt::Debug for LiveConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveConfig")
            .field("mode", &self.mode)
            .field("market", &self.market)
            .field("hardware_id_source", &self.hardware_id_source)
            .field("hardware_id_prefix_ok", &!self.hardware_id_warning())
            .field("mqtt", &self.mqtt)
            .field("secrets", &"[redacted]")
            .finish()
    }
}

impl LiveConfig {
    /// Reads the process environment; the host name comes from the system.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|k| std::env::var(k).ok(), &system_host_name())
    }

    /// Same as [`from_env`](Self::from_env) with an explicit variable lookup and host name
    /// (tests; no process-wide environment mutation).
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
        host_name: &str,
    ) -> Result<Self, ConfigError> {
        let get = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let req = |name: &'static str| get(name).ok_or(ConfigError::Missing(name));
        let redactor = Redactor::new();

        let token_service_url = req(VAR_TOKEN_SERVICE_URL)?;
        redactor.add_url(&token_service_url);
        let username = req(VAR_API_USERNAME)?;
        redactor.add_secret(&username);
        let password = req(VAR_API_PASSWORD)?;
        redactor.add_secret(&password);
        // Every endpoint value is masked, used or not.
        for var in [
            VAR_MQTT_RT_HOST,
            VAR_MQTT_DL_HOST,
            VAR_MQTT_STATS_HOST,
            VAR_MQTT_NEWS_HOST,
        ] {
            if let Some(v) = get(var) {
                redactor.add_url(&v);
            }
        }
        if let Some(t) = get(VAR_PRESET_TOKEN) {
            redactor.add_secret(&t);
        }
        let type_code = req(VAR_SERVICE_TYPE_CODE)?;
        let mode = match req(VAR_MQTT_MODE)?.to_ascii_lowercase().as_str() {
            "delayed" => Mode::Delayed,
            "realtime" => Mode::Realtime,
            _ => {
                return Err(ConfigError::Invalid {
                    name: VAR_MQTT_MODE,
                    reason: "must be 'delayed' or 'realtime'",
                })
            }
        };
        let host_var = mode.host_var();
        let market = Endpoint::parse(mode, &req(host_var)?, "/market", host_var)?;
        redactor.add_url(market.url());

        let (hardware_id, hardware_id_source) = match get(VAR_HARDWARE_ID) {
            Some(h) => (h, HardwareIdSource::Explicit),
            None => (
                derive_hardware_id(host_name),
                HardwareIdSource::DerivedFromHostName,
            ),
        };
        redactor.add_secret(&hardware_id);

        let mut mqtt = MqttSettings::default();
        if let Some(p) = get(VAR_MQTT_PROTOCOL) {
            mqtt.protocol = match p.as_str() {
                "3.1" => MqttProtocol::V31,
                "3.1.1" => MqttProtocol::V311,
                _ => {
                    return Err(ConfigError::Invalid {
                        name: VAR_MQTT_PROTOCOL,
                        reason: "must be '3.1' or '3.1.1'",
                    })
                }
            };
        }
        if let Some(s) = get(VAR_WS_SUBPROTOCOL) {
            mqtt.ws_subprotocol = if s.eq_ignore_ascii_case("none") {
                String::new()
            } else {
                s
            };
        }
        if let Some(k) = get(VAR_KEEPALIVE_SECS) {
            mqtt.keepalive_secs =
                k.parse::<u16>()
                    .ok()
                    .filter(|v| *v >= 5)
                    .ok_or(ConfigError::Invalid {
                        name: VAR_KEEPALIVE_SECS,
                        reason: "must be an integer between 5 and 65535",
                    })?;
        }

        Ok(Self {
            token_service_url: Secret::new(token_service_url),
            username: Secret::new(username),
            password: Secret::new(password),
            type_code,
            hardware_id,
            hardware_id_source,
            mode,
            market,
            mqtt,
            redactor,
        })
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The market endpoint of the configured mode.
    pub fn market_endpoint(&self) -> &Endpoint {
        &self.market
    }

    pub fn hardware_id(&self) -> &str {
        &self.hardware_id
    }

    pub fn hardware_id_source(&self) -> HardwareIdSource {
        self.hardware_id_source
    }

    /// True when the hardware id does not start with `qapsule-` (report it; value not shown).
    pub fn hardware_id_warning(&self) -> bool {
        !self.hardware_id.starts_with(HARDWARE_ID_PREFIX)
    }

    /// The masking list for everything derived from this configuration.
    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    pub(crate) fn token_service_url(&self) -> &Secret {
        &self.token_service_url
    }

    pub(crate) fn username(&self) -> &Secret {
        &self.username
    }

    pub(crate) fn password(&self) -> &Secret {
        &self.password
    }

    pub(crate) fn type_code(&self) -> &str {
        &self.type_code
    }
}

/// `qapsule-<first label of the host name>`, lower case, `[a-z0-9-]` only. Deterministic;
/// never random. An empty host name gives `qapsule-unknown-host`.
pub fn derive_hardware_id(host_name: &str) -> String {
    let first = host_name.trim().split('.').next().unwrap_or("");
    let mut out = String::new();
    for c in first.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    let name = if trimmed.is_empty() {
        "unknown-host"
    } else {
        trimmed
    };
    let mut id = format!("{HARDWARE_ID_PREFIX}{name}");
    id.truncate(64);
    id
}

/// Host name from the `gethostname` system call, then `HOSTNAME`, then `COMPUTERNAME`.
pub fn system_host_name() -> String {
    let sys = gethostname::gethostname()
        .to_string_lossy()
        .trim()
        .to_string();
    if !sys.is_empty() {
        return sys;
    }
    for var in ["HOSTNAME", "COMPUTERNAME"] {
        if let Ok(v) = std::env::var(var) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }

    const BASE: &[(&str, &str)] = &[
        (VAR_TOKEN_SERVICE_URL, "https://auth.fake-vendor.test/issue"),
        (VAR_API_USERNAME, "svc-user-1234"),
        (VAR_API_PASSWORD, "pw-abcdef-5678"),
        (VAR_SERVICE_TYPE_CODE, "H"),
        (VAR_MQTT_MODE, "delayed"),
        (VAR_MQTT_DL_HOST, "dl.fake-vendor.test"),
    ];

    #[test]
    fn missing_variable_error_names_only_the_variable() {
        for (skip, (name, _)) in BASE.iter().enumerate() {
            let pairs: Vec<_> = BASE
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != skip)
                .map(|(_, p)| *p)
                .collect();
            let err = LiveConfig::from_lookup(env(&pairs), "box").unwrap_err();
            assert_eq!(err, ConfigError::Missing(name));
            let text = err.to_string();
            for (_, v) in BASE {
                if v.len() > 2 {
                    assert!(!text.contains(v), "{text}");
                }
            }
        }
    }

    #[test]
    fn invalid_mode_names_variable_not_value() {
        let mut pairs = BASE.to_vec();
        pairs[4] = (VAR_MQTT_MODE, "turbo-mode");
        let err = LiveConfig::from_lookup(env(&pairs), "box").unwrap_err();
        assert!(err.to_string().contains(VAR_MQTT_MODE));
        assert!(!err.to_string().contains("turbo"));
    }

    #[test]
    fn realtime_needs_the_realtime_host() {
        let mut pairs = BASE.to_vec();
        pairs[4] = (VAR_MQTT_MODE, "realtime");
        let err = LiveConfig::from_lookup(env(&pairs), "box").unwrap_err();
        assert_eq!(err, ConfigError::Missing(VAR_MQTT_RT_HOST));
    }

    #[test]
    fn hardware_id_is_derived_deterministically_with_prefix() {
        let a = LiveConfig::from_lookup(env(BASE), "Arkins-MacBook.local").unwrap();
        let b = LiveConfig::from_lookup(env(BASE), "Arkins-MacBook.local").unwrap();
        assert_eq!(a.hardware_id(), "qapsule-arkins-macbook");
        assert_eq!(a.hardware_id(), b.hardware_id());
        assert_eq!(
            a.hardware_id_source(),
            HardwareIdSource::DerivedFromHostName
        );
        assert!(!a.hardware_id_warning());
        assert_eq!(derive_hardware_id(""), "qapsule-unknown-host");
        assert_eq!(derive_hardware_id("  W!N_10 box "), "qapsule-w-n-10-box");
    }

    #[test]
    fn explicit_hardware_id_without_prefix_raises_warning_flag() {
        let mut pairs = BASE.to_vec();
        pairs.push((VAR_HARDWARE_ID, "other-system-7"));
        let c = LiveConfig::from_lookup(env(&pairs), "box").unwrap();
        assert!(c.hardware_id_warning());
        assert_eq!(c.hardware_id_source(), HardwareIdSource::Explicit);
        assert!(!format!("{c:?}").contains("other-system-7"));
    }

    #[test]
    fn endpoint_accepts_bare_host_and_full_url() {
        let v = "MATRIKS_MQTT_DL_HOST";
        let e = Endpoint::parse(Mode::Delayed, "dl.fake.test", "/market", v).unwrap();
        assert_eq!(e.url(), "wss://dl.fake.test/market");
        assert_eq!(e.key().as_str(), "delayed-market");
        let e =
            Endpoint::parse(Mode::Delayed, "wss://dl.fake.test:8443/news", "/market", v).unwrap();
        assert_eq!(e.url(), "wss://dl.fake.test:8443/news");
        assert_eq!(e.path(), "news");
        let e = Endpoint::parse(Mode::Realtime, "dl.fake.test/", "/market", v).unwrap();
        assert_eq!(e.url(), "wss://dl.fake.test/market");
        assert_eq!(e.key().as_str(), "realtime-market");
        assert!(Endpoint::parse(Mode::Delayed, "ws://dl.fake.test/market", "/market", v).is_err());
        assert!(Endpoint::parse(Mode::Delayed, "ws://127.0.0.1:9/market", "/market", v).is_ok());
        assert!(Endpoint::parse(Mode::Delayed, "https://x.test/market", "/market", v).is_err());
        assert!(Endpoint::parse(Mode::Delayed, "u:p@x.test/market", "/market", v).is_err());
    }

    #[test]
    fn debug_shows_no_secret() {
        let c = LiveConfig::from_lookup(env(BASE), "box").unwrap();
        let d = format!("{c:?}");
        for (_, v) in BASE {
            if v.len() > 2 && *v != "delayed" {
                assert!(!d.contains(v), "{d}");
            }
        }
        assert!(!d.contains("fake-vendor"));
    }

    #[test]
    fn protocol_and_subprotocol_are_configurable() {
        let mut pairs = BASE.to_vec();
        pairs.push((VAR_MQTT_PROTOCOL, "3.1.1"));
        pairs.push((VAR_WS_SUBPROTOCOL, "mqtt"));
        pairs.push((VAR_KEEPALIVE_SECS, "60"));
        let c = LiveConfig::from_lookup(env(&pairs), "box").unwrap();
        assert_eq!(c.mqtt.protocol, MqttProtocol::V311);
        assert_eq!(c.mqtt.ws_subprotocol, "mqtt");
        assert_eq!(c.mqtt.keepalive_secs, 60);
        let d = LiveConfig::from_lookup(env(BASE), "box").unwrap();
        assert_eq!(d.mqtt, MqttSettings::default());
    }
}
