//! (R5B, `docs/PROTOCOL.md` §25.6) The `preview` query: a pending proposal expanded on a
//! copy of the asker's document, as its acceptance would expand it now; the changes and a
//! scene summary of every view it adds or changes. Nothing is written.
//!
//! (`Session::scene_as_reader`, the host's compile of a view or a proposal's preview as a
//! reader reads, stays in the authority: it is not a query.)

use qp_grammar::compile;
use qp_schema::protocol::{ChangeOp, ErrorCode, PreviewView, ProposalState, QueryResult};
use qp_schema::Document;

use crate::rd::Rd;
use crate::reading::{HIT_HEIGHT, HIT_WIDTH};
use qp_ops::bounds::gated_view;
use qp_ops::expand::need_proposal;
use qp_ops::provider::{warm, DocProvider};
use qp_plane::error::{PResult, E};
use qp_plane::plane::reading_doc;
use qp_plane::values::Cache;

impl Rd<'_> {
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
        let (doc, changes) = self.v.accept_preview(pid, &asker)?;
        let rd = reading_doc(&doc, self.plane).unwrap_or_else(|| doc.clone());
        Ok((doc, rd, changes))
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
        let opts = qp_designs::options(
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
            let explain = match gated_view(self.plane, &doc, v, &asker, "/query/proposal") {
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
                    Ok(scene) => crate::explain::explain_scene(&gv, &scene),
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
}
