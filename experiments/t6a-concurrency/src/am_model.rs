//! Model B: Automerge 0.12. One document per replica; actor id = principal. Items are
//! maps keyed by id, layers and ledger are lists of maps. Changes are broadcast as
//! native change bytes; a returning replica uses the Automerge sync protocol. Undo is
//! application-level (Automerge has no undo): it restores a value only if the current
//! winning op is still the one the undoer wrote; otherwise it refuses.

use std::collections::BTreeMap;

use automerge::sync::{self, SyncDoc};
use automerge::transaction::Transactable;
use automerge::ObjId as ExId;
use automerge::{ActorId, AutoCommit, ObjType, ReadDoc, ScalarValue, Value, ROOT};

use crate::common::{
    Intent, SCapsule, SFinding, SLayer, SLedger, SSeries, SemOp, Snap, Who, ALL, FACT,
};
use crate::sim::{Exp, Model, Reconnect, UndoStats, Wire};

#[derive(Clone)]
pub struct Roots {
    series: ExId,
    layers: ExId,
    facts: ExId,
    findings: ExId,
    capsules: ExId,
    intents: ExId,
    ledger: ExId,
}

enum AmUndo {
    /// Register writes: (object, key, old value, op id of the new value).
    Fields(Vec<(ExId, String, Option<ScalarValue>, ExId)>),
    /// A list element (map object) this replica inserted.
    ListElem(ExId),
    /// A layer this replica removed: re-insert after `anchor`.
    Reinsert(Option<ExId>, SLayer),
    /// A map key this replica created (+ the layer it added, if any).
    MapKey(ExId, String, ExId, Option<ExId>),
    /// A decision: capsule state field + the ledger element.
    Decide((ExId, String, Option<ScalarValue>, ExId), ExId),
    Unsupported,
}

pub struct AmModel {
    docs: Vec<AutoCommit>,
    roots: Roots,
    outbox: [Vec<Vec<u8>>; 3],
    undo_stack: Vec<AmUndo>,
    wire: Wire,
    undo: UndoStats,
    extra: BTreeMap<String, usize>,
}

fn actor(who: Who) -> ActorId {
    ActorId::from(vec![who.idx() as u8 + 1])
}

pub fn who_of_actor(a: &ActorId) -> &'static str {
    match a.to_bytes() {
        [1] => "arkin",
        [2] => "scout",
        [3] => "editor",
        _ => "init",
    }
}

fn s(v: &str) -> ScalarValue {
    ScalarValue::Str(v.into())
}

fn obj(d: &AutoCommit, o: &ExId, key: &str) -> Option<ExId> {
    match d.get(o, key).ok()? {
        Some((Value::Object(_), id)) => Some(id),
        _ => None,
    }
}

fn obj_at(d: &AutoCommit, o: &ExId, i: usize) -> Option<ExId> {
    match d.get(o, i).ok()? {
        Some((Value::Object(_), id)) => Some(id),
        _ => None,
    }
}

fn scalar(d: &AutoCommit, o: &ExId, key: &str) -> Option<ScalarValue> {
    match d.get(o, key).ok()? {
        Some((Value::Scalar(v), _)) => Some(v.into_owned()),
        _ => None,
    }
}

fn get_s(d: &AutoCommit, o: &ExId, key: &str) -> String {
    match scalar(d, o, key) {
        Some(ScalarValue::Str(x)) => x.to_string(),
        _ => String::new(),
    }
}

fn get_f(d: &AutoCommit, o: &ExId, key: &str) -> f64 {
    match scalar(d, o, key) {
        Some(ScalarValue::F64(x)) => x,
        Some(ScalarValue::Int(x)) => x as f64,
        Some(ScalarValue::Uint(x)) => x as f64,
        _ => f64::NAN,
    }
}

fn op_id(d: &AutoCommit, o: &ExId, key: &str) -> ExId {
    d.get(o, key).expect("get").expect("present").1
}

fn put_layer(d: &mut AutoCommit, layers: &ExId, i: usize, l: &SLayer) -> ExId {
    let lo = d.insert_object(layers, i, ObjType::Map).expect("ins");
    d.put(&lo, "mark", s(&l.mark)).expect("put");
    if let Some(ser) = &l.series {
        d.put(&lo, "series", s(ser)).expect("put");
    }
    d.put(&lo, "panel", ScalarValue::Int(i64::from(l.panel))).expect("put");
    lo
}

