//! `qp demo`'s static files (RP3/M13): the web shell's files with pre-compressed variants
//! and revalidation. Still `std` only: the host never compresses; `scripts/demo.sh` writes
//! `<file>.br` (when the `brotli` tool exists) and `<file>.gz` (`gzip -9`) next to the
//! package files, and the host picks one by the request's `Accept-Encoding`:
//!
//! - `br` if accepted and `<file>.br` is at least as new as `<file>`; else `gzip` likewise
//!   with `<file>.gz`; else the file itself (`identity`). A compressed file older than its
//!   source (a rebuild without re-compressing) is never served.
//! - `Content-Type` is the source file's (`application/wasm` for the WASM, so the page's
//!   `WebAssembly.instantiateStreaming` path is taken), `Content-Encoding` the variant's,
//!   `Vary: Accept-Encoding`.
//! - `Cache-Control: no-cache` plus an `ETag` (length, modification time, encoding): the
//!   browser keeps a copy but asks every time, so a rebuilt package is never stale, and an
//!   unchanged one costs a `304` without a body (and lets the browser reuse its compiled
//!   WASM). Every other response of the host stays `no-store`.

use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The encodings the host can serve, best first, with their file suffixes.
const VARIANTS: [(&str, &str); 2] = [("br", "br"), ("gzip", "gz")];

/// Whether `accept` (an `Accept-Encoding` value) accepts `coding` (RFC 9110 §12.5.3:
/// `q=0` refuses; `*` stands for any coding not named).
pub(crate) fn accepts(accept: &str, coding: &str) -> bool {
    let mut star = None;
    for item in accept.split(',') {
        let mut parts = item.split(';');
        let name = parts.next().unwrap_or("").trim();
        let q = parts
            .filter_map(|p| p.trim().strip_prefix("q="))
            .find_map(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(1.0);
        if name.eq_ignore_ascii_case(coding) {
            return q > 0.0;
        }
        if name == "*" {
            star = Some(q > 0.0);
        }
    }
    star.unwrap_or(false)
}

/// Length and modification time (ns since the epoch) of a regular file.
fn stamp(p: &Path) -> Option<(u64, u128)> {
    let m = std::fs::metadata(p)
        .ok()
        .filter(std::fs::Metadata::is_file)?;
    let t = m
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((m.len(), t))
}

/// The file to send for `file` and `accept`: (path, `Content-Encoding` or `None`, length,
/// modification time).
pub(crate) fn pick(
    file: &Path,
    accept: &str,
) -> Option<(PathBuf, Option<&'static str>, u64, u128)> {
    let (len, t) = stamp(file)?;
    for (coding, suffix) in VARIANTS {
        if !accepts(accept, coding) {
            continue;
        }
        let mut name = file.as_os_str().to_owned();
        name.push(".");
        name.push(suffix);
        let v = PathBuf::from(name);
        if let Some((vlen, vt)) = stamp(&v) {
            if vt >= t {
                return Some((v, Some(coding), vlen, vt));
            }
        }
    }
    Some((file.to_path_buf(), None, len, t))
}

/// Serves `file` (already checked by `static_path`) for a request with `headers`
/// (lower-case names). `ctype` is the source file's type, `csp` the host's policy.
pub(crate) fn serve(
    s: &mut TcpStream,
    file: &Path,
    ctype: &str,
    csp: &str,
    headers: &BTreeMap<String, String>,
) -> bool {
    let accept = headers.get("accept-encoding").map_or("", String::as_str);
    let Some((path, coding, len, t)) = pick(file, accept) else {
        return false;
    };
    let etag = format!("\"{len:x}-{t:x}-{}\"", coding.unwrap_or("identity"));
    let enc = coding
        .map(|c| format!("Content-Encoding: {c}\r\n"))
        .unwrap_or_default();
    let common = format!(
        "Content-Type: {ctype}\r\n{enc}Vary: Accept-Encoding\r\nETag: {etag}\r\n\
Cache-Control: no-cache\r\nContent-Security-Policy: {csp}\r\nConnection: close\r\n"
    );
    let fresh = headers
        .get("if-none-match")
        .is_some_and(|v| v.split(',').any(|x| x.trim() == etag));
    if fresh {
        let head = format!("HTTP/1.1 304 Not Modified\r\n{common}\r\n");
        let _ = s.write_all(head.as_bytes());
        return true;
    }
    let Ok(body) = std::fs::read(&path) else {
        return false;
    };
    let head = format!(
        "HTTP/1.1 200 OK\r\n{common}Content-Length: {}\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(&body);
    true
}

#[cfg(test)]
mod tests {
    use super::accepts;

    #[test]
    fn accept_encoding() {
        assert!(accepts("gzip, deflate, br", "br"));
        assert!(accepts("gzip, deflate", "gzip"));
        assert!(!accepts("gzip, deflate", "br"));
        assert!(!accepts("", "gzip"));
        assert!(!accepts("identity", "gzip"));
        assert!(!accepts("gzip;q=0, br", "gzip"));
        assert!(accepts("GZIP;q=0.5", "gzip"));
        assert!(accepts("*", "br"));
        assert!(!accepts("*;q=0", "gzip"));
        assert!(!accepts("br;q=0, *", "br"));
    }
}
