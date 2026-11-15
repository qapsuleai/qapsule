//! The source provider ("Veri sağlayıcı", `SourceProvider`; `docs/PROTOCOL.md` §18.3):
//! the host-supplied interface that resolves a data binding for a principal, and the
//! binding validation of §18.2 / §18.7.
//!
//! The trait is object-safe and assumes no I/O, threads or clock: a host answers from
//! whatever it has (a cache, a connector's state) and reports `Pending` until data is
//! there. [`MemoryProvider`] is the in-memory test/host provider built from a
//! transcript's `init.data` (§18.6).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qp_schema::data::{BindingState, DataBinding, DataEvent, FeedKey};
use qp_schema::protocol::InitData;
use qp_schema::{Bar, TimestampMs};

use crate::feed::{Applied, EventError, FeedStore};
use crate::window::resolve_window;
use crate::{resolve_binding_path, synthetic_bars, FileSource, SourceError};

/// A provider's answer for one binding and principal.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// The window's bars and the feed's source position they reflect.
    Ready {
        /// Bars of the window (strictly increasing `t`).
        bars: Vec<Bar>,
        /// Source sequence number of the last event these bars include.
        seq: u64,
    },
    /// No answer yet.
    Pending,
    /// The principal may not see this feed.
    NoEntitlement {
        /// Why (for the `binding` query's `reason`).
        reason: String,
    },
    /// No data for this binding (unknown symbol, empty window).
    NoData {
        /// Why.
        reason: String,
    },
}

impl Resolution {
    /// The binding state this answer means.
    #[must_use]
    pub fn state(&self) -> BindingState {
        match self {
            Self::Ready { .. } => BindingState::Ready,
            Self::Pending => BindingState::Pending,
            Self::NoEntitlement { .. } => BindingState::NoEntitlement,
            Self::NoData { .. } => BindingState::NoData,
        }
    }
}

/// The host's data source as the engine sees it (§18.3). Object-safe; no I/O
/// assumptions (wasm-friendly).
pub trait SourceProvider {
    /// True if `principal` may see `feed` (checked on every read, §18.3 item 2).
    fn entitled(&self, principal: &str, feed: &FeedKey) -> bool;

    /// Resolves an already validated binding (see [`validate_binding`]) for `principal`:
    /// entitlement check, then the window's bars. The engine applies the
    /// `max_bind_bars` cap itself ([`bind`]).
    fn resolve(&self, principal: &str, binding: &DataBinding) -> Resolution;
}

