//! The data plane inside the authority (RP2, K23; `docs/PROTOCOL.md` §18).
//!
//! The document carries bindings, never bound bars. The authority keeps, **outside the
//! document**:
//!
//! - the host's source provider ([`Provider`]): from `init.data` (a
//!   [`MemoryProvider`], test host) or handed in by the host
//!   ([`crate::Session::with_provider`]);
//! - per bound series, its binding as resolved and a [`Bound`] (state, reason, anchor and
//!   a [`qp_source::FeedStore`] with the window's bars, its bounds and live growth cap),
//!   resolved with the entitlement of the first principal whose role is `host` (§18.3);
//! - nothing per asker: reads are filtered per asker with [`DataPlane::gate`] (data
//!   plane values do not go to an unentitled asker; the meaning plane does).
//!
//! The session reads bound bars through a *reading document* (`Session::rd`): the
//! document with each ready bound series' bars filled in. It is never serialised, never
//! logged and never sent; every bar reader (values cache, conditions, queries, grammar
//! provider, presenter) takes it instead of the document, so hesap ve görünüm kodu
//! bağlı ile belge içi seriyi ayırt etmez (§18.2).

use std::collections::BTreeMap;
use std::fmt;

use qp_schema::data::{
    data_digest, BindingState, DataBinding, DataEvent, EvidenceRef, FeedKey, DATA_DIGEST_ALG,
};
use qp_schema::protocol::{ErrorCode, ProtocolError};
use qp_schema::{Bar, Document, Series, SeriesData, SeriesSource};
use qp_source::{bind, Applied, Bound, EventError, MemoryProvider, SourceProvider};

use crate::error::E;
use crate::values::{find_series, Delta};

/// The host's source provider as the session holds it.
pub enum Provider {
    /// No provider: bindings stay `pending`.
    None,
    /// The test/simple host provider built from `init.data`; it also takes the data
    /// events the host delivers, so later resolutions see them.
    Memory(MemoryProvider),
    /// A provider handed in by the host (the host keeps it up to date itself).
    Host(Box<dyn SourceProvider>),
}

impl Provider {
    pub(crate) fn get(&self) -> Option<&dyn SourceProvider> {
        match self {
            Self::None => None,
            Self::Memory(m) => Some(m),
            Self::Host(h) => Some(h.as_ref()),
        }
    }
}

/// One bound series as the authority resolved it.
pub struct BoundSeries {
    /// The binding as in the document (its `anchor_t` follows the stamping entry).
    pub binding: DataBinding,
    /// The resolution (state, reason, anchor, bars).
    pub bound: Bound,
}

/// A bound series is not ready for an asker (§18.3).
#[derive(Debug, Clone)]
pub struct Blocked {
    /// The bound series the data rests on.
    pub series: String,
    /// Its state for the asker.
    pub state: BindingState,
    /// Why, in words.
    pub reason: String,
}

pub fn state_str(s: BindingState) -> &'static str {
    match s {
        BindingState::Ready => "ready",
        BindingState::Pending => "pending",
        BindingState::NoEntitlement => "no_entitlement",
        BindingState::NoData => "no_data",
        BindingState::Stale => "stale",
    }
}

impl Blocked {
    /// `principal` has no data entitlement for `key`, the feed of bound series `series`.
    pub fn unentitled(series: &str, principal: &str, key: &FeedKey) -> Self {
        Self {
            series: series.to_owned(),
            state: BindingState::NoEntitlement,
            reason: no_entitlement_reason(principal, key),
        }
    }

    /// The protocol error for a read that needs this data, at `path` (§18.3):
    /// `no_entitlement` for a missing entitlement, `no_data` otherwise (pending, no data,
    /// stale).
    pub fn error(&self, about: &str, path: &str) -> ProtocolError {
        let code = if self.state == BindingState::NoEntitlement {
            ErrorCode::NoEntitlement
        } else {
            ErrorCode::NoData
        };
        let rests = if about == self.series {
            format!("bound series `{about}`")
        } else {
            format!("`{about}` rests on bound series `{}`, which", self.series)
        };
        let hint = match self.state {
            BindingState::NoEntitlement => {
                "The data licence of this feed does not cover you; the rest of the document works. Ask the host for an entitlement."
            }
            BindingState::Pending => "The data provider has not answered yet; ask again later.",
            BindingState::Stale => "The feed is stale until the host re-resolves the binding.",
            BindingState::NoData | BindingState::Ready => {
                "Check the binding (`binding` query): symbol, interval, window."
            }
        };
        E::new(
            code,
            format!("{rests} is `{}`: {}", state_str(self.state), self.reason),
        )
        .at(path)
        .hint(hint)
        .build()
    }
}

