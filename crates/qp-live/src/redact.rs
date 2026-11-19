//! The single place where secret values are masked.
//!
//! Every human-readable text this crate produces (error `Display`, event `Display`) is built
//! through [`Redactor::scrub`]. The redactor knows the configured secrets (password, user
//! name, Basic header, endpoint addresses and host names, token service address, every
//! token obtained) and, independently of that list, masks anything that looks like a JWT.

use std::fmt;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

/// Replacement text for a masked value.
pub const MASK: &str = "[redacted]";

/// Values shorter than this are not registered (masking them would destroy unrelated text).
const MIN_LEN: usize = 4;

/// A secret string. `Debug` never shows the value and there is no `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value. Only for putting it on the wire; never for text output.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({MASK})")
    }
}

/// Shared list of secret values; cheap to clone (all clones see later additions).
#[derive(Clone, Default)]
pub struct Redactor {
    needles: Arc<RwLock<Vec<String>>>,
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Redactor {{ secrets: {} }}", self.len())
    }
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered values.
    pub fn len(&self) -> usize {
        self.needles.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Registers a secret value (trimmed). Values shorter than 4 characters are ignored.
    pub fn add_secret(&self, value: &str) {
        let v = value.trim();
        if v.chars().count() < MIN_LEN {
            return;
        }
        let mut g = self.needles.write().unwrap_or_else(|e| e.into_inner());
        if !g.iter().any(|n| n.eq_ignore_ascii_case(v)) {
            g.push(v.to_string());
            // Longest first, so a URL is masked whole before its host part.
            g.sort_by_key(|n| std::cmp::Reverse(n.len()));
        }
    }

    /// Registers an address: the full value, the value without scheme, the authority
    /// (`host:port`), the host and the host's parent domains (two labels or more).
    pub fn add_url(&self, url: &str) {
        let url = url.trim();
        self.add_secret(url);
        let rest = url.split_once("://").map_or(url, |(_, r)| r);
        self.add_secret(rest);
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let authority = authority.rsplit_once('@').map_or(authority, |(_, a)| a);
        self.add_secret(authority);
        let host = host_of(authority);
        self.add_secret(host);
        let bare = host.trim_start_matches('[').trim_end_matches(']');
        if bare.parse::<IpAddr>().is_err() {
            let labels: Vec<&str> = bare.split('.').collect();
            for start in 1..labels.len().saturating_sub(1) {
                self.add_secret(&labels[start..].join("."));
            }
        }
    }

    /// Returns `text` with every registered value (ASCII case-insensitive) and every
    /// JWT-shaped string replaced by [`MASK`].
    pub fn scrub(&self, text: &str) -> String {
        let mut out = text.to_string();
        {
            let g = self.needles.read().unwrap_or_else(|e| e.into_inner());
            for n in g.iter() {
                out = replace_ci(&out, n);
            }
        }
        scrub_jwt(&out)
    }
}

/// Host part of an authority (`host`, `host:port`, `[v6]:port`).
pub(crate) fn host_of(authority: &str) -> &str {
    if authority.starts_with('[') {
        return match authority.find(']') {
            Some(i) => &authority[..=i],
            None => authority,
        };
    }
    match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => authority,
    }
}

/// ASCII case-insensitive replace of `needle` by [`MASK`].
fn replace_ci(hay: &str, needle: &str) -> String {
    if needle.is_empty() {
        return hay.to_string();
    }
    let lower_hay = hay.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(hay.len());
    let mut pos = 0;
    while let Some(i) = lower_hay[pos..].find(&lower_needle) {
        let at = pos + i;
        out.push_str(&hay[pos..at]);
        out.push_str(MASK);
        pos = at + lower_needle.len();
    }
    out.push_str(&hay[pos..]);
    out
}

/// Masks JWT-shaped runs: `eyJ` followed by base64url characters containing a `.`.
pub fn scrub_jwt(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    let mut i = 0;
    let is_jwt_byte =
        |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'=' | b'+' | b'/');
    while i + 3 <= bytes.len() {
        if &bytes[i..i + 3] == b"eyJ" && (i == 0 || !is_jwt_byte(bytes[i - 1])) {
            let mut j = i;
            while j < bytes.len() && is_jwt_byte(bytes[j]) {
                j += 1;
            }
            let run = &text[i..j];
            if run.len() >= 16 && run.contains('.') {
                out.push_str(&text[pos..i]);
                out.push_str(MASK);
                pos = j;
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out.push_str(&text[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_registered_values_case_insensitively() {
        let r = Redactor::new();
        r.add_secret("Hunter2-pass");
        r.add_url("wss://Feed.Example-Data.com:8443/market");
        let t = r.scrub(
            "pw=hunter2-PASS url=wss://feed.example-data.com:8443/market host=FEED.example-data.com \
             dom=example-data.com auth=feed.example-data.com:8443",
        );
        assert!(!t.to_ascii_lowercase().contains("hunter2"), "{t}");
        assert!(!t.contains("example-data"), "{t}");
        assert!(!t.contains("8443"), "{t}");
    }

    #[test]
    fn short_values_are_ignored() {
        let r = Redactor::new();
        r.add_secret("ab");
        assert_eq!(r.scrub("abc ab"), "abc ab");
    }

    #[test]
    fn masks_jwt_shapes_without_registration() {
        let jwt = "eyJhbGciOiJub25lIn0.eyJleHAiOjF9.c2ln";
        let out = Redactor::new().scrub(&format!("Bearer {jwt} tail"));
        assert_eq!(out, format!("Bearer {MASK} tail"));
        // Not a JWT: no dot, or too short.
        assert_eq!(scrub_jwt("eyJabc"), "eyJabc");
    }

    #[test]
    fn ip_hosts_do_not_register_suffixes() {
        let r = Redactor::new();
        r.add_url("ws://127.0.0.1:4000/market");
        assert_eq!(r.scrub("10.0.0.1"), "10.0.0.1");
        assert_eq!(r.scrub("127.0.0.1"), MASK);
    }

    #[test]
    fn secret_debug_hides_value() {
        let s = Secret::new("very-secret-value");
        assert!(!format!("{s:?}").contains("very"));
    }
}
