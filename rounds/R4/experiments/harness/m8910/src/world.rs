//! The synthetic worlds the harness reads (SPEC §0.2: synthetic data, fixed replay moments)
//! and the clients that drive them in process.
//!
//! - [`World::demo`]: exactly the demo's host (`qp demo --synthetic --paused`,
//!   `qp_wasm::drive::parity_host`): 30 synthetic instruments, daily bars moved by hourly
//!   bars, every feed bound, the capsule engine on all of them, the `visitor` owner without
//!   data entitlement.
//! - [`World::hourly`]: the harness's own world for the S8 state that the demo cannot show
//!   (a capsule on a 60-minute series): the same synthetic generator's hourly bars
//!   (`qp_source::synthetic_market_hourly`) written as local-format files into the scratch
//!   directory and replayed by the same host with `interval: "1h"` (README "S8 #11").
//!
//! [`Raw`] is a bare protocol client (an agent); [`Core`] is the web shell's core
//! (`qp_wasm::shell::ShellCore`) on a connection of the host, as `qp drive` runs it.

use std::collections::BTreeMap;
use std::path::Path;

use qp_grammar::DataProvider;
use qp_schema::protocol::{
    CapsuleItem, OutboundLine, QueryResult, ReplayInit, ReplaySourceKind, ServerMsg,
};
use qp_schema::Bar;
use qp_session::ReplayHost;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

use crate::io;

/// Which world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The demo host.
    Demo,
    /// The harness's hourly world.
    Hourly,
}

impl Kind {
    /// Record name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Kind::Demo => "demo",
            Kind::Hourly => "hourly",
        }
    }

    /// From a record name.
    #[must_use]
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "demo" => Some(Kind::Demo),
            "hourly" => Some(Kind::Hourly),
            _ => None,
        }
    }
}

/// Instruments of the hourly world.
pub const HOURLY_INSTRUMENTS: usize = 12;
/// Trading days of the hourly world.
pub const HOURLY_DAYS: usize = 90;

/// A replay host.
pub struct World {
    /// The host (authority + replay).
    pub host: ReplayHost,
    /// Which world.
    pub kind: Kind,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World").field("kind", &self.kind).finish()
    }
}

impl World {
    /// The world of `kind`.
    ///
    /// # Errors
    /// The hourly world's files cannot be written, or the host does not start.
    pub fn new(kind: Kind) -> Result<Self, String> {
        match kind {
            Kind::Demo => Ok(Self::demo()),
            Kind::Hourly => Self::hourly(),
        }
    }

    /// The demo's host.
    #[must_use]
    pub fn demo() -> Self {
        let mut host = qp_wasm::drive::parity_host();
        host.set_record_events(true);
        Self {
            host,
            kind: Kind::Demo,
        }
    }

    /// The hourly world (see the module docs).
    ///
    /// # Errors
    /// The files cannot be written, or the host does not start.
    pub fn hourly() -> Result<Self, String> {
        let dir = io::scratch().join("hourly").join("bars");
        for i in qp_source::synthetic_market_hourly(HOURLY_INSTRUMENTS, HOURLY_DAYS, 1) {
            let s = json!({"id": i.symbol.to_lowercase(), "interval": "1h",
                "source": {"kind": "inline"}, "data": {"kind": "bars", "bars": i.bars}});
            io::write_json(&dir.join(format!("{}.1hour.adj.json", i.symbol)), &s)?;
        }
        let mut ent = BTreeMap::new();
        for p in ["host", "arkin", "scout", "guest"] {
            ent.insert(p.to_owned(), vec!["*".to_owned()]);
        }
        let init = qp_session::demo_init(ReplayInit {
            source: ReplaySourceKind::Local,
            instruments: Some(u32::try_from(HOURLY_INSTRUMENTS).unwrap_or(12)),
            interval: Some("1h".to_owned()),
            history: Some(120),
            speed: Some(4.0),
            running: false,
            entitlements: ent,
            ..ReplayInit::default()
        });
        let mut host = ReplayHost::with_local_dir(init, Path::new("."), &dir)?;
        if host.state().synthetic {
            return Err("hourly world: the host did not read the harness's files".into());
        }
        host.set_record_events(true);
        Ok(Self {
            host,
            kind: Kind::Hourly,
        })
    }

    /// The replay clock (ms).
    #[must_use]
    pub fn now(&self) -> i64 {
        self.host.state().t
    }

    /// Steps taken.
    #[must_use]
    pub fn step(&self) -> u64 {
        self.host.state().step
    }

    /// Steps in the replay.
    #[must_use]
    pub fn steps(&self) -> u64 {
        self.host.state().steps
    }