/// A data event the authority could not take (§18.4: not applied, reported to the
/// host; the document and the connections are unaffected).
#[derive(Debug, Clone, PartialEq)]
pub enum DataError {
    /// Neither a bound series nor the session's own provider knows the event's feed.
    UnknownFeed(FeedKey),
    /// The event breaks the rules (§18.4) for this bound series.
    Rejected {
        /// The bound series whose store rejected it (absent: the provider's store).
        series: Option<String>,
        /// Why.
        error: EventError,
    },
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFeed(k) => {
                write!(f, "no bound series and no feed {}/{}", k.symbol, k.interval)
            }
            Self::Rejected {
                series: Some(s),
                error,
            } => write!(f, "event rejected for bound series `{s}`: {error}"),
            Self::Rejected {
                series: None,
                error,
            } => write!(f, "event rejected: {error}"),
        }
    }
}

impl std::error::Error for DataError {}

/// What one data event did to the bound series (by id).
pub struct EventOutcome {
    /// Bound series whose bars or state changed, and how (their reading copy and the
    /// values on them follow, §18.8).
    pub changed: Vec<(String, Delta)>,
    /// What the host must be told (§18.4: a gap; a store that became stale).
    pub notices: Vec<HostNotice>,
}

/// A notice for the host about a bound series (§3, §18.4): not a protocol message and
/// not a connection's output; `qp serve --host-frames` writes it as
/// `{"host":{"gap":{…}}}` or `{"host":{"stale":{…}}}`. After one, the binding is
/// `stale` until the host re-resolves it ([`crate::Session::refresh_bindings`]).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostNotice {
    /// A `seq` gap: the event was not applied and the binding became `stale`.
    Gap {
        /// The bound series.
        series: String,
        /// Its feed.
        feed: FeedKey,
        /// The sequence number the authority waited for.
        expected: u64,
        /// The sequence number that arrived.
        got: u64,
    },
    /// The binding became `stale` for another reason (the live growth cap, §18.7).
    Stale {
        /// The bound series.
        series: String,
        /// Its feed.
        feed: FeedKey,
        /// Why, in words.
        reason: String,
    },
}

/// What a host data event caused ([`crate::Session::host_data_event`]).
#[derive(Debug, Clone, Default)]
pub struct DataOutcome {
    /// Notifications for the connections (§18.4, §18.8).
    pub lines: Vec<qp_schema::protocol::OutboundLine>,
    /// Notices for the host (a gap, a binding that became stale).
    pub notices: Vec<HostNotice>,
}

impl HostNotice {
    /// The host frame for this notice (`{"host":{"gap":{…}}}`, `{"host":{"stale":{…}}}`).
    #[must_use]
    pub fn frame(&self) -> serde_json::Value {
        serde_json::json!({ "host": self })
    }
}

/// The authority's data plane.
pub struct DataPlane {
    pub provider: Provider,
    /// The principal whose entitlement the authority's own work uses: the first one with
    /// role `host` (§18.3).
    pub host: Option<String>,
    /// Bound series id → its resolution.
    pub series: BTreeMap<String, BoundSeries>,
    pub max_bind_bars: u32,
    /// (R5B, §25.7) Agents that read for a person: agent id → the person. Such an agent
    /// is entitled to a feed only if the person is and, unless the provider knows the
    /// agent has no row of its own, the agent is too (the intersection; fail closed).
    pub delegates: BTreeMap<String, String>,
    /// (R5B, §25.4) The comment validator's strictness (`init.comment_strictness`); kept
    /// here so that the expansion stamps it on the stored comment.
    pub comment_strictness: qp_schema::comment::Strictness,
}

/// The binding of a series, if it is bound.
pub fn binding_of(s: &Series) -> Option<DataBinding> {
    s.binding()
}

