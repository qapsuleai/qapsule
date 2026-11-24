//! (RF1 S3.3) The narrow operations a follower performs on its authority (the client
//! replica, `qp-replica`: `rounds/RF1/experiments/paths.md` S3.3). A follower's authority
//! is built without a provider (nothing stamped), then takes the mirrored document and its
//! own data plane; its document only changes by mirroring the authority's `change` events
//! with the same application and bookkeeping as the authority's own entries. Each method is
//! one step the replica took on the authority's fields before the split, unchanged.

use qp_plane::plane::DataError;
use qp_plane::values::Delta;

use super::*;

impl Session {
    /// [`Session::build_with`] for a follower (no provider: every binding pending).
    ///
    /// # Errors
    /// As [`Session::build_with`].
    #[doc(hidden)]
    pub fn build_follower(init: SessionInit, subs: Subsystems) -> Result<Self, String> {
        Self::build_with(init, Path::new("."), Provider::None, Value::Null, subs)
    }

    /// Starts following `doc`: its document and initial document, an empty log, a data
    /// plane over `provider` with `principal`'s entitlement (resolved from the document's
    /// anchors), the reading document, and nothing cached or pending.
    #[doc(hidden)]
    pub fn follow_start(&mut self, doc: Document, provider: Provider, principal: &str) {
        self.doc = doc.clone();
        self.initial = doc;
        self.log.clear();
        let max = self.limits.max_bind_bars;
        self.plane = DataPlane::new(provider, Some(principal.to_owned()), max);
        let _ = self.plane.sync(&self.doc);
        self.rd = reading_doc(&self.doc, &self.plane);
        self.cache.clear();
        self.view_reads = None;
        self.pending_views.clear();
        self.pending_stamps.clear();
        let _ = self.claims.take();
    }

    /// The data plane's provider, taken out (a follower reloading its document).
    #[doc(hidden)]
    pub fn take_provider(&mut self) -> Provider {
        std::mem::replace(&mut self.plane.provider, Provider::None)
    }

    /// Applies a mirrored entry's `changes` to the document in place; returns their
    /// inverse (the document is left as it was when one does not fit).
    ///
    /// # Errors
    /// The first change that does not fit.
    #[doc(hidden)]
    pub fn mirror_apply(&mut self, changes: &[ChangeOp]) -> Result<Vec<ChangeOp>, String> {
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        crate::live::apply_in_place(&mut self.doc, changes, restore)
    }

    /// Commits a mirrored entry (the authority's own bookkeeping), with no pending view
    /// announcement and no claim queued.
    #[doc(hidden)]
    pub fn mirror_commit(&mut self, e: LogEntry) {
        self.committed(e);
        self.pending_views.clear();
        let _ = self.claims.take();
    }

    /// A data event of the follower's own feed: the bound series it changed (empty:
    /// nothing changed), the live path run over them.
    ///
    /// # Errors
    /// [`DataError`] as on the authority.
    #[doc(hidden)]
    pub fn follow_data(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<Vec<String>, DataError> {
        let outcome = self.plane.apply_event(ev)?;
        if outcome.changed.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<String> = outcome.changed.iter().map(|(id, _)| id.clone()).collect();
        self.data_changed(outcome.changed);
        self.pending_views.clear();
        Ok(ids)
    }

    /// Resolves every bound series again from the provider (no anchor stamped).
    #[doc(hidden)]
    pub fn follow_resync(&mut self) {
        let ids: Vec<String> = self.plane.series.keys().cloned().collect();
        self.plane.series.clear();
        let _ = self.plane.sync(&self.doc);
        self.pending_stamps.clear();
        self.data_changed(
            ids.into_iter()
                .map(|id| (id, Delta::Rebuild { from: 0 }))
                .collect(),
        );
        self.pending_views.clear();
    }

    /// The document, the reading document, the data plane and the values cache at once
    /// (the display rule evaluates a claim's condition on its own data).
    #[doc(hidden)]
    pub fn eval_parts(&mut self) -> (&Document, &Document, &DataPlane, &mut Cache) {
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        (&self.doc, rd, &self.plane, &mut self.cache)
    }
}