fn find_elem(d: &AutoCommit, list: &ExId, elem: &ExId) -> Option<usize> {
    (0..d.length(list)).find(|i| obj_at(d, list, *i).as_ref() == Some(elem))
}

pub fn snap_of(d: &AutoCommit, r: &Roots) -> Snap {
    let mut sn = Snap::default();
    for id in d.keys(&r.series) {
        let Some(o) = obj(d, &r.series, &id) else {
            continue;
        };
        let mut params = BTreeMap::new();
        if let Some(p) = obj(d, &o, "params") {
            for k in d.keys(&p) {
                params.insert(k.clone(), get_f(d, &p, &k));
            }
        }
        let inputs = get_s(d, &o, "inputs");
        sn.series.insert(
            id,
            SSeries {
                kind: get_s(d, &o, "kind"),
                study: get_s(d, &o, "study"),
                inputs: if inputs.is_empty() {
                    vec![]
                } else {
                    inputs.split(',').map(String::from).collect()
                },
                params,
            },
        );
    }
    for i in 0..d.length(&r.layers) {
        let Some(o) = obj_at(d, &r.layers, i) else {
            continue;
        };
        let ser = get_s(d, &o, "series");
        sn.layers.push(SLayer {
            mark: get_s(d, &o, "mark"),
            series: if ser.is_empty() { None } else { Some(ser) },
            panel: get_f(d, &o, "panel") as u32,
        });
    }
    for id in d.keys(&r.facts) {
        if let Some(o) = obj(d, &r.facts, &id) {
            sn.facts.insert(id, get_s(d, &o, "series"));
        }
    }
    for id in d.keys(&r.findings) {
        if let Some(o) = obj(d, &r.findings, &id) {
            sn.findings.insert(
                id,
                SFinding {
                    claim: get_s(d, &o, "claim"),
                    confidence: get_f(d, &o, "confidence"),
                    inval_series: get_s(d, &o, "inval_series"),
                    facts: vec![get_s(d, &o, "facts")],
                },
            );
        }
    }
    for id in d.keys(&r.capsules) {
        if let Some(o) = obj(d, &r.capsules, &id) {
            sn.capsules.insert(
                id,
                SCapsule {
                    finding: get_s(d, &o, "finding"),
                    state: get_s(d, &o, "state"),
                },
            );
        }
    }
    for id in d.keys(&r.intents) {
        if let Some(o) = obj(d, &r.intents, &id) {
            let t = get_s(d, &o, "trigger_series");
            sn.intents.insert(id, if t.is_empty() { None } else { Some(t) });
        }
    }
    for i in 0..d.length(&r.ledger) {
        if let Some(o) = obj_at(d, &r.ledger, i) {
            sn.ledger.push(SLedger {
                finding: get_s(d, &o, "finding"),
                state: get_s(d, &o, "state"),
                decided_claim: get_s(d, &o, "decided_claim"),
            });
        }
    }
    sn
}