/// True if `s` is a bar series by its kind (inline bars or a binding), whether or not
/// its bars are available now.
pub fn is_bar_series(s: &Series) -> bool {
    s.bars().is_some() || matches!(s.source, SeriesSource::Bind { .. })
}

/// The bar series `series` rests on, by kind (itself if it is one, else the first input
/// of its study, recursively).
pub fn bar_root<'a>(doc: &'a Document, series: &str) -> Option<&'a Series> {
    let mut cur = find_series(doc, series)?;
    for _ in 0..64 {
        if is_bar_series(cur) {
            return Some(cur);
        }
        match &cur.source {
            SeriesSource::Study { inputs, .. } => cur = find_series(doc, inputs.first()?)?,
            _ => return None,
        }
    }
    None
}

fn no_entitlement_reason(principal: &str, key: &FeedKey) -> String {
    let feed = key
        .feed
        .as_deref()
        .map_or_else(String::new, |f| format!("{f}/"));
    format!(
        "principal `{principal}` has no data entitlement for `{feed}{}`",
        key.symbol
    )
}

pub fn same_but_anchor(a: &DataBinding, b: &DataBinding) -> bool {
    a.symbol == b.symbol && a.interval == b.interval && a.window == b.window && a.feed == b.feed
}

fn not_ready(state: BindingState, reason: String) -> Bound {
    Bound {
        state,
        reason: Some(reason),
        anchor_t: None,
        store: None,
    }
}

impl DataPlane {
    pub fn new(provider: Provider, host: Option<String>, max_bind_bars: u32) -> Self {
        Self {
            provider,
            host,
            series: BTreeMap::new(),
            max_bind_bars,
            delegates: BTreeMap::new(),
            comment_strictness: qp_schema::comment::Strictness::CitesNumbers,
        }
    }

    fn resolve(&self, b: &DataBinding) -> Bound {
        let Some(p) = self.provider.get() else {
            return not_ready(
                BindingState::Pending,
                "the host gave the authority no data provider".to_owned(),
            );
        };
        let Some(h) = &self.host else {
            return not_ready(
                BindingState::NoEntitlement,
                "no principal has the role `host`; the authority resolves bindings with its entitlement"
                    .to_owned(),
            );
        };
        match bind(p, h, b, self.max_bind_bars) {
            Ok(bound) => bound,
            Err(e) => not_ready(BindingState::NoData, format!("provider data: {e}")),
        }
    }

    /// Brings the plane in line with the document: bound series that are new or whose
    /// binding changed are resolved, others dropped. A binding whose only change is the
    /// authority's own anchor stamp keeps its resolution. Returns the anchors to stamp
    /// (series id, `anchor_t`): ready resolutions of bindings without `anchor_t` (§18.3).
    pub fn sync(&mut self, doc: &Document) -> Vec<(String, i64)> {
        let wanted: BTreeMap<String, DataBinding> = doc
            .series
            .iter()
            .filter_map(|s| binding_of(s).map(|b| (s.id.clone(), b)))
            .collect();
        self.series.retain(|id, bs| match wanted.get(id) {
            Some(w) if *w == bs.binding => true,
            Some(w)
                if bs.binding.anchor_t.is_none()
                    && w.anchor_t.is_some()
                    && w.anchor_t == bs.bound.anchor_t
                    && same_but_anchor(w, &bs.binding) =>
            {
                bs.binding = w.clone();
                true
            }
            _ => false,
        });
        for (id, w) in wanted {
            if !self.series.contains_key(&id) {
                let bound = self.resolve(&w);
                self.series.insert(id, BoundSeries { binding: w, bound });
            }
        }
        self.stamps()
    }

    fn stamps(&self) -> Vec<(String, i64)> {
        self.series
            .iter()
            .filter(|(_, bs)| {
                bs.binding.anchor_t.is_none() && bs.bound.current_state() == BindingState::Ready
            })
            .filter_map(|(id, bs)| bs.bound.anchor_t.map(|t| (id.clone(), t)))
            .collect()
    }

