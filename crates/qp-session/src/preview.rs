//! (R5B, `docs/PROTOCOL.md` §25.6) The `preview` query: a pending proposal expanded on a
//! copy of the asker's document, as its acceptance would expand it now; the changes and a
//! scene summary of every view it adds or changes. Nothing is written.

use qp_grammar::{compile, CompileOptions};
use qp_schema::protocol::{ChangeOp, ErrorCode, PreviewView, ProposalState, QueryResult};

use crate::error::{PResult, E};
use crate::expand::need_proposal;
use crate::plane::reading_doc;
use crate::provider::{warm, DocProvider};
use crate::reading::{HIT_HEIGHT, HIT_WIDTH};
use crate::session::Session;
use crate::values::Cache;

impl Session {
    pub(crate) fn preview(
        &mut self,
        ci: usize,
        pid: &str,
        (width, height): (Option<u32>, Option<u32>),
    ) -> PResult<QueryResult> {
        let pr = need_proposal(&self.doc, pid, "/query/proposal")?;
        if pr.state != ProposalState::Pending {
            return Err(E::new(
                ErrorCode::NotPending,
                format!("proposal `{pid}` is not pending"),
            )
            .at("/query/proposal")
            .build());
        }
        let asker = self.conns[ci].principal.clone();
        let acc = self.accept_expand(pid, None, &asker, None, self.seq() + 1)?;
        let series: Vec<String> = acc
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::PutSeries { series, .. } => Some(series.id.clone()),
                _ => None,
            })
            .collect();
        let rd = reading_doc(&acc.doc, &self.plane).unwrap_or_else(|| acc.doc.clone());
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
            let changed = acc
                .doc
                .views
                .iter()
                .find(|x| x.id == v.id)
                .is_some_and(|nv| old != Some(nv));
            if !(changed || reads_new) {
                continue;
            }
            let explain = match compile(
                v,
                &DocProvider::new(&rd, &cache).with_headlines(&self.headlines),
                &opts,
            ) {
                Ok(scene) => crate::explain_scene(v, &scene),
                Err(e) => format!("view {}: does not compile: {e:?}", v.id),
            };
            views.push(PreviewView {
                view: v.id.clone(),
                new: old.is_none(),
                explain,
            });
        }
        Ok(QueryResult::Preview {
            proposal: pid.to_owned(),
            changes: acc.changes,
            views,
        })
    }
}
