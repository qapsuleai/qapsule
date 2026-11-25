//! The composition (`docs/CONCEPTS.md` "Bileşim"): the standard subsystems, and the
//! authorities built with them. Each type here wraps its `qp-authority` type
//! (`Deref`/`DerefMut`: today's call sites unchanged) and builds it with [`STANDARD`]; a
//! follower (the replica, the viewer's open side) with [`FOLLOWER`] (RF1 S3.2).
//! New code reaches a subsystem's state with [`Session::sub`] / [`Session::sub_mut`],
//! not through `Deref`.

use std::ops::{Deref, DerefMut};
use std::path::Path;

use qp_schema::protocol::SessionInit;
use qp_subsystem::Subsystem;

/// The standard subsystems, in registration order: one line per subsystem.
static LIST: &[&dyn Subsystem] = &[
    &qp_watch::WATCH,   // izleme niyeti (RF1 S3.1)
    &qp_read::READ,     // okuma ve sunum (RF1 S3.2)
    &qp_viewer::VIEWER, // görüntüleyici bağlantıları (RF1 S3.3)
];

/// The standard subsystems every authority of the composition is built with.
pub const STANDARD: qp_authority::Subsystems = qp_authority::Subsystems::new(LIST);

/// The standard subsystems of a follower (a replica's authority, a viewer's open side):
/// [`STANDARD`] without the reading and the viewer subsystems. A follower takes no `query`,
/// `input` or `subscribe` (no connection talks to it; it follows a host's log and is read
/// through its own methods), so neither would ever be called there; leaving them out keeps
/// their code out of the shell's WASM package (RF1 S3.2: +1.38 MB otherwise; the WASM ±2 %
/// gate; RF1 S3.3 measured the viewer's share: `rounds/RF1/experiments/rewritten.md`).
pub const FOLLOWER: qp_authority::Subsystems = qp_authority::Subsystems::new(FOLLOWER_LIST);

/// [`FOLLOWER`]'s list.
static FOLLOWER_LIST: &[&dyn Subsystem] = &[&qp_watch::WATCH];

/// Implements `Deref`/`DerefMut` to the wrapped `qp-authority` type.
macro_rules! wraps {
    ($outer:ident, $inner:ty) => {
        impl Deref for $outer {
            type Target = $inner;
            fn deref(&self) -> &$inner {
                &self.0
            }
        }
        impl DerefMut for $outer {
            fn deref_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }
    };
}

/// The `qp/0` authority with the standard subsystems (the authority alone:
/// [`qp_authority::Session`]).
pub struct Session(qp_authority::Session);
wraps!(Session, qp_authority::Session);

impl Session {
    /// Starts an authority. A `DocumentRef::Path` is resolved under `repo_root` and may
    /// not leave it.
    ///
    /// # Errors
    /// The document cannot be read or parsed.
    pub fn new(init: SessionInit, repo_root: &Path) -> Result<Self, String> {
        qp_authority::Session::new(init, repo_root, STANDARD).map(Self)
    }

    /// (R3, §20) [`Session::new`] with the host's secret mühür keys.
    ///
    /// # Errors
    /// As [`Session::new`].
    pub fn new_with_trust_keys(
        init: SessionInit,
        repo_root: &Path,
        receipt_key: Vec<u8>,
        audit_key: Vec<u8>,
    ) -> Result<Self, String> {
        qp_authority::Session::new_with_trust_keys(
            init,
            repo_root,
            receipt_key,
            audit_key,
            STANDARD,
        )
        .map(Self)
    }

    /// Starts an authority whose bound series are resolved by the host's `provider`.
    ///
    /// # Errors
    /// As [`Session::new`]; `init.data` given as well.
    pub fn with_provider(
        init: SessionInit,
        repo_root: &Path,
        provider: Box<dyn qp_source::SourceProvider>,
    ) -> Result<Self, String> {
        qp_authority::Session::with_provider(init, repo_root, provider, STANDARD).map(Self)
    }

    /// (R3, §20) [`Session::with_provider`] with the host's secret mühür keys.
    ///
    /// # Errors
    /// As [`Session::with_provider`].
    pub fn with_provider_and_trust_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Box<dyn qp_source::SourceProvider>,
        receipt_key: Vec<u8>,
        audit_key: Vec<u8>,
    ) -> Result<Self, String> {
        qp_authority::Session::with_provider_and_trust_keys(
            init,
            repo_root,
            provider,
            receipt_key,
            audit_key,
            STANDARD,
        )
        .map(Self)
    }

    /// A subsystem's state of type `T` (its slot).
    #[must_use]
    pub fn sub<T: 'static>(&self) -> Option<&T> {
        self.0.slots().get::<T>()
    }

    /// A subsystem's state of type `T`, to change.
    pub fn sub_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.0.slots_mut().get_mut::<T>()
    }

    /// The authority itself.
    #[must_use]
    pub fn into_inner(self) -> qp_authority::Session {
        self.0
    }

    /// Compiles a view as connection `conn` sees it: `view_id` (default: the connection's
    /// current view) at the connection's visible range of that view, unless `opts.range`
    /// is set (the reading subsystem's `hit` range).
    ///
    /// # Errors
    /// Unknown connection or view; compile errors as in `Session::scene`.
    #[allow(clippy::result_large_err)] // the authority's protocol error, as its `scene`
    pub fn scene_for(
        &mut self,
        conn: &str,
        view_id: Option<&str>,
        opts: &qp_authority::CompileOptions,
    ) -> Result<qp_schema::scene::SceneDesc, qp_schema::protocol::ProtocolError> {
        let (vid, range) = self
            .0
            .with_subsystems(|ctx| qp_read::scene_for_range(ctx, conn, view_id))?;
        let mut o = opts.clone();
        if o.range.is_none() {
            o.range = range;
        }
        self.0.scene(&vid, &o)
    }

    /// [`qp_read::explain_scene`] of view `view_id` compiled with `opts`.
    ///
    /// # Errors
    /// As `Session::scene`.
    #[allow(clippy::result_large_err)] // the authority's protocol error, as its `scene`
    pub fn explain(
        &mut self,
        view_id: &str,
        opts: &qp_authority::CompileOptions,
    ) -> Result<String, qp_schema::protocol::ProtocolError> {
        let view = qp_ops::expand::need_view(self.0.document(), view_id, "/view")?.clone();
        let scene = self.0.scene(view_id, opts)?;
        Ok(qp_read::explain_scene(&view, &scene))
    }
}