    /// The authority's bars of `series` now.
    #[must_use]
    pub fn bars(&mut self, series: &str) -> Vec<Bar> {
        self.host
            .session_mut()
            .provider()
            .bars(series)
            .map(|b| b.to_vec())
            .unwrap_or_default()
    }
}

/// The messages of `conn` among `lines`.
fn of(conn: &str, lines: Vec<OutboundLine>) -> Vec<ServerMsg> {
    lines
        .into_iter()
        .filter(|l| l.conn == conn)
        .map(|l| l.msg)
        .collect()
}

/// A bare protocol client on one connection.
#[derive(Debug, Clone)]
pub struct Raw {
    /// Connection id.
    pub conn: String,
    n: u64,
}

impl Raw {
    /// Connects `principal` (phone surface, language `lang`).
    #[must_use]
    pub fn connect(w: &mut World, conn: &str, principal: &str, lang: &str) -> Self {
        let m = json!({"type":"hello","protocol":"qp/0","principal":principal,
            "surface":{"kind":"phone","screen":[390,844],"input":["touch"],"lang":lang}});
        w.host.handle_raw(conn, &m.to_string());
        Self {
            conn: conn.to_owned(),
            n: 0,
        }
    }

    /// Sends one message; returns this connection's messages.
    pub fn send(&mut self, w: &mut World, m: &Value) -> Vec<ServerMsg> {
        of(&self.conn, w.host.handle_raw(&self.conn, &m.to_string()))
    }

    /// A query; its result.
    pub fn query(&mut self, w: &mut World, q: Value) -> Option<QueryResult> {
        self.n += 1;
        let id = format!("q{}", self.n);
        let out = self.send(w, &json!({"type":"query","id":id,"query":q}));
        out.into_iter().find_map(|m| match m {
            ServerMsg::Result { id: i, data, .. } if i == id => Some(data),
            _ => None,
        })
    }

    /// The capsules of a `capsules` query with fields `q` (`kind` added).
    pub fn capsules(&mut self, w: &mut World, mut q: Value) -> Vec<CapsuleItem> {
        q["kind"] = json!("capsules");
        match self.query(w, q) {
            Some(QueryResult::Capsules { capsules, .. }) => capsules,
            _ => Vec::new(),
        }
    }

    /// A replay control (`step`, `seek`…).
    pub fn replay(&mut self, w: &mut World, control: Value) {
        self.n += 1;
        let id = format!("r{}", self.n);
        self.send(w, &json!({"type":"replay","id":id,"control":control}));
    }

    /// A command's ops.
    pub fn command(&mut self, w: &mut World, ops: Value) {
        self.n += 1;
        let id = format!("c{}", self.n);
        self.send(w, &json!({"type":"command","id":id,"ops":ops}));
    }
}

/// The web shell's core on one connection of the host (`qp drive`'s `with_core`, with a
/// connection of its own).
pub struct Core {
    /// Connection id.
    pub conn: String,
    /// The core.
    pub core: ShellCore,
}

impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core").field("conn", &self.conn).finish()
    }
}

impl Core {
    /// A shell core for `principal` in `lang` on connection `conn`, connected and idle.
    ///
    /// # Errors
    /// The core cannot be built.
    pub fn connect(w: &mut World, conn: &str, principal: &str, lang: &str) -> Result<Self, String> {
        let data = w
            .host
            .init()
            .data
            .clone()
            .ok_or("replay init has no data")?;
        let evs = w.host.recorded_events().to_vec();
        let core = ShellCore::new(principal, lang, &data, &evs)?;
        let mut c = Self {
            conn: conn.to_owned(),
            core,
        };
        c.pump(w);
        Ok(c)
    }

    /// Passes everything the core queues to the host and the answers back, until idle.
    pub fn pump(&mut self, w: &mut World) {
        for _ in 0..1000 {
            let out = self.core.take_outbox();
            if out.is_empty() {
                break;
            }
            for m in out {
                for msg in of(&self.conn, w.host.handle_raw(&self.conn, &m.to_string())) {
                    self.core.on_message(&msg);
                }
            }
        }
    }

    /// Seeks the replay to `step` (forward) and gives the core the data events.
    pub fn seek(&mut self, w: &mut World, step: u64) {
        let before = w.host.recorded_events().len();
        self.core.seek(&step.to_string());
        self.pump(w);
        let evs: Vec<_> = w.host.recorded_events()[before..].to_vec();
        for ev in &evs {
            self.core.data_event(ev);
        }
        self.pump(w);
    }

    /// Runs `f` on the core, then pumps.
    pub fn act(&mut self, w: &mut World, f: impl FnOnce(&mut ShellCore)) {
        f(&mut self.core);
        self.pump(w);
    }
}