/// Checks a symbol or feed id: `^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$`.
///
/// # Errors
/// A message naming the rule.
pub fn validate_id(what: &str, s: &str) -> Result<(), String> {
    let ok = !s.is_empty()
        && s.len() <= 32
        && s.bytes().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{what}` `{s}` must match ^[A-Za-z0-9][A-Za-z0-9._-]{{0,31}}$"
        ))
    }
}

/// Checks an interval against the closed set of `qp_calendar::Interval::parse`
/// (`1m`…`1440m`, `1h`…`24h`, `1d`, `1w`, `1M`). Same rules, implemented here so this
/// crate (and the wasm build under it) does not pull in the calendar tables; a test
/// compares both on every candidate.
///
/// # Errors
/// A message naming the value.
pub fn validate_interval(s: &str) -> Result<(), String> {
    let err = || {
        Err(format!(
            "`interval` `{s}` must be one of 1m…1440m, 1h…24h, 1d, 1w, 1M"
        ))
    };
    let Some(split) = s.find(|c: char| !c.is_ascii_digit()) else {
        return err();
    };
    let (num, unit) = s.split_at(split);
    let Ok(n) = num.parse::<u32>() else {
        return err();
    };
    match (unit, n) {
        (_, 0) => err(),
        ("m", n) if n <= 1440 => Ok(()),
        ("h", n) if n <= 24 => Ok(()),
        ("d" | "w" | "M", 1) => Ok(()),
        _ => err(),
    }
}

/// Checks an entitlement entry (§18.3): `*` (everything), `SYMBOL` (that symbol on any
/// feed), `feed/SYMBOL` (that symbol on that named feed only) or `feed/*` (every symbol
/// of that named feed).
///
/// # Errors
/// A message naming the entry.
pub fn validate_entitlement(entry: &str) -> Result<(), String> {
    let bad = |_| format!("entitlement `{entry}` must be `*`, `SYMBOL`, `feed/SYMBOL` or `feed/*`");
    match entry.split_once('/') {
        _ if entry == "*" => Ok(()),
        None => validate_id("symbol", entry).map_err(bad),
        Some((feed, sym)) => {
            validate_id("feed", feed).map_err(bad)?;
            if sym == "*" {
                Ok(())
            } else {
                validate_id("symbol", sym).map_err(bad)
            }
        }
    }
}

/// True if entitlement `entry` covers `feed` (§18.3). A plain `SYMBOL` covers that symbol
/// on every feed, the host's default feed included; `feed/…` entries cover only bindings
/// that name that feed. Entries that fail [`validate_entitlement`] cover nothing.
#[must_use]
pub fn entitlement_matches(entry: &str, feed: &FeedKey) -> bool {
    if validate_entitlement(entry).is_err() {
        return false;
    }
    match entry.split_once('/') {
        _ if entry == "*" => true,
        None => entry == feed.symbol,
        Some((f, sym)) => feed.feed.as_deref() == Some(f) && (sym == "*" || sym == feed.symbol),
    }
}

/// Validates a binding from outside (§18.7): symbol and feed pattern, interval set,
/// window rules with `last <= max_bind_bars`.
///
/// # Errors
/// A message naming the offending field.
pub fn validate_binding(b: &DataBinding, max_bind_bars: u32) -> Result<(), String> {
    validate_id("symbol", &b.symbol)?;
    if let Some(f) = &b.feed {
        validate_id("feed", f)?;
    }
    validate_interval(&b.interval)?;
    b.window.validate(max_bind_bars)
}

/// A binding resolved for one principal, ready for the engine.
#[derive(Debug, Clone)]
pub struct Bound {
    /// The state at resolution (see [`Bound::current_state`] afterwards).
    pub state: BindingState,
    /// Why it is not ready, or that the window was capped.
    pub reason: Option<String>,
    /// Time of bar 0 (only when ready).
    pub anchor_t: Option<TimestampMs>,
    /// The bars, with window bounds set (only when ready).
    pub store: Option<FeedStore>,
}

impl Bound {
    /// The state now: the store's (`ready`, or `stale` after a gap or at the live growth
    /// cap) when there is one, else the resolution's.
    #[must_use]
    pub fn current_state(&self) -> BindingState {
        self.store.as_ref().map_or(self.state, FeedStore::state)
    }

    /// The `reason` now (the store's staleness, else the resolution's).
    #[must_use]
    pub fn current_reason(&self) -> Option<String> {
        self.store
            .as_ref()
            .and_then(FeedStore::stale)
            .map(|r| r.reason())
            .or_else(|| self.reason.clone())
    }
}

/// Validates `binding`, asks `provider` for it on behalf of `principal` and applies the
/// engine's own window rules to the answer ([`resolve_window`] with the binding's
/// `anchor_t`): bars outside the window are dropped; without an anchor a selection over
/// `max_bind_bars` keeps the latest ones (`reason` says so); with an anchor such a
/// selection is `stale` (reason: limit) because the anchor must stay bar 0; an empty
/// result is `no_data`. The store is capped at `2 × max_bind_bars` bars for live growth
/// (§18.7).
///
/// # Errors
/// An invalid binding (message names the field), or provider bars that break the bar
/// rules (non-increasing, non-finite, inconsistent).
pub fn bind(
    provider: &dyn SourceProvider,
    principal: &str,
    binding: &DataBinding,
    max_bind_bars: u32,
) -> Result<Bound, String> {
    validate_binding(binding, max_bind_bars)?;
    let not_ready = |state, reason| Bound {
        state,
        reason: Some(reason),
        anchor_t: None,
        store: None,
    };
    match provider.resolve(principal, binding) {
        Resolution::Pending => Ok(Bound {
            state: BindingState::Pending,
            reason: None,
            anchor_t: None,
            store: None,
        }),
        Resolution::NoEntitlement { reason } => Ok(not_ready(BindingState::NoEntitlement, reason)),
        Resolution::NoData { reason } => Ok(not_ready(BindingState::NoData, reason)),
        Resolution::Ready { mut bars, seq } => {
            let w = resolve_window(&bars, &binding.window, binding.anchor_t, max_bind_bars)?;
            if w.anchor_missing {
                // Bar numbers would not match the authority's (§18.3): stale, no values.
                return Ok(Bound {
                    state: BindingState::Stale,
                    reason: Some(format!(
                        "the source has no bar at the anchor t={}",
                        binding.anchor_t.unwrap_or_default()
                    )),
                    anchor_t: binding.anchor_t,
                    store: None,
                });
            }
            if w.is_empty() {
                return Ok(not_ready(
                    BindingState::NoData,
                    "the window selects no bars".to_owned(),
                ));
            }
            if let Some(reason) = w.limit {
                return Ok(Bound {
                    state: BindingState::Stale,
                    reason: Some(reason),
                    anchor_t: binding.anchor_t,
                    store: None,
                });
            }
            let reason = w.capped;
            bars.truncate(w.range.end);
            bars.drain(..w.range.start);
            let max_bars = usize::try_from(max_bind_bars)
                .unwrap_or(usize::MAX)
                .saturating_mul(2);
            let store = FeedStore::with_bars(binding.feed_key(), bars, seq)
                .map_err(|e| format!("provider data: {e}"))?
                .with_bounds(w.bounds)
                .with_max_bars(max_bars);
            Ok(Bound {
                state: BindingState::Ready,
                reason,
                anchor_t: w.anchor_t,
                store: Some(store),
            })
        }
    }
}

/// An in-memory provider: feeds with their bars, per-principal entitlements and feeds
/// that never answer. It also takes data events, so later resolutions see them.
#[derive(Debug, Clone, Default)]
pub struct MemoryProvider {
    feeds: BTreeMap<FeedKey, FeedStore>,
    entitlements: BTreeMap<String, BTreeSet<String>>,
    pending: BTreeSet<FeedKey>,
}

impl MemoryProvider {
    /// An empty provider (no feeds, nobody entitled).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// From a transcript's `init.data` (§18.6). `path` feeds are `Series` JSON files
    /// confined to `root` ([`resolve_binding_path`]).
    ///
    /// # Errors
    /// A feed with not exactly one of `bars` / `path` / `synthetic`, an invalid feed key,
    /// a duplicate feed, an unreadable or escaping path, invalid bars.
    pub fn from_init(init: &InitData, root: &Path) -> Result<Self, SourceError> {
        let mut p = Self::new();
        for f in &init.feeds {
            let name = format!("feed {}/{}", f.feed.symbol, f.feed.interval);
            check_feed_key(&f.feed).map_err(|e| SourceError::new(format!("{name}: {e}")))?;
            let bars = match (&f.bars, &f.path, &f.synthetic) {
                (Some(b), None, None) => b.clone(),
                (None, Some(path), None) => {
                    let full = resolve_binding_path(root, path)?;
                    crate::collect_bars(&mut FileSource::open(&full)?)?
                }
                (None, None, Some(s)) => synthetic_bars(s.n as usize, s.seed),
                _ => {
                    return Err(SourceError::new(format!(
                        "{name}: give exactly one of `bars`, `path`, `synthetic`"
                    )))
                }
            };
            p.add_feed(f.feed.clone(), bars)
                .map_err(|e| SourceError::new(format!("{name}: {e}")))?;
        }
        for (principal, symbols) in &init.entitlements {
            p.entitle(principal, symbols.iter().cloned())?;
        }
        for k in &init.pending {
            p.set_pending(k.clone());
        }
        Ok(p)
    }

    /// Adds a feed with its initial bars at `seq` 0.
    ///
    /// # Errors
    /// The feed exists already, or the bars break the bar rules.
    pub fn add_feed(&mut self, feed: FeedKey, bars: Vec<Bar>) -> Result<(), SourceError> {
        if self.feeds.contains_key(&feed) {
            return Err(SourceError::new("duplicate feed"));
        }
        let store = FeedStore::with_bars(feed.clone(), bars, 0)
            .map_err(|e| SourceError::new(e.to_string()))?;
        self.feeds.insert(feed, store);
        Ok(())
    }

    /// Grants `principal` these entitlement entries (see [`entitlement_matches`]), in
    /// addition to earlier grants.
    ///
    /// # Errors
    /// An entry that is not `*`, `SYMBOL`, `feed/SYMBOL` or `feed/*`; nothing is granted.
    pub fn entitle(
        &mut self,
        principal: &str,
        entries: impl IntoIterator<Item = String>,
    ) -> Result<(), SourceError> {
        let entries: Vec<String> = entries.into_iter().collect();
        for e in &entries {
            validate_entitlement(e)
                .map_err(|m| SourceError::new(format!("principal `{principal}`: {m}")))?;
        }
        self.entitlements
            .entry(principal.to_owned())
            .or_default()
            .extend(entries);
        Ok(())
    }

    /// Makes `feed` answer `Pending` (never resolved).
    pub fn set_pending(&mut self, feed: FeedKey) {
        self.pending.insert(feed);
    }

    /// The store of a feed.
    #[must_use]
    pub fn feed(&self, feed: &FeedKey) -> Option<&FeedStore> {
        self.feeds.get(feed)
    }

    /// Delivers a data event to its feed's store (host path).
    ///
    /// # Errors
    /// [`EventError::UnknownFeed`], or the store's rule errors.
    pub fn apply(&mut self, ev: &DataEvent) -> Result<Applied, EventError> {
        match self.feeds.get_mut(&ev.feed) {
            Some(s) => s.apply(ev),
            None => Err(EventError::UnknownFeed {
                feed: Box::new(ev.feed.clone()),
            }),
        }
    }
}

fn check_feed_key(k: &FeedKey) -> Result<(), String> {
    validate_id("symbol", &k.symbol)?;
    if let Some(f) = &k.feed {
        validate_id("feed", f)?;
    }
    validate_interval(&k.interval)
}

impl SourceProvider for MemoryProvider {
    fn entitled(&self, principal: &str, feed: &FeedKey) -> bool {
        self.entitlements
            .get(principal)
            .is_some_and(|s| s.iter().any(|e| entitlement_matches(e, feed)))
    }

    fn resolve(&self, principal: &str, binding: &DataBinding) -> Resolution {
        let key = binding.feed_key();
        if !self.entitled(principal, &key) {
            return Resolution::NoEntitlement {
                reason: format!(
                    "principal `{principal}` has no data entitlement for `{}`",
                    key.symbol
                ),
            };
        }
        if self.pending.contains(&key) {
            return Resolution::Pending;
        }
        let Some(store) = self.feeds.get(&key) else {
            return Resolution::NoData {
                reason: format!("no feed {}/{}", key.symbol, key.interval),
            };
        };
        match resolve_window(store.bars(), &binding.window, binding.anchor_t, u32::MAX) {
            Ok(w) if !w.is_empty() => Resolution::Ready {
                bars: w.slice(store.bars()).to_vec(),
                seq: store.seq(),
            },
            Ok(_) => Resolution::NoData {
                reason: "the window selects no bars".to_owned(),
            },
            Err(e) => Resolution::NoData { reason: e },
        }
    }
}