    /// Resolves again every binding that is not ready (host API after a gap, a limit or
    /// a provider that answered late). Returns the anchors to stamp and the series whose
    /// state or bars changed.
    pub fn refresh(&mut self) -> (Vec<(String, i64)>, Vec<String>) {
        let ids: Vec<String> = self
            .series
            .iter()
            .filter(|(_, bs)| bs.bound.current_state() != BindingState::Ready)
            .map(|(id, _)| id.clone())
            .collect();
        let mut changed = Vec::new();
        for id in ids {
            let b = self.series[&id].binding.clone();
            let bound = self.resolve(&b);
            if let Some(bs) = self.series.get_mut(&id) {
                bs.bound = bound;
            }
            changed.push(id);
        }
        (self.stamps(), changed)
    }

    /// Ready bars of a bound series (the host's resolution).
    pub fn bars(&self, id: &str) -> Option<&[Bar]> {
        let bs = self.series.get(id)?;
        if bs.bound.current_state() != BindingState::Ready {
            return None;
        }
        bs.bound.store.as_ref().map(qp_source::FeedStore::bars)
    }

    /// The host's state and reason of a bound series.
    pub fn host_state(&self, id: &str) -> Option<(BindingState, Option<String>)> {
        self.series
            .get(id)
            .map(|bs| (bs.bound.current_state(), bs.bound.current_reason()))
    }

    /// True if `asker` may read `feed` (data entitlement, §18.3 item 2).
    /// (R5B, §25.7) An agent that works for a person reads with the person's
    /// entitlement, intersected with its own row when it has one.
    pub fn entitled(&self, asker: &str, feed: &FeedKey) -> bool {
        let Some(p) = self.provider.get() else {
            return false;
        };
        match self.delegates.get(asker) {
            Some(person) => {
                p.entitled(person, feed)
                    && (p.has_row(asker) == Some(false) || p.entitled(asker, feed))
            }
            None => p.entitled(asker, feed),
        }
    }

    /// The state of bound series `id` for `asker` (`None`: the authority's own view).
    pub fn state_for(
        &self,
        id: &str,
        asker: Option<&str>,
    ) -> Option<(BindingState, Option<String>)> {
        let bs = self.series.get(id)?;
        if let Some(a) = asker {
            let key = bs.binding.feed_key();
            if !self.entitled(a, &key) {
                return Some((
                    BindingState::NoEntitlement,
                    Some(no_entitlement_reason(a, &key)),
                ));
            }
        }
        let state = bs.bound.current_state();
        let reason = bs.bound.current_reason().or_else(|| {
            (state == BindingState::Pending)
                .then(|| "the data provider has not answered yet".to_owned())
        });
        Some((state, reason))
    }

