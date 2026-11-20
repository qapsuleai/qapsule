//! (RP4, K66) Topach: which data types, topic patterns and endpoints the licence allows.
//!
//! `GET /dumrul/v2/topach` (`docs/DATA_ACCESS.md`, "Lisansa göre konu keşfi") answers with
//! the user's data types, their topic patterns (`%s` = symbol) and which endpoint (real-time
//! / delayed, path) to connect to. **The response shape is not known yet** (no call has been
//! made), so [`parse_topach`] is tolerant: it walks any JSON and collects every topic pattern
//! it finds with the nearest data type name and endpoint hint. It is pure (input: a JSON
//! value; no network, no clock).
//!
//! Addresses never leave this module: an endpoint given as a URL is reduced to its path
//! (`wss://host/trade` → `trade`); the host is dropped.
//!
//! TODO(RP4, orchestrator): the network call belongs to `crate::rest::RestClient` (same
//! token, same pacing), e.g. `RestClient::topach(&self) -> Result<serde_json::Value, RestError>`
//! followed by [`parse_topach`]. Not written here (no network in this task).

use std::collections::BTreeSet;

use serde_json::Value;

/// One allowed topic pattern.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TopachEntry {
    /// Data type name as served (`trade`, `symbol`, `depth`, …), or `""` if none was found.
    pub data_type: String,
    /// Topic pattern with `%s` for the symbol (e.g. `mx/trade/%s`).
    pub topic_pattern: String,
    /// Endpoint path without slashes (`trade`, `market`), if the response named one.
    pub endpoint_path: Option<String>,
    /// `Some(true)` real-time, `Some(false)` delayed, `None` not said.
    pub realtime: Option<bool>,
}

impl TopachEntry {
    /// The topic for `symbol` (`%s` replaced; a pattern without `%s` is returned as is).
    pub fn topic_for(&self, symbol: &str) -> String {
        self.topic_pattern.replace("%s", symbol)
    }

    /// Level suffix of the pattern (`lvl0`, `lvl2`), if any.
    pub fn level(&self) -> Option<&str> {
        self.topic_pattern.rsplit_once('@').map(|(_, l)| l)
    }

    /// A level-2 trade pattern (buyer / seller brokers). Qapsule never subscribes to it
    /// (K66), whatever the licence allows.
    pub fn is_level2_trade(&self) -> bool {
        crate::conn::is_level2_trade_topic(&self.topic_for("X"))
    }
}

const TYPE_KEYS: &[&str] = &[
    "dataType",
    "data_type",
    "datatype",
    "type",
    "name",
    "key",
    "code",
    "id",
];
const ENDPOINT_KEYS: &[&str] = &[
    "endpoint",
    "endPoint",
    "endpointPath",
    "path",
    "mqttPath",
    "url",
    "uri",
    "stream",
    "server",
];
const MODE_KEYS: &[&str] = &[
    "mode",
    "realtime",
    "realTime",
    "isRealtime",
    "isRealTime",
    "delayed",
    "isDelayed",
];

/// Looks like a topic pattern: `mx/…` or containing `%s`.
fn is_pattern(s: &str) -> bool {
    let s = s.trim();
    !s.contains("://") && (s.starts_with("mx/") || (s.contains("%s") && s.contains('/')))
}

/// `wss://host:port/trade?x` → `trade`; `/trade` → `trade`; `trade` → `trade`. A bare
/// host (no path) gives `None`.
fn endpoint_path(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let rest = match raw.split_once("://") {
        Some((_, r)) => match r.find('/') {
            Some(i) => &r[i..],
            None => return None,
        },
        None => raw,
    };
    let p = rest
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim_matches('/');
    if p.is_empty() || p.contains('.') && !p.contains('/') {
        // `api.host.com` without a path is an address, not a path.
        return None;
    }
    Some(p.to_string())
}