fn build_initial(doc: &qp_schema::Document) -> AutoCommit {
    let mut d = AutoCommit::new().with_actor(ActorId::from(vec![0u8]));
    let series = d.put_object(&ROOT, "series", ObjType::Map).expect("o");
    let layers = d.put_object(&ROOT, "layers", ObjType::List).expect("o");
    let facts = d.put_object(&ROOT, "facts", ObjType::Map).expect("o");
    let findings = d.put_object(&ROOT, "findings", ObjType::Map).expect("o");
    let capsules = d.put_object(&ROOT, "capsules", ObjType::Map).expect("o");
    let _intents = d.put_object(&ROOT, "intents", ObjType::Map).expect("o");
    let _ledger = d.put_object(&ROOT, "ledger", ObjType::List).expect("o");
    d.put(&ROOT, "title", s(doc.title.as_deref().unwrap_or(""))).expect("p");
    for x in &doc.series {
        let o = d.put_object(&series, x.id.as_str(), ObjType::Map).expect("o");
        match &x.source {
            qp_schema::SeriesSource::Study {
                study,
                inputs,
                params,
                ..
            } => {
                d.put(&o, "kind", s("study")).expect("p");
                d.put(&o, "study", s(study)).expect("p");
                d.put(&o, "inputs", s(&inputs.join(","))).expect("p");
                let p = d.put_object(&o, "params", ObjType::Map).expect("o");
                for (k, v) in params {
                    d.put(&p, k.as_str(), ScalarValue::F64(*v)).expect("p");
                }
            }
            _ => {
                d.put(&o, "kind", s("inline")).expect("p");
                // Bars are not edited collaboratively: one opaque scalar.
                let bars = serde_json::to_vec(&x.data).expect("bars");
                d.put(&o, "bars", ScalarValue::Bytes(bars)).expect("p");
            }
        }
    }
    let main = doc.views.iter().find(|v| v.id == "main").expect("main");
    for (i, l) in main.layers.iter().enumerate() {
        let mark = serde_json::to_value(l.mark).expect("m");
        put_layer(
            &mut d,
            &layers,
            i,
            &SLayer {
                mark: mark.as_str().unwrap_or("").into(),
                series: l.series.clone(),
                panel: l.panel,
            },
        );
    }
    for f in &doc.facts {
        let o = d.put_object(&facts, f.id.as_str(), ObjType::Map).expect("o");
        d.put(&o, "series", s(&f.series)).expect("p");
        d.put(&o, "text", s(&f.text)).expect("p");
    }
    for f in &doc.findings {
        let o = d.put_object(&findings, f.id.as_str(), ObjType::Map).expect("o");
        d.put(&o, "claim", s(&f.claim)).expect("p");
        d.put(&o, "confidence", ScalarValue::F64(f.confidence)).expect("p");
        if let qp_schema::Condition::Threshold { series, .. } = &f.invalidation {
            d.put(&o, "inval_series", s(series)).expect("p");
        }
        d.put(&o, "facts", s(FACT)).expect("p");
        d.put(&o, "author", s("scout")).expect("p");
    }
    for c in &doc.capsules {
        let o = d.put_object(&capsules, c.id.as_str(), ObjType::Map).expect("o");
        d.put(&o, "finding", s(&c.finding)).expect("p");
        d.put(&o, "state", s("live")).expect("p");
        d.put(&o, "headline", s(&c.headline)).expect("p");
    }
    d.commit();
    d
}

impl AmModel {
    fn sync_pair(&mut self, a: usize, b: usize) -> (usize, usize, usize) {
        let (mut sa, mut sb) = (sync::State::new(), sync::State::new());
        let (mut ab, mut ba, mut rt) = (0, 0, 0);
        loop {
            let mut progressed = false;
            let gen = self.docs[a].sync().generate_sync_message(&mut sa);
            if let Some(m) = gen {
                let enc = m.encode();
                ab += enc.len();
                let m = sync::Message::decode(&enc).expect("decode");
                self.docs[b].sync().receive_sync_message(&mut sb, m).expect("recv");
                progressed = true;
            }
            let gen = self.docs[b].sync().generate_sync_message(&mut sb);
            if let Some(m) = gen {
                let enc = m.encode();
                ba += enc.len();
                let m = sync::Message::decode(&enc).expect("decode");
                self.docs[a].sync().receive_sync_message(&mut sa, m).expect("recv");
                progressed = true;
            }
            if !progressed {
                break;
            }
            rt += 1;
        }
        (ab, ba, rt)
    }

