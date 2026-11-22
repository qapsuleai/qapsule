//! (R5B, `docs/PROTOCOL.md` §25.6) The `preview` query: a pending proposal expanded on a
//! copy of the asker's document, as its acceptance would expand it now; the changes and a
//! scene summary of every view it adds or changes. Nothing is written.
//!
//! [`Session::scene_as_reader`] (host side, the MCP `render` tool) compiles a view — or a
//! proposal's preview of it — as a connection's reader reads (its projection, its
//! world).

use qp_grammar::{compile, CompileOptions};
use qp_schema::protocol::{
    ChangeOp, ErrorCode, PreviewView, ProposalState, ProtocolError, QueryResult,
};
use qp_schema::scene::SceneDesc;
use qp_schema::{Document, View};

use crate::error::{PResult, E};
use crate::expand::{need_proposal, need_view};
use crate::plane::reading_doc;
use crate::provider::{warm, DocProvider};
use crate::reading::{HIT_HEIGHT, HIT_WIDTH};
use crate::session::Session;
use crate::values::Cache;

impl Session {
    /// (§18.3) View `view` of `doc` as `asker` may draw it: layers whose data it may not
    /// read left out; a view whose bar series it may not read refused (as `hit`).
    fn gated_view(&self, doc: &Document, view: &View, asker: &str, path: &str) -> PResult<View> {
        let mut blocked = std::collections::BTreeMap::new();
        for (i, l) in view.layers.iter().enumerate() {
            if self.plane.gate_layer(doc, Some(asker), l).is_some() {
                blocked.insert(i, ());
            }
        }
        if blocked.is_empty() {
            return Ok(view.clone());
        }
        if let Some(bs) = crate::expand::view_bar_series_any(doc, view) {
            if let Some(b) = self.plane.gate(doc, Some(asker), &bs.id) {
                return Err(b.error(&bs.id, path));
            }
        }
        Ok(crate::reading::without_layers(view, &blocked))
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

    pub(crate) fn preview(
        &mut self,
        ci: usize,
        pid: &str,
        (width, height): (Option<u32>, Option<u32>),
    ) -> PResult<QueryResult> {
        let (doc, rd, changes) = self.preview_doc(ci, pid, "/query/proposal")?;
        let series: Vec<String> = changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::PutSeries { series, .. } => Some(series.id.clone()),
                _ => None,
            })
            .collect();
        let mut cache = Cache::default();
        warm(&rd, &mut cache);
        let opts = CompileOptions::new(
            width.unwrap_or(HIT_WIDTH).clamp(16, 8192),
            height.unwrap_or(HIT_HEIGHT).clamp(16, 8192),
        );
        let mut views = Vec::new();
        for v in &rd.views {
            let old = self.doc.views.iter().find(|x| x.id == v.id);
            let reads_new = {
                let j = serde_json::to_string(v).unwrap_or_default();
                series.iter().any(|s| j.contains(&format!("\"{s}\"")))
            };
            let changed = doc
                .views
                .iter()
                .find(|x| x.id == v.id)
                .is_some_and(|nv| old != Some(nv));
            if !(changed || reads_new) {
                continue;
            }
            let asker = self.conns[ci].principal.clone();
            // (§18.3) Only what the asker may read is drawn and described.
            let explain = match self.gated_view(&doc, v, &asker, "/query/proposal") {
                Err(e) => format!(
                    "view {}: not readable by you ({})",
                    v.id,
                    serde_json::to_value(e.code)
                        .ok()
                        .and_then(|c| c.as_str().map(str::to_owned))
                        .unwrap_or_default()
                ),
                Ok(gv) => match compile(
                    &gv,
                    &DocProvider::new(&rd, &cache).with_headlines(&self.headlines),
                    &opts,
                ) {
                    Ok(scene) => crate::explain_scene(&gv, &scene),
                    Err(e) => format!("view {}: does not compile: {e:?}", v.id),
                },
            };
            views.push(PreviewView {
                view: v.id.clone(),
                new: old.is_none(),
                explain,
            });
        }
        Ok(QueryResult::Preview {
            proposal: pid.to_owned(),
            changes,
            views,
        })
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
                let gv = s.gated_view(&s.doc, &view, &principal, "/view")?;
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
                let view = s.gated_view(&doc, view, &principal, "/view")?;
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
