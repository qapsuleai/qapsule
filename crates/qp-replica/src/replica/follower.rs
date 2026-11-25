//! (RL1/Ö2, A71) The follower: the replica's authority, closed behind one type.
//!
//! The authority's narrow hidden follower methods write outside the command path
//! (`build_follower`, `follow_start`, `take_provider`, `mirror_apply`, `mirror_commit`,
//! `follow_data`, `follow_resync`, `eval_parts`; `qp-authority`
//! `authority/session/follow.rs`). This file is their **only** caller outside
//! `qp-authority` (machine-checked: `scripts/arch-check`, rule `calls.restricted`); the
//! rest of the replica reaches them through [`Follower`]'s methods. A host holding the
//! session (`Replica::session_mut`) may not call them (the same rule).
//! Each method is one call, unchanged (RF1 tutanağıyla bayt eşit).

use std::ops::{Deref, DerefMut};

use qp_authority::{Session, Subsystems};
use qp_plane::plane::{DataError, DataPlane, Provider};
use qp_plane::values::Cache;
use qp_schema::data::DataEvent;
use qp_schema::protocol::{ChangeOp, DocumentRef, LogEntry, Principal, Role, SessionInit};
use qp_schema::{Author, Document};

/// A follower authority: a [`Session`] whose document only changes by mirroring the
/// authority's log, and whose data comes from its own provider. Read access (document,
/// scene, binding state) goes through [`Deref`] to the session.
pub(crate) struct Follower(Session);

impl Deref for Follower {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.0
    }
}

impl DerefMut for Follower {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.0
    }
}

impl Follower {
    /// A follower session over `doc` whose bound series are resolved by `provider` with
    /// `principal`'s entitlement, from the document's anchors (no stamping, no engine
    /// entry).
    pub(crate) fn start(
        doc: Document,
        principal: &str,
        provider: Provider,
        subs: Subsystems,
    ) -> Result<Self, String> {
        let init = SessionInit {
            document: DocumentRef::Inline(Box::new(doc.clone())),
            principals: vec![Principal {
                id: principal.to_owned(),
                author: Author::Human {
                    name: principal.to_owned(),
                },
                role: Role::Viewer,
                on_behalf: None,
            }],
            clock: 0,
            epoch: 0,
            proposal_ttl_ms: None,
            data: None,
            capsules: None,
            replay: None,
            feed_rule: None,
            agent_policy: None,
            comment_strictness: None,
        };
        // Built without a provider: every binding is pending, so nothing is stamped.
        let mut s = Session::build_follower(init, subs)?;
        s.follow_start(doc, provider, principal);
        Ok(Self(s))
    }

    /// Its data plane's provider, taken out (the follower is dropped: a reload).
    pub(crate) fn into_provider(mut self) -> Provider {
        self.0.take_provider()
    }

    /// `Session::mirror_apply`: applies a mirrored entry's changes, returns their inverse.
    pub(crate) fn apply(&mut self, changes: &[ChangeOp]) -> Result<Vec<ChangeOp>, String> {
        self.0.mirror_apply(changes)
    }

    /// `Session::mirror_commit`: commits a mirrored entry.
    pub(crate) fn commit(&mut self, e: LogEntry) {
        self.0.mirror_commit(e);
    }

    /// `Session::follow_data`: a data event of its own feed; the bound series it changed.
    pub(crate) fn data(&mut self, ev: &DataEvent) -> Result<Vec<String>, DataError> {
        self.0.follow_data(ev)
    }

    /// `Session::follow_resync`: resolves every bound series again from the provider.
    pub(crate) fn resync(&mut self) {
        self.0.follow_resync();
    }

    /// `Session::eval_parts`: document, reading document, data plane and values cache.
    pub(crate) fn parts(&mut self) -> (&Document, &Document, &DataPlane, &mut Cache) {
        self.0.eval_parts()
    }
}