/// The replay host with the standard subsystems.
#[derive(Debug)]
pub struct ReplayHost(qp_replayhost::ReplayHost);
wraps!(ReplayHost, qp_replayhost::ReplayHost);

impl ReplayHost {
    /// Starts an authority on the replay of `init.replay`; local data is read from
    /// [`qp_source::local_bars_dir`].
    ///
    /// # Errors
    /// No `init.replay`, an invalid replay, or the session's own errors.
    pub fn new(init: SessionInit, root: &Path) -> Result<Self, String> {
        qp_replayhost::ReplayHost::new(init, root, STANDARD).map(Self)
    }

    /// [`ReplayHost::new`] with local data read from `local_dir`.
    ///
    /// # Errors
    /// As [`ReplayHost::new`].
    pub fn with_local_dir(
        init: SessionInit,
        root: &Path,
        local_dir: &Path,
    ) -> Result<Self, String> {
        qp_replayhost::ReplayHost::with_local_dir(init, root, local_dir, STANDARD).map(Self)
    }

    /// (R4/T6) [`ReplayHost::with_local_dir`] with a history window.
    ///
    /// # Errors
    /// As [`ReplayHost::new`].
    pub fn with_window(
        init: SessionInit,
        root: &Path,
        local_dir: &Path,
        window: Option<usize>,
    ) -> Result<Self, String> {
        qp_replayhost::ReplayHost::with_window(init, root, local_dir, window, STANDARD).map(Self)
    }

    /// (R6/T7) A live host: `feeds` are the initial bars; the host gives the data events.
    ///
    /// # Errors
    /// No `init.replay`, no feeds, invalid bars, or the session's own errors.
    pub fn live(
        init: SessionInit,
        root: &Path,
        feeds: Vec<qp_source::ReplayFeed>,
    ) -> Result<Self, String> {
        qp_replayhost::ReplayHost::live(init, root, feeds, STANDARD).map(Self)
    }
}

/// A replica whose follower authority has the follower's subsystems ([`FOLLOWER`]).
#[derive(Debug)]
pub struct Replica(qp_replica::Replica);
wraps!(Replica, qp_replica::Replica);

impl Replica {
    /// A replica for `principal` whose data comes from the host's `provider`.
    #[must_use]
    pub fn new(principal: &str, provider: Box<dyn qp_source::SourceProvider>) -> Self {
        Self(qp_replica::Replica::new(principal, provider, FOLLOWER))
    }

    /// A replica whose provider is a [`qp_source::MemoryProvider`] that also takes the
    /// data events the host delivers.
    #[must_use]
    pub fn with_memory(principal: &str, provider: qp_source::MemoryProvider) -> Self {
        Self(qp_replica::Replica::with_memory(
            principal, provider, FOLLOWER,
        ))
    }
}

/// The device's side of the open capsule, its replica with the follower's subsystems
/// ([`FOLLOWER`]).
#[derive(Debug)]
pub struct OpenSide(qp_viewer::OpenSide);
wraps!(OpenSide, qp_viewer::OpenSide);

impl OpenSide {
    /// The open side of `open` at log position `seq`, its replica's own data from
    /// `provider`.
    #[must_use]
    pub fn new(
        principal: &str,
        open: qp_schema::protocol::ViewerOpen,
        seq: u64,
        provider: qp_source::MemoryProvider,
        calendar: Option<qp_schema::Calendar>,
    ) -> Self {
        Self(qp_viewer::OpenSide::new(
            principal, open, seq, provider, calendar, FOLLOWER,
        ))
    }
}

#[cfg(test)]
mod lists {
    use super::{FOLLOWER, STANDARD};
    use qp_subsystem::Subsystem as _;

    /// (RF1 S3.2, audit-s32 B2; S3.3) The follower's list is the standard list without the
    /// reading and the viewer subsystems, in the same order: a subsystem added to `LIST` is
    /// added to `FOLLOWER_LIST` too unless it serves a connection (queries, gestures, a
    /// pushed topic).
    #[test]
    fn the_follower_list_is_the_standard_list_without_reading_and_viewers() {
        let names = |l: qp_authority::Subsystems| -> Vec<&str> {
            l.list().iter().map(|s| s.name()).collect()
        };
        let mut want = names(STANDARD);
        want.retain(|n| *n != qp_read::READ.name() && *n != qp_viewer::VIEWER.name());
        assert_eq!(names(FOLLOWER), want);
        assert!(names(STANDARD).contains(&"read"));
        assert!(names(STANDARD).contains(&"viewer"));
    }
}
