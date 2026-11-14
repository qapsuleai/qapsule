//! JSON-side alternatives: emitted JSON Patch (from ChangeOp + current state), an
//! LCS/id-aware JSON Patch diff (the "best generic diff"), RFC 7396 merge-patch diff,
//! `test` guards, normalisation and a post-state domain validator.

use json_patch::jsonptr::PointerBuf;
use json_patch::{
    AddOperation, Patch, PatchOperation, RemoveOperation, ReplaceOperation, TestOperation,
};
use qp_schema::protocol::ChangeOp;
use qp_schema::{Document, SeriesSource};
use serde_json::{Map, Value};

use crate::model::{self, ExpandCfg};

fn ptr(s: &str) -> PointerBuf {
    PointerBuf::parse(s.to_owned()).expect("pointer")
}
fn add(p: &str, v: Value) -> PatchOperation {
    PatchOperation::Add(AddOperation {
        path: ptr(p),
        value: v,
    })
}
fn rm(p: &str) -> PatchOperation {
    PatchOperation::Remove(RemoveOperation { path: ptr(p) })
}
fn rep(p: &str, v: Value) -> PatchOperation {
    PatchOperation::Replace(ReplaceOperation {
        path: ptr(p),
        value: v,
    })
}

fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

// ---------------------------------------------------------------------------
// A2: LCS / id-aware diff (jsondiffpatch-style "objectHash" = `id`)
// ---------------------------------------------------------------------------

pub fn diff_lcs(l: &Value, r: &Value) -> Patch {
    let mut out = vec![];
    let mut path = String::new();
    go(l, r, &mut path, &mut out);
    Patch(out)
}

pub fn diff_lcs_at(prefix: &str, l: &Value, r: &Value, out: &mut Vec<PatchOperation>) {
    let mut path = prefix.to_owned();
    go(l, r, &mut path, out);
}

fn go(l: &Value, r: &Value, path: &mut String, out: &mut Vec<PatchOperation>) {
    match (l, r) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, bv) in b {
                let n = path.len();
                path.push('/');
                path.push_str(&esc(k));
                match a.get(k) {
                    Some(av) => go(av, bv, path, out),
                    None => out.push(add(path, bv.clone())),
                }
                path.truncate(n);
            }
            for k in a.keys() {
                if !b.contains_key(k) {
                    out.push(rm(&format!("{path}/{}", esc(k))));
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => diff_arr(a, b, path, out),
        _ if l == r => {}
        _ => out.push(rep(path, r.clone())),
    }
}

#[derive(PartialEq)]
enum Same {
    Equal,
    SameId,
    No,
}

fn same(a: &Value, b: &Value) -> Same {
    match (a.get("id"), b.get("id")) {
        (Some(x), Some(y)) if x == y => {
            if a == b {
                Same::Equal
            } else {
                Same::SameId
            }
        }
        (Some(_), Some(_)) => Same::No,
        _ => {
            if a == b {
                Same::Equal
            } else {
                Same::No
            }
        }
    }
}

fn at(path: &str, i: usize) -> String {
    format!("{path}/{i}")
}

