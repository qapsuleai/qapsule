//! The authority's own compile (RF1 S3.2): the document provider over its document and
//! values cache, and a view's scene. Queries and input gestures are the reading
//! subsystem's (`qp-read`), registered by the composition; these two stay here because the
//! host and the replica compile with them outside any query.

use qp_grammar::{compile, CompileOptions};
use qp_look::{ErrorKind, GrammarError};
use qp_schema::protocol::{ErrorCode, ProtocolError, SurfaceKind};
use qp_schema::scene::SceneDesc;

use crate::session::Session;
use qp_ops::expand::need_view;
use qp_ops::provider::{warm, DocProvider};
use qp_plane::error::E;

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
}
