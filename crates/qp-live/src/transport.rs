//! TCP + TLS (rustls, ring provider, webpki roots, certificate verification ON) + WebSocket
//! client handshake with a configurable `Sec-WebSocket-Protocol`.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::HandshakeError;
use tungstenite::http::{HeaderValue, Uri};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::WebSocket;

pub(crate) enum Stream {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Stream {
    pub(crate) fn tcp(&self) -> &TcpStream {
        match self {
            Stream::Plain(s) => s,
            Stream::Tls(t) => &t.sock,
        }
    }

    pub(crate) fn shutdown(&self) {
        let _ = self.tcp().shutdown(Shutdown::Both);
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read(buf),
            Stream::Tls(t) => t.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.write(buf),
            Stream::Tls(t) => t.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.flush(),
            Stream::Tls(t) => t.flush(),
        }
    }
}

/// rustls client configuration: ring provider, safe default protocol versions, Mozilla root
/// store (webpki-roots), standard certificate and host name verification.
pub(crate) fn tls_config() -> Result<Arc<rustls::ClientConfig>, String> {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("tls setup: {e}"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(cfg))
}

/// Opens the WebSocket. Errors are raw text; the caller masks them.
pub(crate) fn open_websocket(
    url: &str,
    subprotocol: &str,
    timeout: Duration,
) -> Result<WebSocket<Stream>, String> {
    let uri: Uri = url
        .parse()
        .map_err(|_| "invalid endpoint address".to_string())?;
    let tls = match uri.scheme_str() {
        Some("wss") => true,
        Some("ws") => false,
        _ => return Err("unsupported scheme".into()),
    };
    let host = uri.host().ok_or("endpoint address has no host")?;
    let bare = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let port = uri.port_u16().unwrap_or(if tls { 443 } else { 80 });
    let addrs = (bare.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("name resolution failed: {e}"))?;
    let mut last_err = None;
    let mut tcp = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => {
                tcp = Some(s);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let tcp = tcp.ok_or_else(|| match last_err {
        Some(e) => format!("tcp connect failed: {e}"),
        None => "tcp connect failed: no address".to_string(),
    })?;
    let _ = tcp.set_nodelay(true);
    tcp.set_read_timeout(Some(timeout))
        .map_err(|e| format!("socket: {e}"))?;
    tcp.set_write_timeout(Some(timeout))
        .map_err(|e| format!("socket: {e}"))?;
    let stream = if tls {
        let name = ServerName::try_from(bare.clone()).map_err(|_| "invalid server name")?;
        let conn =
            rustls::ClientConnection::new(tls_config()?, name).map_err(|e| format!("tls: {e}"))?;
        Stream::Tls(Box::new(rustls::StreamOwned::new(conn, tcp)))
    } else {
        Stream::Plain(tcp)
    };
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("websocket request: {e}"))?;
    if !subprotocol.is_empty() {
        let v = HeaderValue::from_str(subprotocol).map_err(|_| "invalid subprotocol")?;
        request.headers_mut().insert("Sec-WebSocket-Protocol", v);
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(crate::mqtt::MAX_PACKET + 1024))
        .max_frame_size(Some(crate::mqtt::MAX_PACKET + 1024));
    match tungstenite::client::client_with_config(request, stream, Some(config)) {
        Ok((ws, _response)) => Ok(ws),
        Err(HandshakeError::Failure(e)) => Err(format!("websocket handshake failed: {e}")),
        Err(HandshakeError::Interrupted(_)) => Err("websocket handshake timed out".into()),
    }
}