fn diff_arr(a: &[Value], b: &[Value], path: &mut String, out: &mut Vec<PatchOperation>) {
    let mut pre = 0;
    while pre < a.len() && pre < b.len() {
        match same(&a[pre], &b[pre]) {
            Same::Equal => {}
            Same::SameId => {
                let mut p = at(path, pre);
                go(&a[pre], &b[pre], &mut p, out);
            }
            Same::No => break,
        }
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre
        && suf < b.len() - pre
        && same(&a[a.len() - 1 - suf], &b[b.len() - 1 - suf]) != Same::No
    {
        suf += 1;
    }
    let am = &a[pre..a.len() - suf];
    let bm = &b[pre..b.len() - suf];
    // Alignment of the middle: true = match.
    #[derive(Clone, Copy)]
    enum Step {
        M(usize, usize),
        D(usize),
        A(usize),
    }
    let mut steps = vec![];
    if am.len() * bm.len() <= 4_000_000 {
        let (n, m) = (am.len(), bm.len());
        let mut t = vec![0u32; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i * (m + 1) + j] = if same(&am[i], &bm[j]) != Same::No {
                    t[(i + 1) * (m + 1) + j + 1] + 1
                } else {
                    t[(i + 1) * (m + 1) + j].max(t[i * (m + 1) + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && same(&am[i], &bm[j]) != Same::No {
                steps.push(Step::M(i, j));
                i += 1;
                j += 1;
            } else if j < m && (i == n || t[i * (m + 1) + j + 1] >= t[(i + 1) * (m + 1) + j]) {
                steps.push(Step::A(j));
                j += 1;
            } else {
                steps.push(Step::D(i));
                i += 1;
            }
        }
    } else {
        // Too large: positional fallback.
        for i in 0..am.len().max(bm.len()) {
            match (i < am.len(), i < bm.len()) {
                (true, true) => {
                    steps.push(Step::D(i));
                    steps.push(Step::A(i));
                }
                (true, false) => steps.push(Step::D(i)),
                _ => steps.push(Step::A(i)),
            }
        }
    }
    let mut k = pre;
    let mut dels: Vec<usize> = vec![];
    let mut adds: Vec<usize> = vec![];
    let flush =
        |dels: &mut Vec<usize>, adds: &mut Vec<usize>, k: &mut usize, out: &mut Vec<PatchOperation>| {
            let pairs = dels.len().min(adds.len());
            for q in 0..pairs {
                let mut p = at(path, *k);
                go(&am[dels[q]], &bm[adds[q]], &mut p, out);
                *k += 1;
            }
            for _ in pairs..dels.len() {
                out.push(rm(&at(path, *k)));
            }
            for &j in &adds[pairs..] {
                out.push(add(&at(path, *k), bm[j].clone()));
                *k += 1;
            }
            dels.clear();
            adds.clear();
        };
    for s in steps {
        match s {
            Step::M(i, j) => {
                flush(&mut dels, &mut adds, &mut k, out);
                if am[i] != bm[j] {
                    let mut p = at(path, k);
                    go(&am[i], &bm[j], &mut p, out);
                }
                k += 1;
            }
            Step::D(i) => dels.push(i),
            Step::A(j) => adds.push(j),
        }
    }
    flush(&mut dels, &mut adds, &mut k, out);
    for q in 0..suf {
        let (x, y) = (&a[a.len() - suf + q], &b[b.len() - suf + q]);
        if x != y {
            let mut p = at(path, k);
            go(x, y, &mut p, out);
        }
        k += 1;
    }
}

// ---------------------------------------------------------------------------
// A3: JSON Patch emitted by the authority from what it did (no diffing)
// ---------------------------------------------------------------------------

fn coll_index(doc: &Document, id: &str) -> Option<(&'static str, usize)> {
    macro_rules! find {
        ($f:ident, $name:literal) => {
            if let Some(i) = doc.$f.iter().position(|x| x.id == id) {
                return Some(($name, i));
            }
        };
    }
    find!(series, "series");
    find!(views, "views");
    find!(facts, "facts");
    find!(findings, "findings");
    find!(capsules, "capsules");
    find!(intents, "intents");
    find!(narratives, "narratives");
    find!(proposals, "proposals");
    None
}

fn tv<T: serde::Serialize>(x: &T) -> Value {
    serde_json::to_value(x).expect("value")
}

fn put_item<T: serde::Serialize>(
    doc: &Document,
    coll: &str,
    id: &str,
    old: Option<Value>,
    new: &T,
    out: &mut Vec<PatchOperation>,
) {
    match (coll_index(doc, id), old) {
        (Some((_, i)), Some(o)) => diff_lcs_at(&format!("/{coll}/{i}"), &o, &tv(new), out),
        _ => out.push(add(&format!("/{coll}/-"), tv(new))),
    }
}

/// Translates one change into JSON Patch against the state *before* the change.
pub fn emit_one(doc: &Document, c: &ChangeOp, out: &mut Vec<PatchOperation>) {
    match c {
        ChangeOp::PutSeries { series } => {
            let old = model::series(doc, &series.id).map(tv);
            put_item(doc, "series", &series.id, old, series, out);
        }
        ChangeOp::PutView { view } => {
            let old = doc.views.iter().find(|v| v.id == view.id).map(tv);
            put_item(doc, "views", &view.id, old, view, out);
        }
        ChangeOp::PutFact { fact } => {
            let old = doc.facts.iter().find(|v| v.id == fact.id).map(tv);
            put_item(doc, "facts", &fact.id, old, fact, out);
        }
        ChangeOp::PutFinding { finding } => {
            let old = doc.findings.iter().find(|v| v.id == finding.id).map(tv);
            put_item(doc, "findings", &finding.id, old, finding, out);
        }
        ChangeOp::PutCapsule { capsule } => {
            let old = doc.capsules.iter().find(|v| v.id == capsule.id).map(tv);
            put_item(doc, "capsules", &capsule.id, old, capsule, out);
        }
        ChangeOp::PutIntent { intent } => {
            let old = doc.intents.iter().find(|v| v.id == intent.id).map(tv);
            put_item(doc, "intents", &intent.id, old, intent, out);
        }
        ChangeOp::PutNarrative { narrative } => {
            let old = doc.narratives.iter().find(|v| v.id == narrative.id).map(tv);
            put_item(doc, "narratives", &narrative.id, old, narrative, out);
        }
        ChangeOp::PutProposal { proposal } => {
            let old = doc.proposals.iter().find(|v| v.id == proposal.id).map(tv);
            put_item(doc, "proposals", &proposal.id, old, proposal, out);
        }
        ChangeOp::InsertLayer { view, index, layer } => {
            let (_, vi) = coll_index(doc, view).expect("view");
            out.push(add(&format!("/views/{vi}/layers/{index}"), tv(layer)));
        }
        ChangeOp::DeleteLayer { view, index } => {
            let (_, vi) = coll_index(doc, view).expect("view");
            out.push(rm(&format!("/views/{vi}/layers/{index}")));
        }
        ChangeOp::SetXDomain { view, domain } => {
            let (_, vi) = coll_index(doc, view).expect("view");
            let had = doc.views[vi].x.domain.is_some();
            let p = format!("/views/{vi}/x/domain");
            match (had, domain) {
                (true, Some(d)) => out.push(rep(&p, tv(d))),
                (false, Some(d)) => out.push(add(&p, tv(d))),
                (true, None) => out.push(rm(&p)),
                (false, None) => {}
            }
        }
        ChangeOp::SetCapsuleState { capsule, state } => {
            let (_, ci) = coll_index(doc, capsule).expect("cap");
            out.push(rep(&format!("/capsules/{ci}/state"), tv(state)));
        }
        ChangeOp::PushLedger { entry } => out.push(add("/ledger/entries/-", tv(entry))),
        ChangeOp::PopLedger => {
            out.push(rm(&format!("/ledger/entries/{}", doc.ledger.entries.len() - 1)));
        }
        ChangeOp::Delete { id } => {
            let (coll, i) = coll_index(doc, id).expect("item");
            out.push(rm(&format!("/{coll}/{i}")));
        }
        ChangeOp::AppendBars { series, bars } => {
            let (_, si) = coll_index(doc, series).expect("series");
            for b in bars {
                out.push(add(&format!("/series/{si}/data/bars/-"), tv(b)));
            }
        }
        ChangeOp::TruncateBars { series: sid, count } => {
            let (_, si) = coll_index(doc, sid).expect("series");
            let n = model::series(doc, sid).and_then(|s| s.bars()).expect("bars").len();
            for q in 0..*count as usize {
                out.push(rm(&format!("/series/{si}/data/bars/{}", n - 1 - q)));
            }
        }
        ChangeOp::SetLastBar { series: sid, bar } => {
            let (_, si) = coll_index(doc, sid).expect("series");
            let n = model::series(doc, sid).and_then(|s| s.bars()).expect("bars").len();
            out.push(rep(&format!("/series/{si}/data/bars/{}", n - 1), tv(bar)));
        }
    }
}

// ---------------------------------------------------------------------------
// A4: RFC 7396 merge patch diff
// ---------------------------------------------------------------------------

pub fn merge_diff(l: &Value, r: &Value) -> Value {
    merge_diff_opt(l, r).unwrap_or_else(|| Value::Object(Map::new()))
}

fn merge_diff_opt(l: &Value, r: &Value) -> Option<Value> {
    match (l, r) {
        (Value::Object(a), Value::Object(b)) => {
            let mut m = Map::new();
            for (k, bv) in b {
                match a.get(k) {
                    Some(av) => {
                        if let Some(d) = merge_diff_opt(av, bv) {
                            m.insert(k.clone(), d);
                        }
                    }
                    None => {
                        m.insert(k.clone(), bv.clone());
                    }
                }
            }
            for k in a.keys() {
                if !b.contains_key(k) {
                    m.insert(k.clone(), Value::Null);
                }
            }
            (!m.is_empty()).then_some(Value::Object(m))
        }
        _ if l == r => None,
        _ => Some(r.clone()),
    }
}

// ---------------------------------------------------------------------------
// Guards, inverses, normalisation
// ---------------------------------------------------------------------------

/// Adds an RFC 6902 `test` before every remove/replace, checking the value the writer
/// saw at that path (best practice for optimistic JSON Patch).
pub fn guard(before: &Value, p: &Patch) -> Patch {
    let mut v = before.clone();
    let mut out = vec![];
    for op in p.iter() {
        if let PatchOperation::Remove(_) | PatchOperation::Replace(_) = op {
            let path = op.path().to_owned();
            let cur = v.pointer(path.as_str()).cloned().unwrap_or(Value::Null);
            out.push(PatchOperation::Test(TestOperation { path, value: cur }));
        }
        json_patch::patch(&mut v, std::slice::from_ref(op)).expect("guard walk");
        out.push(op.clone());
    }
    Patch(out)
}

/// Stronger generic guard: besides the value at the path, test the identity of every
/// array element the path walks through (`.../i/id` if the element has an `id`, else
/// the whole element). This is the best a generic JSON Patch writer can do against
/// index shifts.
pub fn guard_strong(before: &Value, p: &Patch) -> Patch {
    let mut v = before.clone();
    let mut out = vec![];
    for op in p.iter() {
        let path = op.path().as_str().to_owned();
        let segs: Vec<&str> = path.split('/').skip(1).collect();
        let mut pre = String::new();
        for (i, sg) in segs.iter().enumerate() {
            pre.push('/');
            pre.push_str(sg);
            if i + 1 >= segs.len() {
                break;
            }
            if sg.parse::<usize>().is_ok() {
                if let Some(el) = v.pointer(&pre) {
                    let t = match el.get("id") {
                        Some(id) => (format!("{pre}/id"), id.clone()),
                        None => (pre.clone(), el.clone()),
                    };
                    out.push(PatchOperation::Test(TestOperation {
                        path: ptr(&t.0),
                        value: t.1,
                    }));
                }
            }
        }
        if let PatchOperation::Remove(_) | PatchOperation::Replace(_) = op {
            let cur = v.pointer(&path).cloned().unwrap_or(Value::Null);
            out.push(PatchOperation::Test(TestOperation {
                path: ptr(&path),
                value: cur,
            }));
        }
        json_patch::patch(&mut v, std::slice::from_ref(op)).expect("guard walk");
        out.push(op.clone());
    }
    Patch(out)
}

/// Collections sorted by id (position of items in id-keyed collections is not
/// meaningful); layers, bars and ledger entries keep their order.
pub fn normalize(v: &Value) -> Value {
    let mut v = v.clone();
    for c in [
        "series",
        "views",
        "facts",
        "findings",
        "capsules",
        "intents",
        "narratives",
        "proposals",
    ] {
        if let Some(Value::Array(a)) = v.get_mut(c) {
            a.sort_by(|x, y| {
                x.get("id")
                    .and_then(Value::as_str)
                    .cmp(&y.get("id").and_then(Value::as_str))
            });
        }
    }
    v
}

/// Post-state domain validation of a whole document (what an authority receiving
/// JSON Patch can do after applying it). Returns violations.
pub fn validate_doc(doc: &Document, cfg: &ExpandCfg) -> Vec<String> {
    let mut e = vec![];
    let ids = model::all_ids(doc);
    let n_items = doc.series.len()
        + doc.views.len()
        + doc.facts.len()
        + doc.findings.len()
        + doc.capsules.len()
        + doc.intents.len()
        + doc.narratives.len()
        + doc.proposals.len();
    if ids.len() != n_items {
        e.push("duplicate ids".into());
    }
    let sid = |s: &str| model::series(doc, s).is_some();
    for s in &doc.series {
        if let Some(b) = s.bars() {
            if let Err(x) = qp_schema::validate_bars(b) {
                e.push(format!("series {}: {x}", s.id));
            }
        }
        if let SeriesSource::Study {
            study,
            inputs,
            params,
            ..
        } = &s.source
        {
            match model::known_params(cfg, study) {
                None => e.push(format!("series {}: unknown study", s.id)),
                Some(k) => {
                    for (p, v) in params {
                        if !k.contains(&p.as_str()) {
                            e.push(format!("series {}: unknown param {p}", s.id));
                        }
                        if !(v.is_finite() && *v > 0.0) {
                            e.push(format!("series {}: param {p} must be > 0", s.id));
                        }
                    }
                }
            }
            for i in inputs {
                if model::series(doc, i).and_then(|x| x.bars()).is_none() {
                    e.push(format!("series {}: input {i} missing/not bars", s.id));
                }
            }
        }
    }
    for v in &doc.views {
        for (i, l) in v.layers.iter().enumerate() {
            if let Some(s) = &l.series {
                if !sid(s) {
                    e.push(format!("view {} layer {i}: series {s} missing", v.id));
                }
            }
        }
        if let Some([a, b]) = v.x.domain {
            if a > b {
                e.push(format!("view {}: x domain reversed", v.id));
            }
        }
    }
    let cond_ok = |c: &qp_schema::Condition| match c {
        qp_schema::Condition::Threshold { series, .. } => sid(series),
        qp_schema::Condition::Compare { left, right, .. } => sid(left) && sid(right),
        qp_schema::Condition::Text { .. } => true,
    };
    for f in &doc.facts {
        match model::backing_bars(doc, &f.series) {
            None => e.push(format!("fact {}: series missing", f.id)),
            Some(b) => {
                if b.get(f.bar as usize).map(|x| x.t) != Some(f.t) {
                    e.push(format!("fact {}: bar/t mismatch", f.id));
                }
            }
        }
    }
    for f in &doc.findings {
        for x in &f.facts {
            if !doc.facts.iter().any(|y| &y.id == x) {
                e.push(format!("finding {}: fact {x} missing", f.id));
            }
        }
        if !(0.0..=1.0).contains(&f.confidence) {
            e.push(format!("finding {}: confidence", f.id));
        }
        if !cond_ok(&f.invalidation) {
            e.push(format!("finding {}: condition series missing", f.id));
        }
    }
    for c in &doc.capsules {
        if !doc.findings.iter().any(|f| f.id == c.finding) {
            e.push(format!("capsule {}: finding missing", c.id));
        }
    }
    for i in &doc.intents {
        if i.trigger.as_ref().is_some_and(|t| !cond_ok(t)) {
            e.push(format!("intent {}: trigger series missing", i.id));
        }
    }
    for l in &doc.ledger.entries {
        if !doc.findings.iter().any(|f| f.id == l.finding) {
            e.push(format!("ledger: finding {} missing", l.finding));
        }
    }
    e
}

/// Path-based ACL for the `feed` role on JSON Patch: may touch only bar arrays.
pub fn feed_acl_ok(p: &Patch) -> bool {
    p.iter().all(|op| {
        let s = op.path().as_str();
        let parts: Vec<&str> = s.split('/').collect();
        parts.len() >= 5
            && parts[1] == "series"
            && parts[2].parse::<usize>().is_ok()
            && parts[3] == "data"
            && parts[4] == "bars"
    })
}
