//! Inlining plan: which `_` nodes are written where they are used. A node is inlined when
//! used exactly once, by something after it; a chain is cut (the inner node gets its own
//! line) where the rendered statement would exceed what the parser accepts, measured
//! with the parser's own depths ([`R`]). So `print(p)` always lowers again, and text
//! within the limit is never cut (canonical text is never deeper than its source).

use std::collections::BTreeMap;

use qp_schema::program::Program;

use super::expr::{base, output_refs, Printer};
use super::text::names;

impl<'a> Printer<'a> {
    /// A printer whose statements fit `max_depth`.
    pub(super) fn new(p: &'a Program, max_depth: usize) -> Printer<'a> {
        let mut index = BTreeMap::new();
        for (i, n) in p.nodes.iter().enumerate() {
            index.entry(n.id.as_str()).or_insert(i);
        }
        let n = p.nodes.len();
        let mut uses = vec![0usize; n];
        // The user of a node: a later node, or `n` for an output.
        let mut user = vec![0usize; n];
        let node_refs = p
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(j, nd)| nd.op.refs().into_iter().map(move |r| (j, r)));
        let out_refs = p.outputs.iter().flat_map(output_refs).map(|r| (n, r));
        for (j, r) in node_refs.chain(out_refs) {
            if let Some(&i) = index.get(base(r).0) {
                uses[i] += 1;
                user[i] = j;
            }
        }
        let inline = (0..n)
            .map(|i| p.nodes[i].id.starts_with('_') && uses[i] == 1 && user[i] > i)
            .collect();
        // Names do not change depths: placeholders while planning.
        let mut pr = Printer {
            p,
            index,
            inline,
            names: vec!["v".to_owned(); n],
        };
        pr.cut(max_depth);
        pr.names = names(p, &pr.inline);
        pr
    }

    /// Cuts inlined chains so that every node line and output line fits `max`. Children
    /// come before parents, so each node is checked with its children already settled.
    fn cut(&mut self, max: usize) {
        for i in 0..self.p.nodes.len() {
            let kids: Vec<usize> = self.p.nodes[i]
                .op
                .refs()
                .into_iter()
                .filter_map(|r| self.index.get(base(r).0).copied())
                .filter(|&k| k < i && self.inline[k])
                .collect();
            if !kids.is_empty() && !self.render(&self.p.nodes[i].op).fits(max) {
                for k in kids {
                    self.inline[k] = false;
                }
            }
        }
        let refs: Vec<&str> = self.p.outputs.iter().flat_map(output_refs).collect();
        for r in refs {
            if let Some(&i) = self.index.get(base(r).0) {
                if self.inline[i] && !self.reference(r).fits(max) {
                    self.inline[i] = false;
                }
            }
        }
    }
}
