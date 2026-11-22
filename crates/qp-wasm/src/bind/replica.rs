//! The replica binding (RG1/M1a: moved out of `lib.rs` as a whole). The module stays
//! inline so that its text, formatted at this depth, is unchanged by the move.

pub use replica_binding::Replica;

mod replica_binding {
    use wasm_bindgen::prelude::*;

    /// A replica of an authority's document (RP2/T8, `docs/PROTOCOL.md` §19): a thin wrapper
    /// over `qp_session::replica::Replica`. Messages and data events go in as JSON text; the
    /// display state and the client messages to send come out as JSON text.
    #[wasm_bindgen]
    pub struct Replica {
        inner: qp_session::Replica,
    }

    #[wasm_bindgen]
    impl Replica {
        /// A replica for `principal` whose own data comes from a memory provider described by
        /// `provider_json` (an `InitData`: feeds with `bars` or `synthetic`, entitlements).
        ///
        /// # Errors
        /// Invalid provider description.
        #[wasm_bindgen(constructor)]
        pub fn new(principal: &str, provider_json: &str) -> Result<Replica, JsError> {
            let v: serde_json::Value =
                serde_json::from_str(provider_json).map_err(|e| JsError::new(&e.to_string()))?;
            let p = qp_session::tape::memory_provider(&v).map_err(|e| JsError::new(&e))?;
            Ok(Self {
                inner: qp_session::Replica::with_memory(principal, p),
            })
        }

        /// Shows provisional claims on the live bar (off by default).
        pub fn set_live(&mut self, on: bool) {
            let mut o = self.inner.options();
            o.live = on;
            self.inner.set_options(o);
        }

        /// Watches a condition (`Condition` JSON) as subscription `sub`.
        ///
        /// # Errors
        /// Invalid condition JSON.
        pub fn watch(&mut self, sub: &str, condition_json: &str) -> Result<(), JsError> {
            let c =
                serde_json::from_str(condition_json).map_err(|e| JsError::new(&e.to_string()))?;
            self.inner.watch(sub, c);
            Ok(())
        }

        /// One message of the authority (`{"conn":…,"msg":…}` or the bare message).
        ///
        /// # Errors
        /// Not a server message.
        pub fn on_message(&mut self, line: &str) -> Result<(), JsError> {
            self.inner.on_line(line).map_err(|e| JsError::new(&e))
        }

        /// One data event of its own feed (`DataEvent` JSON).
        ///
        /// # Errors
        /// Invalid JSON or an event the §18.4 rules reject.
        pub fn data_event(&mut self, event_json: &str) -> Result<(), JsError> {
            let ev = serde_json::from_str(event_json).map_err(|e| JsError::new(&e.to_string()))?;
            self.inner
                .data_event(&ev)
                .map_err(|e| JsError::new(&e.to_string()))
        }

        /// Re-resolves its bindings from its provider (after a gap or a settled fetch).
        pub fn resync(&mut self) {
            self.inner.resync();
        }

        /// The client messages to send (JSON array).
        #[must_use]
        pub fn take_outbox(&mut self) -> String {
            serde_json::to_string(&self.inner.take_outbox()).unwrap_or_default()
        }

        /// The display state (JSON array).
        #[must_use]
        pub fn display(&self) -> String {
            serde_json::to_string(self.inner.display()).unwrap_or_default()
        }

        /// The display digest.
        #[must_use]
        pub fn display_digest(&self) -> String {
            self.inner.display_digest()
        }

        /// The mirrored document's digest (empty before the document arrives).
        #[must_use]
        pub fn document_digest(&self) -> String {
            self.inner.document_digest().unwrap_or_default()
        }

        /// The digest of view `view`'s scene over its own data.
        ///
        /// # Errors
        /// No document yet, unknown view or a compile error.
        pub fn view_digest(
            &mut self,
            view: &str,
            width: u32,
            height: u32,
        ) -> Result<String, JsError> {
            self.inner
                .view_digest(view, width, height)
                .map_err(|e| JsError::new(&e))
        }

        /// The mirrored log position.
        #[must_use]
        pub fn seq(&self) -> u64 {
            self.inner.seq()
        }
    }
}
