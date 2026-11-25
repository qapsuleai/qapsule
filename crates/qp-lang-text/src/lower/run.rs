//! The lowering driver: names first, then the refusal or the statements in order.

use std::collections::BTreeMap;

use serde_json::Value;

use super::state::{Def, Lower};
use super::stmt;
use crate::ast::Located;
use crate::diag::Diag;
use crate::map::SourceMap;
use crate::parse::Parsed;

/// Lowers parsed statements; returns the IR (a program or a refusal), its source map
/// and the errors (parse errors included).
pub(crate) fn lower(parsed: Parsed) -> (Value, SourceMap, Vec<Diag>) {
    let mut lw = Lower {
        names: BTreeMap::new(),
        lines: BTreeMap::new(),
        params: Vec::new(),
        nodes: Vec::new(),
        outputs: Vec::new(),
        anon: 0,
        map: SourceMap::default(),
        diags: parsed.diags,
    };
    for (n, line) in &parsed.poisoned {
        lw.lines.entry(n.clone()).or_insert(*line);
    }
    for s in &parsed.stmts {
        if let Some((n, sp)) = stmt::defines(&s.stmt) {
            lw.lines.entry(n.to_owned()).or_insert(sp.line);
        }
    }
    if let Some(ir) = stmt::reject(&mut lw, &parsed.stmts) {
        return (ir, lw.map, lw.diags);
    }
    run(&mut lw, &parsed.stmts, &parsed.poisoned);
    let ir = lw.program();
    (ir, lw.map, lw.diags)
}

fn run(lw: &mut Lower, stmts: &[Located], poisoned: &[(String, u32)]) {
    let mut seen_body = false;
    let mut pi = 0;
    for s in stmts {
        // A failed statement's name becomes known where it stood.
        while let Some((n, _)) = poisoned.get(pi).filter(|(_, l)| *l <= s.span.line) {
            lw.names.entry(n.clone()).or_insert(Def::Poisoned);
            pi += 1;
        }
        stmt::statement(lw, s, &mut seen_body);
    }
    for (n, _) in &poisoned[pi..] {
        lw.names.entry(n.clone()).or_insert(Def::Poisoned);
    }
}
