//! Series sources ("Seri kaynağı", `SeriesSource` in `docs/CONCEPTS.md`).
//!
//! A source delivers a bar series as a stream of [`BarEvent`]s: a new bar, or a revision
//! of the last bar (a live bar whose price keeps changing). Consumers (the compute core,
//! the renderer, the CLI) see only this trait and never the kind of source: a file, a
//! deterministic synthetic generator today; a Matriks feed or NATS subject later (R6).
//!
//! The events map one-to-one onto `qp_compute::Study::push` / `update_last`.
//!
//! The data plane (RP2, K23; `docs/PROTOCOL.md` §18) adds field-level [`DataEvent`]s
//! (`qp_schema::data`): [`FeedStore`] applies them by the §18.4 rules and keeps an
//! incremental digest for `DataPosition`; [`resolve_window`] resolves a binding's window;
//! [`SourceProvider`] is the host-supplied provider ([`MemoryProvider`] for tests and
//! simple hosts); [`SyntheticEvents`] generates a deterministic field-level stream.
//!
//! R3 (data for fact detectors and capsule measurements): [`synthetic_market`] generates
//! regime-driven XIST-aligned bars for many instruments, [`synthetic_flow`] a licensed
//! broker-flow feed (K33), and [`load_local_bars`] reads real bars fetched into
//! `fixtures/local/bars/` by `scripts/fetch-bars.sh` (K23).
//!
//! R4 (K44, `docs/PROTOCOL.md` §21): [`Replay`] delivers historical bars (local real
//! data or the synthetic market) like a live feed, as data events on a virtual timeline.
//!
//! R6 (M22): [`LiveBars`] builds calendar-aligned live bars from price-card updates
//! ([`CardTick`]) as data events; [`compare_bars`] measures them against REST bars.
//!
//! [`DataEvent`]: qp_schema::data::DataEvent
//!
//! Note on names: `qp_schema::SeriesSource` is the *description* of a binding inside a
//! document (`{"kind": "file", "path": ...}`); this crate's [`SeriesSource`] trait is the
//! runtime side that produces the data. [`open_binding`] turns the former into the latter.

mod events;
mod feed;
mod file;
mod flow;
mod livebar;
mod local;
mod provider;
mod regimes;
mod replay;
mod source;
mod synthetic;
mod window;

pub use events::{
    bar_patch_diff, data_events_from_bar_events, SyntheticEvents, SyntheticEventsConfig,
};
pub use feed::{Applied, EventError, FeedStore, StaleReason, TimeBounds};
pub use file::{open_binding, resolve_binding_path, FileSource, MAX_FILE_BYTES};
pub use flow::{synthetic_flow, FlowRecord, FLOW_KEYS};
pub use livebar::{
    compare_bars, compare_bars_with, BarDiff, BarMethod, CardTick, DiffTolerance, FieldDiff,
    LiveBars, LiveConfig, LiveError, LiveStats, SpecialTrades, TradeTick, VolumeUnit,
};
pub use local::{load_local_bars, load_local_bars_with, local_bars_dir, LOCAL_BARS_ENV};
pub use provider::{
    bind, entitlement_matches, validate_binding, validate_entitlement, validate_id,
    validate_interval, Bound, MemoryProvider, Resolution, SourceProvider,
};
pub use regimes::{
    regime_at, regime_bars, regime_schedule, synthetic_calendar, synthetic_market,
    synthetic_market_hourly, trading_day_opens, trading_hour_opens, Regime, RegimeSpan, RegimeSpec,
    SyntheticInstrument, MARKET_START, SYNTHETIC_HALF_DAYS, SYNTHETIC_HOLIDAYS,
};
pub use replay::{
    aggregate_days, date_start, local_replay_feeds, read_symbols, synthetic_replay_feeds, Intraday,
    IntradayCounts, Replay, ReplayCalendar, ReplayCursor, ReplayFeed, ReplayOptions, ReplayStep,
};
pub use source::{
    apply, collect_bars, collect_series, BarEvent, MemorySource, SeriesMeta, SeriesSource,
    SourceError,
};
pub use synthetic::{synthetic_bars, synthetic_series, SyntheticSource};
pub use window::{resolve_window, ResolvedWindow};

use file::read;