    fn do_undo(&mut self, d: usize) -> &'static str {
        let Some(u) = self.undo_stack.pop() else {
            return "nothing";
        };
        let r = self.roots.clone();
        let doc = &mut self.docs[d];
        let field_ok = |doc: &AutoCommit, (o, k, _, id): &(ExId, String, Option<ScalarValue>, ExId)| {
            let all = doc.get_all(o, k.as_str()).unwrap_or_default();
            all.len() == 1 && &all[0].1 == id
        };
        let restore = |doc: &mut AutoCommit, (o, k, old, _): &(ExId, String, Option<ScalarValue>, ExId)| {
            match old {
                Some(v) => doc.put(o, k.as_str(), v.clone()).expect("put"),
                None => doc.delete(o, k.as_str()).expect("del"),
            }
        };
        match u {
            AmUndo::Fields(fs) => {
                if !fs.iter().all(|f| field_ok(doc, f)) {
                    return "conflict";
                }
                for f in &fs {
                    restore(doc, f);
                }
            }
            AmUndo::ListElem(e) => {
                let Some(i) = find_elem(doc, &r.layers, &e) else {
                    return "conflict";
                };
                doc.delete(&r.layers, i).expect("del");
            }
            AmUndo::Reinsert(anchor, l) => {
                let i = match anchor {
                    None => 0,
                    Some(a) => match find_elem(doc, &r.layers, &a) {
                        Some(i) => i + 1,
                        None => return "conflict",
                    },
                };
                put_layer(doc, &r.layers, i, &l);
            }
            AmUndo::MapKey(map, key, o, layer) => {
                let all = doc.get_all(&map, key.as_str()).unwrap_or_default();
                if all.len() != 1 || all[0].1 != o {
                    return "conflict";
                }
                if let Some(le) = &layer {
                    match find_elem(doc, &r.layers, le) {
                        Some(i) => doc.delete(&r.layers, i).expect("del"),
                        None => return "conflict",
                    }
                }
                doc.delete(&map, key.as_str()).expect("del");
            }
            AmUndo::Decide(f, le) => {
                if !field_ok(doc, &f) {
                    return "conflict";
                }
                let Some(i) = find_elem(doc, &r.ledger, &le) else {
                    return "conflict";
                };
                restore(doc, &f);
                doc.delete(&r.ledger, i).expect("del");
            }
            AmUndo::Unsupported => return "unsupported",
        }
        "applied"
    }

    /// Apply a semantic op to replica `d` (already validated against its view).
    fn apply_op(&mut self, who: Who, op: &SemOp, view: &Snap) {
        let di = who.idx();
        let r = self.roots.clone();
        let doc = &mut self.docs[di];
        let mut undo = AmUndo::Unsupported;
        match op {
            SemOp::AddStudy {
                id,
                study,
                params,
                show_panel,
            } => {
                let o = doc.put_object(&r.series, id.as_str(), ObjType::Map).expect("o");
                doc.put(&o, "kind", s("study")).expect("p");
                doc.put(&o, "study", s(study)).expect("p");
                doc.put(&o, "inputs", s("px")).expect("p");
                doc.put(&o, "author", s(who.name())).expect("p");
                let p = doc.put_object(&o, "params", ObjType::Map).expect("o");
                for (k, v) in params {
                    doc.put(&p, k.as_str(), ScalarValue::F64(*v)).expect("p");
                }
                let layer = show_panel.map(|panel| {
                    let n = doc.length(&r.layers);
                    put_layer(
                        doc,
                        &r.layers,
                        n,
                        &SLayer {
                            mark: "line".into(),
                            series: Some(id.clone()),
                            panel,
                        },
                    )
                });
                undo = AmUndo::MapKey(r.series.clone(), id.clone(), o, layer);
            }
            SemOp::SetParams { series, params } => {
                let o = obj(doc, &r.series, series).expect("series");
                let p = obj(doc, &o, "params").expect("params");
                let mut fs = Vec::new();
                for (k, v) in params {
                    let old = scalar(doc, &p, k);
                    doc.put(&p, k.as_str(), ScalarValue::F64(*v)).expect("p");
                    fs.push((p.clone(), k.clone(), old, op_id(doc, &p, k)));
                }
                undo = AmUndo::Fields(fs);
            }
            SemOp::AddLayer {
                series,
                panel,
                index,
                ..
            } => {
                let e = put_layer(
                    doc,
                    &r.layers,
                    *index as usize,
                    &SLayer {
                        mark: "line".into(),
                        series: Some(series.clone()),
                        panel: *panel,
                    },
                );
                undo = AmUndo::ListElem(e);
            }
            SemOp::RemoveLayer { index, .. } => {
                let i = *index as usize;
                let anchor = if i == 0 {
                    None
                } else {
                    obj_at(doc, &r.layers, i - 1)
                };
                let l = view.layers[i].clone();
                doc.delete(&r.layers, i).expect("del");
                undo = AmUndo::Reinsert(anchor, l);
            }
            SemOp::RemoveSeries { id } => {
                doc.delete(&r.series, id.as_str()).expect("del");
                for (i, l) in view.layers.iter().enumerate().rev() {
                    if l.series.as_ref() == Some(id) {
                        doc.delete(&r.layers, i).expect("del");
                    }
                }
            }
            SemOp::AddFinding {
                id,
                claim,
                confidence,
                inval_series,
            } => {
                let o = doc.put_object(&r.findings, id.as_str(), ObjType::Map).expect("o");
                doc.put(&o, "claim", s(claim)).expect("p");
                doc.put(&o, "confidence", ScalarValue::F64(*confidence)).expect("p");
                doc.put(&o, "inval_series", s(inval_series)).expect("p");
                doc.put(&o, "facts", s(FACT)).expect("p");
                doc.put(&o, "author", s(who.name())).expect("p");
                let cid = format!("cap-{id}");
                let c = doc.put_object(&r.capsules, cid.as_str(), ObjType::Map).expect("o");
                doc.put(&c, "finding", s(id)).expect("p");
                doc.put(&c, "state", s("live")).expect("p");
                doc.put(&c, "headline", s(&format!("{claim} (kapsül)"))).expect("p");
                doc.put(&c, "author", s(who.name())).expect("p");
            }
            SemOp::UpdateFinding {
                finding,
                claim,
                confidence,
            } => {
                let o = obj(doc, &r.findings, finding).expect("finding");
                let mut fs = Vec::new();
                if let Some(c) = claim {
                    let old = scalar(doc, &o, "claim");
                    doc.put(&o, "claim", s(c)).expect("p");
                    fs.push((o.clone(), "claim".into(), old, op_id(doc, &o, "claim")));
                }
                if let Some(c) = confidence {
                    let old = scalar(doc, &o, "confidence");
                    doc.put(&o, "confidence", ScalarValue::F64(*c)).expect("p");
                    fs.push((o.clone(), "confidence".into(), old, op_id(doc, &o, "confidence")));
                }
                undo = AmUndo::Fields(fs);
            }
            SemOp::Decide {
                finding,
                approve,
                note,
            } => {
                let (cid, _) = view.capsule_of(finding).expect("capsule");
                let c = obj(doc, &r.capsules, cid).expect("cap");
                let state = if *approve { "approved" } else { "rejected" };
                let old = scalar(doc, &c, "state");
                doc.put(&c, "state", s(state)).expect("p");
                let f = (c.clone(), "state".to_string(), old, op_id(doc, &c, "state"));
                let n = doc.length(&r.ledger);
                let le = doc.insert_object(&r.ledger, n, ObjType::Map).expect("o");
                doc.put(&le, "finding", s(finding)).expect("p");
                doc.put(&le, "state", s(state)).expect("p");
                doc.put(&le, "note", s(note)).expect("p");
                doc.put(&le, "decided_claim", s(&view.findings[finding].claim)).expect("p");
                doc.put(&le, "recorded_by", s(who.name())).expect("p");
                undo = AmUndo::Decide(f, le);
            }
            SemOp::AddIntent {
                id,
                text,
                trigger_series,
            } => {
                let o = doc.put_object(&r.intents, id.as_str(), ObjType::Map).expect("o");
                doc.put(&o, "text", s(text)).expect("p");
                doc.put(&o, "trigger_series", s(trigger_series)).expect("p");
                doc.put(&o, "author", s(who.name())).expect("p");
                undo = AmUndo::MapKey(r.intents.clone(), id.clone(), o, None);
            }
            SemOp::RemoveFinding { id } => {
                doc.delete(&r.findings, id.as_str()).expect("del");
                for (cid, c) in &view.capsules {
                    if &c.finding == id {
                        doc.delete(&r.capsules, cid.as_str()).expect("del");
                    }
                }
            }
            SemOp::Undo => unreachable!(),
        }
        if who == Who::Arkin {
            self.undo_stack.push(undo);
        }
    }

    fn commit_and_outbox(&mut self, who: Who) {
        let doc = &mut self.docs[who.idx()];
        if doc.commit().is_some() {
            let ch = doc.get_last_local_change().expect("change");
            let bytes = ch.raw_bytes().to_vec();
            self.wire.up.push(bytes.len());
            self.outbox[who.idx()].push(bytes);
        }
    }

    /// Native provenance: the actor of the op that wrote the value at `path`
    /// (e.g. `["series","macd","params","fast"]`), plus how many concurrent values exist.
    pub fn writer_of(&self, path: &[&str]) -> String {
        let d = &self.docs[0];
        let mut o = ROOT;
        for k in &path[..path.len() - 1] {
            match obj(d, &o, k) {
                Some(x) => o = x,
                None => return "-".into(),
            }
        }
        let all = d.get_all(&o, *path.last().expect("path")).unwrap_or_default();
        let names: Vec<&str> = all
            .iter()
            .map(|(_, id)| match id {
                ExId::Id(_, a, _) => who_of_actor(a),
                ExId::Root => "root",
            })
            .collect();
        names.join("|")
    }
}