    /// The first bound series `series` rests on (itself, or through every study input,
    /// recursively) whose feed `principal` has no data entitlement for (§18.3, the write
    /// side of a condition). It reads the document's bindings, not the plane's
    /// resolutions (a binding put earlier in the same command counts) and looks only at
    /// the entitlement, not at readiness. No provider: nobody is entitled.
    pub fn unentitled(&self, doc: &Document, principal: &str, series: &str) -> Option<Blocked> {
        let mut stack = vec![series.to_owned()];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id.clone()) || seen.len() > 1024 {
                continue;
            }
            let Some(s) = find_series(doc, &id) else {
                continue;
            };
            match &s.source {
                SeriesSource::Bind { .. } => {
                    let key = binding_of(s)?.feed_key();
                    if !self.entitled(principal, &key) {
                        return Some(Blocked {
                            series: id,
                            state: BindingState::NoEntitlement,
                            reason: no_entitlement_reason(principal, &key),
                        });
                    }
                }
                SeriesSource::Study { inputs, .. } => {
                    stack.extend(inputs.iter().rev().cloned());
                }
                _ => {}
            }
        }
        None
    }

    /// If the data of `series` (itself or, for a study, its bar input) rests on a bound
    /// series that is not ready for `asker` (the authority's own view for `None`): which,
    /// in what state and why (§18.3).
    pub fn gate(&self, doc: &Document, asker: Option<&str>, series: &str) -> Option<Blocked> {
        if self.series.is_empty() {
            return None;
        }
        // Every bound series the data rests on, through all study inputs (recursively).
        let mut stack = vec![series.to_owned()];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id.clone()) || seen.len() > 1024 {
                continue;
            }
            let Some(s) = find_series(doc, &id) else {
                continue;
            };
            match &s.source {
                SeriesSource::Bind { .. } => {
                    let (state, reason) = self
                        .state_for(&id, asker)
                        .unwrap_or((BindingState::Pending, Some("not resolved yet".to_owned())));
                    if state != BindingState::Ready {
                        return Some(Blocked {
                            series: id,
                            state,
                            reason: reason.unwrap_or_else(|| state_str(state).to_owned()),
                        });
                    }
                }
                SeriesSource::Study { inputs, .. } => {
                    stack.extend(inputs.iter().rev().cloned());
                }
                _ => {}
            }
        }
        None
    }

    /// The first series `layer` reads (its data, its facts' series, series named in
    /// channels, colour conditions and origins) that is not ready for `asker`.
    pub fn gate_layer(
        &self,
        doc: &Document,
        asker: Option<&str>,
        layer: &qp_schema::Layer,
    ) -> Option<Blocked> {
        if self.series.is_empty() {
            return None;
        }
        layer_reads(doc, layer)
            .iter()
            .find_map(|s| self.gate(doc, asker, s))
    }

    /// Applies a host data event to every bound series of its feed (§18.4), and to the
    /// session's own memory provider (so later resolutions see it).
    ///
    /// # Errors
    /// The event's feed is unknown, or a store rejects it (nothing of that store changes;
    /// stores already updated keep the event: their own rules accepted it).
    pub fn apply_event(&mut self, ev: &DataEvent) -> Result<EventOutcome, DataError> {
        let mut provider_knows = false;
        let mut provider_err = None;
        if let Provider::Memory(m) = &mut self.provider {
            match m.apply(ev) {
                Ok(_) => provider_knows = true,
                Err(EventError::UnknownFeed { .. }) => {}
                Err(e) => {
                    provider_knows = true;
                    provider_err = Some(e);
                }
            }
        }
        let mut changed = Vec::new();
        let mut notices = Vec::new();
        let mut any = false;
        let mut first_err = None;
        for (id, bs) in &mut self.series {
            if bs.binding.feed_key() != ev.feed {
                continue;
            }
            let Some(store) = bs.bound.store.as_mut() else {
                // Not resolved (pending, no entitlement, no data, anchored limit): the
                // event waits for the host's re-resolution.
                any = true;
                continue;
            };
            any = true;
            let before = store.state();
            match store.apply(ev) {
                Ok(Applied::NewBar { index }) => {
                    changed.push((id.clone(), Delta::Appended { from: index }));
                }
                Ok(Applied::LiveUpdate { index } | Applied::Correction { index }) => {
                    changed.push((id.clone(), Delta::Revised { index }));
                }
                Ok(a @ (Applied::Gap { .. } | Applied::Stale(_))) => {
                    if before != store.state() {
                        changed.push((id.clone(), Delta::Rebuild { from: 0 }));
                        notices.push(match a {
                            Applied::Gap { expected, got } => HostNotice::Gap {
                                series: id.clone(),
                                feed: ev.feed.clone(),
                                expected,
                                got,
                            },
                            Applied::Stale(r) => HostNotice::Stale {
                                series: id.clone(),
                                feed: ev.feed.clone(),
                                reason: r.reason(),
                            },
                            #[allow(clippy::unreachable)] // invariant: see outer arm
                            _ => unreachable!("matched above"),
                        });
                    }
                }
                Ok(Applied::Duplicate | Applied::OutsideWindow { .. }) => {}
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(DataError::Rejected {
                            series: Some(id.clone()),
                            error: e,
                        });
                    }
                }
            }
        }
        if let Some(e) = first_err {
            return Err(e);
        }
        if !any {
            if let Some(e) = provider_err {
                return Err(DataError::Rejected {
                    series: None,
                    error: e,
                });
            }
            if !provider_knows {
                return Err(DataError::UnknownFeed(ev.feed.clone()));
            }
        }
        Ok(EventOutcome { changed, notices })
    }
}