fn mode_of(v: &Value, key: &str) -> Option<bool> {
    match v {
        Value::Bool(b) => {
            let delayed_key = key.to_ascii_lowercase().contains("delayed");
            Some(*b != delayed_key)
        }
        Value::String(s) => mode_word(s),
        _ => None,
    }
}

fn mode_word(s: &str) -> Option<bool> {
    let l = s.to_ascii_lowercase();
    if l.contains("delay") || l == "dl" || l.ends_with("-dl") || l.starts_with("dl-") {
        Some(false)
    } else if l.contains("real") || l == "rt" || l.ends_with("-rt") || l.starts_with("rt-") {
        Some(true)
    } else {
        None
    }
}

#[derive(Clone, Default)]
struct Ctx {
    data_type: String,
    /// The data type came from a type field of the current object (a key does not override).
    explicit: bool,
    endpoint: Option<String>,
    realtime: Option<bool>,
}

fn obj_ctx(map: &serde_json::Map<String, Value>, parent: &Ctx, parent_key: Option<&str>) -> Ctx {
    let mut c = parent.clone();
    c.explicit = false;
    if let Some(k) = parent_key {
        if let Some(m) = mode_word(k) {
            c.realtime = Some(m);
        } else if key_names_type(k) {
            c.data_type = k.to_string();
        }
    }
    for k in TYPE_KEYS {
        if let Some(Value::String(s)) = map.get(*k) {
            if !is_pattern(s) && !s.trim().is_empty() {
                c.data_type = s.trim().to_string();
                c.explicit = true;
                break;
            }
        }
    }
    for k in ENDPOINT_KEYS {
        if let Some(Value::String(s)) = map.get(*k) {
            if !is_pattern(s) {
                if let Some(m) = mode_word(s) {
                    c.realtime = Some(m);
                }
                if let Some(p) = endpoint_path(s) {
                    // `realtime` / `delayed` as an endpoint name is a mode, not a path.
                    if mode_word(&p).is_none() || p.contains('/') {
                        c.endpoint = Some(p);
                    }
                }
                break;
            }
        }
    }
    for k in MODE_KEYS {
        if let Some(v) = map.get(*k) {
            if let Some(m) = mode_of(v, k) {
                c.realtime = Some(m);
                break;
            }
        }
    }
    c
}

/// A map key that names a data type: not a wrapper, a type field, an index or a mode.
fn key_names_type(k: &str) -> bool {
    !k.is_empty()
        && !k.chars().all(|ch| ch.is_ascii_digit())
        && !is_container_key(k)
        && !TYPE_KEYS.contains(&k)
        && !ENDPOINT_KEYS.contains(&k)
        && !MODE_KEYS.contains(&k)
        && mode_word(k).is_none()
}

/// Keys that only wrap the payload (`result`, `data`, `topics`, …): not data type names.
fn is_container_key(k: &str) -> bool {
    matches!(
        k.to_ascii_lowercase().as_str(),
        "result"
            | "results"
            | "data"
            | "items"
            | "list"
            | "topics"
            | "topic"
            | "patterns"
            | "pattern"
            | "topicpattern"
            | "topicpatterns"
            | "permissions"
            | "licenses"
            | "payload"
            | "content"
            | "value"
            | "values"
    )
}

fn walk(v: &Value, ctx: &Ctx, key: Option<&str>, out: &mut BTreeSet<TopachEntry>, depth: u32) {
    if depth > 32 {
        return;
    }
    match v {
        Value::String(s) if is_pattern(s) => {
            let mut c = ctx.clone();
            if let Some(k) = key {
                if !c.explicit && key_names_type(k) {
                    c.data_type = k.to_string();
                }
            }
            out.insert(TopachEntry {
                data_type: c.data_type,
                topic_pattern: s.trim().to_string(),
                endpoint_path: c.endpoint,
                realtime: c.realtime,
            });
        }
        Value::Array(items) => {
            for it in items {
                walk(it, ctx, key, out, depth + 1);
            }
        }
        Value::Object(map) => {
            let c = obj_ctx(map, ctx, key);
            for (k, child) in map {
                walk(child, &c, Some(k), out, depth + 1);
            }
        }
        _ => {}
    }
}