impl Model for AmModel {
    const NAME: &'static str = "B-automerge";

    fn new(doc: &qp_schema::Document) -> Self {
        let mut base = build_initial(doc);
        let bytes = base.save();
        let roots = Roots {
            series: obj(&base, &ROOT, "series").expect("r"),
            layers: obj(&base, &ROOT, "layers").expect("r"),
            facts: obj(&base, &ROOT, "facts").expect("r"),
            findings: obj(&base, &ROOT, "findings").expect("r"),
            capsules: obj(&base, &ROOT, "capsules").expect("r"),
            intents: obj(&base, &ROOT, "intents").expect("r"),
            ledger: obj(&base, &ROOT, "ledger").expect("r"),
        };
        let docs = ALL
            .iter()
            .map(|w| AutoCommit::load(&bytes).expect("load").with_actor(actor(*w)))
            .collect();
        AmModel {
            docs,
            roots,
            outbox: [vec![], vec![], vec![]],
            undo_stack: Vec::new(),
            wire: Wire::default(),
            undo: UndoStats::default(),
            extra: BTreeMap::new(),
        }
    }

    fn view(&self, who: Who) -> Snap {
        snap_of(&self.docs[who.idx()], &self.roots)
    }

    fn write(&mut self, who: Who, _it: &Intent, op: &SemOp, _online: bool, view: &Snap) -> Exp {
        if matches!(op, SemOp::Undo) {
            self.undo.attempted += 1;
            match self.do_undo(who.idx()) {
                "applied" => self.undo.applied += 1,
                "conflict" => {
                    // Known locally: the app refuses before writing anything.
                    self.undo.refused_conflict += 1;
                    return Exp::Refused(vec![crate::common::Code::UndoConflict]);
                }
                _ => {
                    self.undo.nothing_or_unsupported += 1;
                    return Exp::Refused(vec![crate::common::Code::Unsupported]);
                }
            }
        } else {
            self.apply_op(who, op, view);
        }
        self.commit_and_outbox(who);
        Exp::Applied
    }