/// Every series a layer reads values from: its `series`; the series of its fact or of
/// every fact of its finding; series named in `x`/`y`/`y2` references and in a colour
/// condition's operand; series and fact origins of its constants (§18.3: a layer whose
/// data the asker may not read is not drawn).
pub fn layer_reads(doc: &Document, l: &qp_schema::Layer) -> Vec<String> {
    use qp_schema::grammar::{ColorSpec, Operand, Origin, Position};
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if !out.iter().any(|x| x == s) {
            out.push(s.to_owned());
        }
    };
    if let Some(s) = &l.series {
        add(s);
    }
    let fact_series = |id: &str| {
        doc.facts
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.series.clone())
    };
    let mut facts: Vec<String> = l.fact.iter().cloned().collect();
    if let Some(fnd) = &l.finding {
        if let Some(f) = doc.findings.iter().find(|x| &x.id == fnd) {
            facts.extend(f.facts.iter().cloned());
        }
    }
    for o in l.origin.values() {
        match o {
            Origin::Series { series, .. } => add(series),
            Origin::Fact { fact, .. } => facts.push(fact.clone()),
            Origin::Threshold { .. } => {}
        }
    }
    for f in &facts {
        if let Some(s) = fact_series(f) {
            add(&s);
        }
    }
    let e = &l.encoding;
    for p in [&e.x, &e.y, &e.y2].into_iter().flatten() {
        if let Position::Ref(r) = p {
            if let Some(s) = &r.series {
                add(s);
            }
        }
    }
    if let Some(ColorSpec::When(w)) = &e.color {
        if let Operand::Ref(r) = &w.when.than {
            if let Some(s) = &r.series {
                add(s);
            }
        }
    }
    out
}

/// Puts the ready bars of bound series `id` into the reading document (or removes them
/// when the binding is not ready).
pub fn inject(rd: &mut Document, plane: &DataPlane, id: &str) {
    let Some(s) = rd.series.iter_mut().find(|s| s.id == id) else {
        return;
    };
    if !matches!(s.source, SeriesSource::Bind { .. }) {
        return;
    }
    s.data = plane
        .bars(id)
        .map(|b| SeriesData::Bars { bars: b.to_vec() });
}

/// The reading document: `doc` with every ready bound series' bars (`None` when the
/// document has no bound series; readers then use the document itself).
pub fn reading_doc(doc: &Document, plane: &DataPlane) -> Option<Document> {
    if plane.series.is_empty() {
        return None;
    }
    let mut rd = doc.clone();
    let ids: Vec<String> = plane.series.keys().cloned().collect();
    for id in ids {
        inject(&mut rd, plane, &id);
    }
    Some(rd)
}

/// Read access to the data plane for command expansion (`add_fact`, `set_view_range`).
#[derive(Clone, Copy)]
pub struct Reader<'a> {
    /// The reading document as of the command's start.
    pub rd: &'a Document,
    /// The plane (entitlements, states).
    pub plane: &'a DataPlane,
}

/// The bars of the bar series of `series` as `doc` (a scratch document of a command)
/// describes it: inline bars from `doc`; a bound series' bars from the reading document
/// when its binding there is still the same. Returns the bar series and its bars.
pub fn bars_for<'a>(
    doc: &'a Document,
    data: Option<&Reader<'a>>,
    series: &str,
) -> Option<(&'a Series, &'a [Bar])> {
    let root = bar_root(doc, series)?;
    if let Some(b) = root.bars() {
        return Some((root, b));
    }
    let r = data?;
    let rs = find_series(r.rd, &root.id)?;
    let same = match (&rs.source, &root.source) {
        (a @ SeriesSource::Bind { .. }, b) => a == b,
        _ => false,
    };
    if !same {
        return None;
    }
    rs.bars().map(|b| (root, b))
}

/// Frozen evidence of a fact (§18.5): bars 0 to `to` (inclusive) of bar series `bs`.
///
/// # Panics
/// `to` out of range (callers check).
pub fn evidence_ref(bs: &Series, bars: &[Bar], to: usize) -> EvidenceRef {
    let range = &bars[..=to];
    EvidenceRef {
        series: bs.id.clone(),
        binding: binding_of(bs),
        from_t: range[0].t,
        to_t: range[to].t,
        bars: range.len() as u64,
        alg: DATA_DIGEST_ALG.to_owned(),
        digest: data_digest(range),
    }
}
