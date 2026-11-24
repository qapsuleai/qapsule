//! The authority's own compile (RF1 S3.2): the document provider over its document and
//! values cache, a view's scene, and a view (or a proposal's preview of it) as a
//! connection's reader reads it ([`Session::scene_as_reader`], the MCP `render` tool).
//! Queries and input gestures are the reading subsystem's (`qp-read`), registered by the
//! composition; these stay here because the host and the replica compile with them
//! outside any query.

use qp_grammar::{compile, CompileOptions};
use qp_look::{ErrorKind, GrammarError};
use qp_schema::protocol::{ChangeOp, ErrorCode, ProposalState, ProtocolError, SurfaceKind};
use qp_schema::scene::SceneDesc;
use qp_schema::Document;

use crate::session::Session;
use qp_ops::bounds::gated_view;
use qp_ops::expand::{need_proposal, need_view};
use qp_ops::provider::{warm, DocProvider};
use qp_plane::error::{PResult, E};
use qp_plane::plane::reading_doc;
use qp_plane::values::Cache;

#[cfg(test)]
mod tests;

/// A compile error as a protocol error at `path`.
fn grammar_error(e: &GrammarError, path: &str) -> ProtocolError {
    let code = match e.kind {
        ErrorKind::NotFound => ErrorCode::NotFound,
        ErrorKind::Invalid => ErrorCode::Invalid,
        ErrorKind::NoOrigin => ErrorCode::NoOrigin,
    };
    E::new(code, e.to_string()).at(path).build()
}

impl Session {
    /// The grammar compiler's data source over this session's document and values cache
    /// (study values computed with `qp-compute` and cached first).
    pub fn provider(&mut self) -> DocProvider<'_> {
        warm(rd!(self), &mut self.cache);
        DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines)
    }

    /// Compiles view `view_id` into its scene description with `opts` (frame size,
    /// transient range and emphasis, theme). Headless; nothing is rendered.
    ///
    /// # Errors
    /// `not_found` for an unknown view; the compiler's error (layer, pointer, reason) as
    /// `invalid`, `not_found` or `no_origin`.
    pub fn scene(
        &mut self,
        view_id: &str,
        opts: &CompileOptions,
    ) -> Result<SceneDesc, ProtocolError> {
        warm(rd!(self), &mut self.cache);
        let view = need_view(rd!(self), view_id, "/view")?;
        compile(
            view,
            &DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines),
            opts,
        )
        .map_err(|e| grammar_error(&e, "/view"))
    }

    /// The surface kind of connection `ci` (for tests and benchmarks).
    #[doc(hidden)]
    pub fn surface_kind(&self, conn: &str) -> Option<SurfaceKind> {
        self.conns
            .iter()
            .find(|c| c.id == conn)
            .map(|c| c.surface.kind)
    }

    /// The document after accepting pending proposal `pid` now (as connection `ci`'s
    /// reader would), its reading document and the changes. Nothing is written.
    fn preview_doc(
        &self,
        ci: usize,
        pid: &str,
        path: &str,
    ) -> PResult<(Document, Document, Vec<ChangeOp>)> {
        let pr = need_proposal(&self.doc, pid, path)?;
        if pr.state != ProposalState::Pending {
            return Err(E::new(
                ErrorCode::NotPending,
                format!("proposal `{pid}` is not pending"),
            )
            .at(path)
            .build());
        }
        let asker = self.conns[ci].principal.clone();
        let acc = self.accept_expand(pid, None, &asker, None, self.seq() + 1)?;
        let rd = reading_doc(&acc.doc, &self.plane).unwrap_or_else(|| acc.doc.clone());
        Ok((acc.doc, rd, acc.changes))
    }

    /// (R5B, §25.6) Compiles view `view_id` as connection `conn`'s reader reads it (its
    /// projection and world, `on_behalf` for an agent); with `proposal`, on that pending
    /// proposal's preview. For the host (the MCP `render` tool); nothing is written.
    ///
    /// # Errors
    /// Unknown connection, view or proposal; a proposal that is not pending; compile
    /// errors.
    pub fn scene_as_reader(
        &mut self,
        conn: &str,
        view_id: &str,
        proposal: Option<&str>,
        opts: &CompileOptions,
    ) -> Result<SceneDesc, ProtocolError> {
        let ci = self
            .conns
            .iter()
            .position(|c| c.id == conn)
            .ok_or_else(|| {
                E::new(ErrorCode::NotFound, format!("unknown connection `{conn}`")).build()
            })?;
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        self.as_reader(&principal, lang, |s| match proposal {
            None => {
                let view = need_view(&s.doc, view_id, "/view")?.clone();
                let gv = gated_view(&s.plane, &s.doc, &view, &principal, "/view")?;
                warm(rd!(s), &mut s.cache);
                compile(
                    &gv,
                    &DocProvider::new(rd!(s), &s.cache).with_headlines(&s.headlines),
                    opts,
                )
                .map_err(|e| {
                    E::new(ErrorCode::Invalid, format!("view `{view_id}`: {e:?}"))
                        .at("/view")
                        .build()
                })
            }
            Some(pid) => {
                let (doc, rd, _) = s.preview_doc(ci, pid, "/proposal")?;
                let view = need_view(&rd, view_id, "/view")?;
                let view = gated_view(&s.plane, &doc, view, &principal, "/view")?;
                let mut cache = Cache::default();
                warm(&rd, &mut cache);
                compile(
                    &view,
                    &DocProvider::new(&rd, &cache).with_headlines(&s.headlines),
                    opts,
                )
                .map_err(|e| {
                    E::new(ErrorCode::Invalid, format!("view `{view_id}`: {e:?}"))
                        .at("/view")
                        .build()
                })
            }
        })
    }
}