    fn end_round(&mut self, online: [bool; 3]) {
        for w in ALL {
            if !online[w.idx()] {
                continue;
            }
            let out = std::mem::take(&mut self.outbox[w.idx()]);
            for bytes in &out {
                for r in ALL {
                    if r != w && online[r.idx()] {
                        self.docs[r.idx()].load_incremental(bytes).expect("load");
                        self.wire.down.push(bytes.len());
                    }
                }
            }
        }
    }

    fn reconnect(&mut self, who: Who) -> Reconnect {
        let mut rc = Reconnect {
            queued: self.outbox[who.idx()].len(),
            ..Reconnect::default()
        };
        // The sync protocol carries the offline changes; drop the per-change outbox.
        self.outbox[who.idx()].clear();
        let (up, down, rt) = self.sync_pair(who.idx(), Who::Arkin.idx());
        rc.bytes_up = up;
        rc.bytes_down = down;
        rc.round_trips = rt;
        rc.applied = rc.queued;
        rc.details.push("all changes merged silently; no errors".into());
        // Relay to the other online replica.
        let (a, b, _) = self.sync_pair(Who::Arkin.idx(), Who::Scout.idx());
        self.wire.down.push(a + b);
        *self.extra.entry("relay_sync_bytes".into()).or_default() += a + b;
        rc
    }

    fn snaps(&self) -> Vec<Snap> {
        self.docs.iter().map(|d| snap_of(d, &self.roots)).collect()
    }

    fn history_bytes(&mut self) -> Vec<u8> {
        self.docs[0].save()
    }

    fn state_bytes(&mut self) -> usize {
        // Automerge has no state-only export; report the JSON of the semantic state.
        serde_json::to_vec(&snap_of(&self.docs[0], &self.roots))
            .expect("json")
            .len()
    }

    fn wire(&self) -> &Wire {
        &self.wire
    }

    fn undo_stats(&self) -> &UndoStats {
        &self.undo
    }

    fn extra(&self) -> BTreeMap<String, usize> {
        let mut e = self.extra.clone();
        // Unresolved register conflicts still visible at the end (multi-value).
        let d = &self.docs[0];
        let mut multi = 0;
        for id in d.keys(&self.roots.series) {
            if d.get_all(&self.roots.series, id.as_str()).map(|v| v.len()).unwrap_or(0) > 1 {
                multi += 1;
            }
            if let Some(o) = obj(d, &self.roots.series, &id) {
                if let Some(p) = obj(d, &o, "params") {
                    for k in d.keys(&p) {
                        if d.get_all(&p, k.as_str()).map(|v| v.len()).unwrap_or(0) > 1 {
                            multi += 1;
                        }
                    }
                }
            }
        }
        e.insert("visible_multi_value_conflicts_in_series".into(), multi);
        e
    }
}
