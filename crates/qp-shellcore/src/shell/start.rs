//! Starting a shell core (RG1: moved out of `shell.rs`): the typed start and data
//! frames, `core_from_start`, and the demo session's options and init.

use super::*;

/// (RP3/T4) A data frame of the host's stream (`{"data":[…],"replay":…}`), typed. `msg`
/// only tells a message frame apart (it takes the message path).
#[derive(serde::Deserialize)]
pub(super) struct DataFrame {
    #[serde(default)]
    pub(super) msg: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    pub(super) data: Option<Vec<DataEvent>>,
    #[serde(default)]
    pub(super) replay: Option<ReplayState>,
}

/// (R6/T3, §24) The host's market status frame (`{"market":MarketStatus}`).
#[derive(serde::Deserialize)]
pub(super) struct MarketFrame {
    pub(super) market: qp_schema::market::MarketStatus,
}

/// (RP3/M15, §22) The host's `bind` frame (`{"bind":{feeds, events}}`), typed.
#[derive(serde::Deserialize)]
pub(super) struct BindFrame {
    pub(super) bind: BindBody,
}

/// (RP3/M15) The body of a `bind` frame.
#[derive(serde::Deserialize)]
pub(super) struct BindBody {
    pub(super) feeds: Vec<qp_schema::protocol::InitFeed>,
    #[serde(default)]
    pub(super) events: Vec<DataEvent>,
}

/// (RP3/T4) The host's `start` frame (`{conn, principal, lang, data, events, …}`), typed.
#[derive(serde::Deserialize)]
pub(super) struct StartFrame {
    #[serde(default)]
    principal: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    data: InitData,
    #[serde(default)]
    events: Vec<DataEvent>,
    /// (RP3/M15) `viewer`: the viewer core (no bars, no events; `instruments`).
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    instruments: Vec<String>,
}

/// (RP3/T4) The core of a page from the host's `start` frame (`{conn, principal, lang,
/// data, events}`; principal `arkin` and language `tr` when absent or not text, no events
/// when they are malformed), parsed typed in one pass; a frame that does not parse typed
/// takes the JSON-tree path, with the same result. The tree of a start frame costs some
/// 35 times its text (natively 200 MB for the 5.8 MB frame at step 1,250) and the WASM
/// memory keeps that peak (`rounds/RP3/experiments/T4.md`).
///
/// # Errors
/// A malformed frame or provider description.
pub fn core_from_start(start_json: &str) -> Result<ShellCore, String> {
    let mut core = core_from_start_parsed(start_json)?;
    core.count_frame("start", start_json.len());
    Ok(core)
}

fn core_from_start_parsed(start_json: &str) -> Result<ShellCore, String> {
    if let Ok(st) = serde_json::from_str::<StartFrame>(start_json) {
        let principal = st.principal.as_deref().unwrap_or("arkin");
        let lang = st.lang.as_deref().unwrap_or("tr");
        // (RP3/M15, §22) A viewer's start frame carries no data of the universe.
        if st.mode.as_deref() == Some("viewer") {
            return ShellCore::new_viewer(principal, lang, &st.instruments);
        }
        return ShellCore::new(principal, lang, &st.data, &st.events);
    }
    let v: Value = serde_json::from_str(start_json).map_err(|e| e.to_string())?;
    if v["mode"] == "viewer" {
        let instruments: Vec<String> =
            serde_json::from_value(v["instruments"].clone()).unwrap_or_default();
        return ShellCore::new_viewer(
            v["principal"].as_str().unwrap_or("arkin"),
            v["lang"].as_str().unwrap_or("tr"),
            &instruments,
        );
    }
    let data: InitData = serde_json::from_value(v["data"].clone()).map_err(|e| e.to_string())?;
    let events: Vec<DataEvent> = serde_json::from_value(v["events"].clone()).unwrap_or_default();
    ShellCore::new(
        v["principal"].as_str().unwrap_or("arkin"),
        v["lang"].as_str().unwrap_or("tr"),
        &data,
        &events,
    )
}

/// Queue of frames (used by drivers that hold frames between steps).
pub type Frames = VecDeque<String>;

// ---------------------------------------------------------------------------
// The demo session
// ---------------------------------------------------------------------------

/// Principal of the demo whose data entitlement is withheld ("no data entitlement"
/// mode of the settings page; an owner, so it may still drive the replay).
pub const VISITOR: &str = "visitor";

/// What `qp demo` starts (host configuration, not protocol).
#[derive(Debug, Clone)]
pub struct DemoOptions {
    /// Local data (falls back to the synthetic market with a note) or synthetic.
    pub local: bool,
    /// Instruments.
    pub instruments: u32,
    /// Bars of history before the first replayed bar.
    pub history: u32,
    /// Synthetic trading days.
    pub days: u32,
    /// Steps per wall second.
    pub speed: f64,
    /// Start of the replay (ms), if given.
    pub from_t: Option<i64>,
    /// Start running (else paused).
    pub running: bool,
    /// (R5/M19, K53) The host's feed rule by name (`qp_capsule::feedrule::FEED_RULES`;
    /// default the presentation's, `PRESENTED_FEED_RULE`, with the K42 exemption); `None`:
    /// no rule (the protocol's default).
    pub feed_rule: Option<String>,
    /// (RP4, `docs/CAPSULE.md` §9.9) The capsule engine's named constant record
    /// (`init.capsules.engine`: `r3`, `rp4`); `None`: not written (the protocol default
    /// `r3`, existing dumps byte-identical).
    pub engine: Option<String>,
}

impl Default for DemoOptions {
    fn default() -> Self {
        Self {
            local: false,
            instruments: 30,
            history: 120,
            days: 260,
            speed: 4.0,
            from_t: None,
            running: false,
            feed_rule: Some(qp_capsule::feedrule::PRESENTED_FEED_RULE.to_owned()),
            engine: None,
        }
    }
}

/// The demo's session init: `qp_session::demo_init` (owner `arkin`, host, agent `scout`,
/// viewer `guest`) plus the owner [`VISITOR`] without data entitlement; every feed bound,
/// the capsule engine on all of them; daily bars moved by hourly bars.
#[must_use]
pub fn demo_session_init(o: &DemoOptions) -> qp_schema::protocol::SessionInit {
    use qp_schema::protocol::{ReplayInit, ReplaySourceKind};
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin", "scout", "guest"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    ent.insert(VISITOR.to_owned(), Vec::new());
    let mut init = qp_session::demo_init(ReplayInit {
        source: if o.local {
            ReplaySourceKind::Local
        } else {
            ReplaySourceKind::Synthetic
        },
        instruments: Some(o.instruments),
        days: Some(o.days),
        history: Some(o.history),
        fine: Some("1h".to_owned()),
        from_t: o.from_t,
        speed: Some(o.speed),
        running: o.running,
        entitlements: ent,
        ..ReplayInit::default()
    });
    let visitor: qp_schema::protocol::Principal = serde_json::from_value(json!(
        {"id": VISITOR, "author": {"kind": "human", "name": VISITOR}, "role": "owner"}))
    .expect("visitor principal");
    init.principals.push(visitor);
    init.feed_rule = o
        .feed_rule
        .as_ref()
        .map(|rule| qp_schema::protocol::FeedRuleInit {
            rule: rule.clone(),
            exempt_negative: true,
        });
    if let Some(e) = &o.engine {
        init.capsules.get_or_insert_with(Default::default).engine = Some(e.clone());
    }
    init
}