/// Every topic pattern in a Topach response, with the nearest data type name, endpoint
/// path and real-time / delayed hint. Tolerant of the (unknown) shape: arrays, wrapper
/// objects (`result`, `data`), maps keyed by data type, single strings or lists of
/// patterns. Sorted and de-duplicated. Hosts are never returned.
pub fn parse_topach(v: &Value) -> Vec<TopachEntry> {
    let mut out = BTreeSet::new();
    walk(v, &Ctx::default(), None, &mut out, 0);
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn find<'a>(es: &'a [TopachEntry], pattern: &str) -> &'a TopachEntry {
        es.iter()
            .find(|e| e.topic_pattern == pattern)
            .unwrap_or_else(|| panic!("{pattern} not in {es:?}"))
    }

    #[test]
    fn array_of_objects_with_topic_lists() {
        let v = json!({"success": true, "result": [
            {"dataType": "trade", "topics": ["mx/trade/%s", "mx/tradeex/%s", "mx/trade/%s@lvl2"],
             "endpoint": "wss://rt.fake-vendor.test/trade", "realtime": true},
            {"dataType": "symbol", "topic": "mx/symbol/%s@lvl2", "endpoint": "/market",
             "mode": "delayed"},
        ]});
        let es = parse_topach(&v);
        assert_eq!(es.len(), 4);
        let t = find(&es, "mx/trade/%s");
        assert_eq!(t.data_type, "trade");
        assert_eq!(t.endpoint_path.as_deref(), Some("trade"));
        assert_eq!(t.realtime, Some(true));
        assert_eq!(t.topic_for("TEST1"), "mx/trade/TEST1");
        assert!(!t.is_level2_trade());
        assert!(find(&es, "mx/trade/%s@lvl2").is_level2_trade());
        let s = find(&es, "mx/symbol/%s@lvl2");
        assert_eq!(s.data_type, "symbol");
        assert_eq!(s.endpoint_path.as_deref(), Some("market"));
        assert_eq!(s.realtime, Some(false));
        assert!(!s.is_level2_trade());
        assert_eq!(s.level(), Some("lvl2"));
        // No host anywhere.
        assert!(!format!("{es:?}").contains("fake-vendor"));
    }

    #[test]
    fn map_keyed_by_data_type_and_mode() {
        let v = json!({"data": {
            "trade": {"realtime": {"pattern": "mx/trade/%s", "path": "trade"}},
            "depth": "mx/depth/%s",
        }});
        let es = parse_topach(&v);
        let t = find(&es, "mx/trade/%s");
        assert_eq!(t.data_type, "trade");
        assert_eq!(t.realtime, Some(true));
        assert_eq!(t.endpoint_path.as_deref(), Some("trade"));
        let d = find(&es, "mx/depth/%s");
        assert_eq!(d.data_type, "depth");
        assert_eq!(d.endpoint_path, None);
    }

    #[test]
    fn unknown_shapes_give_nothing_or_partial() {
        assert!(parse_topach(&json!(null)).is_empty());
        assert!(parse_topach(&json!({"success": false, "message": "denied"})).is_empty());
        assert!(parse_topach(&json!([1, 2, "x"])).is_empty());
        // A URL-looking string is not a pattern; a bare host is not a path.
        let es = parse_topach(&json!({"type": "news", "url": "wss://h.test", "t": "mx/news/%s"}));
        assert_eq!(es.len(), 1);
        assert_eq!(es[0].endpoint_path, None);
        assert_eq!(es[0].data_type, "news");
    }

    #[test]
    fn deterministic_and_deduplicated() {
        let v = json!([{"type": "trade", "topic": "mx/trade/%s"},
                       {"type": "trade", "topic": "mx/trade/%s"}]);
        assert_eq!(parse_topach(&v), parse_topach(&v));
        assert_eq!(parse_topach(&v).len(), 1);
    }
}
