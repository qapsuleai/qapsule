//! Model C: Loro 1.17. One document per replica; peer id = principal. Items are
//! *mergeable* maps keyed by id (Loro's recommended idiom for "this key is this entity"),
//! layers and ledger are lists of maps. Updates are exported per commit; a returning
//! replica exchanges version vectors and updates. Undo is Loro's native `UndoManager`
//! (local-peer only, transformed over remote changes).

use std::collections::BTreeMap;

use loro::{ExportMode, LoroDoc, LoroList, LoroMap, LoroValue, UndoManager, VersionVector};

use crate::common::{
    Intent, SCapsule, SFinding, SLayer, SLedger, SSeries, SemOp, Snap, Who, ALL, FACT,
};
use crate::sim::{Exp, Model, Reconnect, UndoStats, Wire};

pub fn peer(who: Who) -> u64 {
    who.idx() as u64 + 1
}

pub fn who_of_peer(p: u64) -> &'static str {
    match p {
        1 => "arkin",
        2 => "scout",
        3 => "editor",
        _ => "init",
    }
}

pub struct LoroModel {
    docs: Vec<LoroDoc>,
    undo_mgr: UndoManager,
    /// Mirror of arkin's undo stack (SemOps) for clobber accounting.
    arkin_ops: Vec<SemOp>,
    sent_vv: [VersionVector; 3],
    outbox: [Vec<Vec<u8>>; 3],
    wire: Wire,
    undo: UndoStats,
    extra: BTreeMap<String, usize>,
}

fn st(m: &LoroMap, k: &str) -> String {
    match m.get(k) {
        Some(loro::ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => String::new(),
    }
}

fn vstr(v: Option<&LoroValue>) -> String {
    match v {
        Some(LoroValue::String(s)) => s.to_string(),
        _ => String::new(),
    }
}

fn vnum(v: Option<&LoroValue>) -> f64 {
    match v {
        Some(LoroValue::Double(x)) => *x,
        Some(LoroValue::I64(x)) => *x as f64,
        _ => f64::NAN,
    }
}

fn as_map(v: &LoroValue) -> Option<&loro::LoroMapValue> {
    match v {
        LoroValue::Map(m) => Some(m),
        _ => None,
    }
}

pub fn snap_of(doc: &LoroDoc) -> Snap {
    let mut sn = Snap::default();
    let series = doc.get_map("series").get_deep_value();
    if let Some(m) = as_map(&series) {
        for (id, v) in m.iter() {
            let Some(o) = as_map(v) else { continue };
            let mut params = BTreeMap::new();
            if let Some(p) = o.get("params").and_then(as_map) {
                for (k, x) in p.iter() {
                    params.insert(k.clone(), vnum(Some(x)));
                }
            }
            let inputs = vstr(o.get("inputs"));
            sn.series.insert(
                id.clone(),
                SSeries {
                    kind: vstr(o.get("kind")),
                    study: vstr(o.get("study")),
                    inputs: if inputs.is_empty() {
                        vec![]
                    } else {
                        inputs.split(',').map(String::from).collect()
                    },
                    params,
                },
            );
        }
    }
    if let LoroValue::List(l) = doc.get_list("layers").get_deep_value() {
        for v in l.iter() {
            let Some(o) = as_map(v) else { continue };
            let ser = vstr(o.get("series"));
            sn.layers.push(SLayer {
                mark: vstr(o.get("mark")),
                series: if ser.is_empty() { None } else { Some(ser) },
                panel: vnum(o.get("panel")) as u32,
            });
        }
    }
    if let Some(m) = as_map(&doc.get_map("facts").get_deep_value()) {
        for (id, v) in m.iter() {
            if let Some(o) = as_map(v) {
                sn.facts.insert(id.clone(), vstr(o.get("series")));
            }
        }
    }
    if let Some(m) = as_map(&doc.get_map("findings").get_deep_value()) {
        for (id, v) in m.iter() {
            if let Some(o) = as_map(v) {
                sn.findings.insert(
                    id.clone(),
                    SFinding {
                        claim: vstr(o.get("claim")),
                        confidence: vnum(o.get("confidence")),
                        inval_series: vstr(o.get("inval_series")),
                        facts: vec![vstr(o.get("facts"))],
                    },
                );
            }
        }
    }
    if let Some(m) = as_map(&doc.get_map("capsules").get_deep_value()) {
        for (id, v) in m.iter() {
            if let Some(o) = as_map(v) {
                sn.capsules.insert(
                    id.clone(),
                    SCapsule {
                        finding: vstr(o.get("finding")),
                        state: vstr(o.get("state")),
                    },
                );
            }
        }
    }
    if let Some(m) = as_map(&doc.get_map("intents").get_deep_value()) {
        for (id, v) in m.iter() {
            if let Some(o) = as_map(v) {
                let t = vstr(o.get("trigger_series"));
                sn.intents
                    .insert(id.clone(), if t.is_empty() { None } else { Some(t) });
            }
        }
    }
    if let LoroValue::List(l) = doc.get_list("ledger").get_deep_value() {
        for v in l.iter() {
            if let Some(o) = as_map(v) {
                sn.ledger.push(SLedger {
                    finding: vstr(o.get("finding")),
                    state: vstr(o.get("state")),
                    decided_claim: vstr(o.get("decided_claim")),
                });
            }
        }
    }
    sn
}

fn put_layer(layers: &LoroList, i: usize, l: &SLayer) {
    let m = layers.insert_container(i, LoroMap::new()).expect("ins");
    m.insert("mark", l.mark.as_str()).expect("p");
    if let Some(s) = &l.series {
        m.insert("series", s.as_str()).expect("p");
    }
    m.insert("panel", i64::from(l.panel)).expect("p");
}

fn build_initial(doc: &qp_schema::Document) -> LoroDoc {
    let d = LoroDoc::new();
    d.set_peer_id(9).expect("peer");
    let series = d.get_map("series");
    for x in &doc.series {
        let o = series.ensure_mergeable_map(&x.id).expect("o");
        match &x.source {
            qp_schema::SeriesSource::Study {
                study,
                inputs,
                params,
                ..
            } => {
                o.insert("kind", "study").expect("p");
                o.insert("study", study.as_str()).expect("p");
                o.insert("inputs", inputs.join(",")).expect("p");
                let p = o.ensure_mergeable_map("params").expect("o");
                for (k, v) in params {
                    p.insert(k, *v).expect("p");
                }
            }
            _ => {
                o.insert("kind", "inline").expect("p");
                let bars = serde_json::to_vec(&x.data).expect("bars");
                o.insert("bars", bars).expect("p");
            }
        }
    }
    let layers = d.get_list("layers");
    let main = doc.views.iter().find(|v| v.id == "main").expect("main");
    for (i, l) in main.layers.iter().enumerate() {
        let mark = serde_json::to_value(l.mark).expect("m");
        put_layer(
            &layers,
            i,
            &SLayer {
                mark: mark.as_str().unwrap_or("").into(),
                series: l.series.clone(),
                panel: l.panel,
            },
        );
    }
    let facts = d.get_map("facts");
    for f in &doc.facts {
        let o = facts.ensure_mergeable_map(&f.id).expect("o");
        o.insert("series", f.series.as_str()).expect("p");
        o.insert("text", f.text.as_str()).expect("p");
    }
    let findings = d.get_map("findings");
    for f in &doc.findings {
        let o = findings.ensure_mergeable_map(&f.id).expect("o");
        o.insert("claim", f.claim.as_str()).expect("p");
        o.insert("confidence", f.confidence).expect("p");
        if let qp_schema::Condition::Threshold { series, .. } = &f.invalidation {
            o.insert("inval_series", series.as_str()).expect("p");
        }
        o.insert("facts", FACT).expect("p");
        o.insert("author", "scout").expect("p");
    }
    let caps = d.get_map("capsules");
    for c in &doc.capsules {
        let o = caps.ensure_mergeable_map(&c.id).expect("o");
        o.insert("finding", c.finding.as_str()).expect("p");
        o.insert("state", "live").expect("p");
        o.insert("headline", c.headline.as_str()).expect("p");
    }
    d.get_map("meta")
        .insert("title", doc.title.as_deref().unwrap_or(""))
        .expect("p");
    d.get_list("ledger");
    d.commit();
    d
}

fn get_map(m: &LoroMap, k: &str) -> Option<LoroMap> {
    match m.get(k) {
        Some(loro::ValueOrContainer::Container(loro::Container::Map(x))) => Some(x),
        _ => None,
    }
}

impl LoroModel {
    fn apply_op(&mut self, who: Who, op: &SemOp, view: &Snap) {
        let doc = &self.docs[who.idx()];
        let layers = doc.get_list("layers");
        match op {
            SemOp::AddStudy {
                id,
                study,
                params,
                show_panel,
            } => {
                let o = doc.get_map("series").ensure_mergeable_map(id).expect("o");
                o.insert("kind", "study").expect("p");
                o.insert("study", study.as_str()).expect("p");
                o.insert("inputs", "px").expect("p");
                o.insert("author", who.name()).expect("p");
                let p = o.ensure_mergeable_map("params").expect("o");
                for (k, v) in params {
                    p.insert(k, *v).expect("p");
                }
                if let Some(panel) = show_panel {
                    put_layer(
                        &layers,
                        layers.len(),
                        &SLayer {
                            mark: "line".into(),
                            series: Some(id.clone()),
                            panel: *panel,
                        },
                    );
                }
            }
            SemOp::SetParams { series, params } => {
                let o = get_map(&doc.get_map("series"), series).expect("series");
                let p = o.ensure_mergeable_map("params").expect("params");
                for (k, v) in params {
                    p.insert(k, *v).expect("p");
                }
            }
            SemOp::AddLayer {
                series,
                panel,
                index,
                ..
            } => put_layer(
                &layers,
                *index as usize,
                &SLayer {
                    mark: "line".into(),
                    series: Some(series.clone()),
                    panel: *panel,
                },
            ),
            SemOp::RemoveLayer { index, .. } => layers.delete(*index as usize, 1).expect("del"),
            SemOp::RemoveSeries { id } => {
                doc.get_map("series").delete(id).expect("del");
                for (i, l) in view.layers.iter().enumerate().rev() {
                    if l.series.as_ref() == Some(id) {
                        layers.delete(i, 1).expect("del");
                    }
                }
            }
            SemOp::AddFinding {
                id,
                claim,
                confidence,
                inval_series,
            } => {
                let o = doc.get_map("findings").ensure_mergeable_map(id).expect("o");
                o.insert("claim", claim.as_str()).expect("p");
                o.insert("confidence", *confidence).expect("p");
                o.insert("inval_series", inval_series.as_str()).expect("p");
                o.insert("facts", FACT).expect("p");
                o.insert("author", who.name()).expect("p");
                let c = doc
                    .get_map("capsules")
                    .ensure_mergeable_map(&format!("cap-{id}"))
                    .expect("o");
                c.insert("finding", id.as_str()).expect("p");
                c.insert("state", "live").expect("p");
                c.insert("headline", format!("{claim} (kapsül)")).expect("p");
                c.insert("author", who.name()).expect("p");
            }
            SemOp::UpdateFinding {
                finding,
                claim,
                confidence,
            } => {
                let o = get_map(&doc.get_map("findings"), finding).expect("finding");
                if let Some(c) = claim {
                    o.insert("claim", c.as_str()).expect("p");
                }
                if let Some(c) = confidence {
                    o.insert("confidence", *c).expect("p");
                }
            }
            SemOp::Decide {
                finding,
                approve,
                note,
            } => {
                let (cid, _) = view.capsule_of(finding).expect("capsule");
                let c = get_map(&doc.get_map("capsules"), cid).expect("cap");
                let state = if *approve { "approved" } else { "rejected" };
                c.insert("state", state).expect("p");
                let ledger = doc.get_list("ledger");
                let le = ledger.insert_container(ledger.len(), LoroMap::new()).expect("o");
                le.insert("finding", finding.as_str()).expect("p");
                le.insert("state", state).expect("p");
                le.insert("note", note.as_str()).expect("p");
                le.insert("decided_claim", view.findings[finding].claim.as_str())
                    .expect("p");
                le.insert("recorded_by", who.name()).expect("p");
            }
            SemOp::AddIntent {
                id,
                text,
                trigger_series,
            } => {
                let o = doc.get_map("intents").ensure_mergeable_map(id).expect("o");
                o.insert("text", text.as_str()).expect("p");
                o.insert("trigger_series", trigger_series.as_str()).expect("p");
                o.insert("author", who.name()).expect("p");
            }
            SemOp::RemoveFinding { id } => {
                doc.get_map("findings").delete(id).expect("del");
                for (cid, c) in &view.capsules {
                    if &c.finding == id {
                        doc.get_map("capsules").delete(cid).expect("del");
                    }
                }
            }
            SemOp::Undo => unreachable!(),
        }
    }

    /// Register values (and their last editors) an undo of `op` could change.
    fn registers(&self, op: &SemOp) -> Vec<(String, u64)> {
        let doc = &self.docs[Who::Arkin.idx()];
        let mut v = Vec::new();
        match op {
            SemOp::SetParams { series, params } => {
                if let Some(o) = get_map(&doc.get_map("series"), series) {
                    if let Some(p) = get_map(&o, "params") {
                        for k in params.keys() {
                            let val = format!("{:?}", p.get(k).map(|x| x.get_deep_value()));
                            v.push((val, p.get_last_editor(k).unwrap_or(0)));
                        }
                    }
                }
            }
            SemOp::Decide { finding, .. } => {
                let caps = doc.get_map("capsules");
                for k in caps.keys() {
                    if let Some(c) = get_map(&caps, &k) {
                        if st(&c, "finding") == *finding {
                            v.push((st(&c, "state"), c.get_last_editor("state").unwrap_or(0)));
                        }
                    }
                }
            }
            _ => {}
        }
        v
    }

    fn commit_and_outbox(&mut self, who: Who) {
        let i = who.idx();
        self.docs[i].commit();
        let bytes = self.docs[i]
            .export(ExportMode::updates(&self.sent_vv[i]))
            .expect("export");
        self.sent_vv[i] = self.docs[i].oplog_vv();
        self.wire.up.push(bytes.len());
        self.outbox[i].push(bytes);
    }

    /// Native provenance: the peer that last wrote the map entry at `path`.
    pub fn writer_of(&self, path: &[&str]) -> String {
        let d = &self.docs[0];
        let mut m = d.get_map(path[0]);
        for k in &path[1..path.len() - 1] {
            match get_map(&m, k) {
                Some(x) => m = x,
                None => return "-".into(),
            }
        }
        m.get_last_editor(path.last().expect("path"))
            .map(who_of_peer)
            .unwrap_or("-")
            .into()
    }
}

impl Model for LoroModel {
    const NAME: &'static str = "C-loro";

    fn new(doc: &qp_schema::Document) -> Self {
        let base = build_initial(doc);
        let snap = base.export(ExportMode::Snapshot).expect("snap");
        let docs: Vec<LoroDoc> = ALL
            .iter()
            .map(|w| {
                let d = LoroDoc::new();
                d.import(&snap).expect("import");
                d.set_peer_id(peer(*w)).expect("peer");
                d
            })
            .collect();
        // LORO_NO_UNDO=1 measures the cost of the UndoManager (it is bound to arkin's doc).
        let undo_doc = if std::env::var("LORO_NO_UNDO").is_ok() {
            LoroDoc::new()
        } else {
            docs[0].clone()
        };
        let mut undo_mgr = UndoManager::new(&undo_doc);
        undo_mgr.set_max_undo_steps(1_000_000);
        let sent_vv = [docs[0].oplog_vv(), docs[1].oplog_vv(), docs[2].oplog_vv()];
        LoroModel {
            docs,
            undo_mgr,
            arkin_ops: Vec::new(),
            sent_vv,
            outbox: [vec![], vec![], vec![]],
            wire: Wire::default(),
            undo: UndoStats::default(),
            extra: BTreeMap::new(),
        }
    }

    fn view(&self, who: Who) -> Snap {
        snap_of(&self.docs[who.idx()])
    }

    fn write(&mut self, who: Who, _it: &Intent, op: &SemOp, _online: bool, view: &Snap) -> Exp {
        if matches!(op, SemOp::Undo) {
            self.undo.attempted += 1;
            let top = self.arkin_ops.last().cloned();
            let before = top.as_ref().map(|t| self.registers(t)).unwrap_or_default();
            match self.undo_mgr.undo() {
                Ok(true) => {
                    self.undo.applied += 1;
                    self.arkin_ops.pop();
                    if let Some(t) = &top {
                        let after = self.registers(t);
                        for ((vb, ed), (va, _)) in before.iter().zip(&after) {
                            if *ed != peer(Who::Arkin) && vb != va {
                                self.undo.clobbered_others += 1;
                            }
                        }
                    }
                }
                Ok(false) => {
                    self.arkin_ops.pop();
                    self.undo.nothing_or_unsupported += 1
                }
                Err(_) => self.undo.refused_conflict += 1,
            }
        } else {
            self.apply_op(who, op, view);
            if who == Who::Arkin {
                self.arkin_ops.push(op.clone());
            }
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
                        self.docs[r.idx()].import(bytes).expect("import");
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
        self.outbox[who.idx()].clear();
        let e = who.idx();
        let a = Who::Arkin.idx();
        let evv = self.docs[e].oplog_vv();
        let avv = self.docs[a].oplog_vv();
        rc.bytes_up += evv.encode().len();
        rc.bytes_down += avv.encode().len();
        let to_e = self.docs[a].export(ExportMode::updates(&evv)).expect("x");
        let to_a = self.docs[e].export(ExportMode::updates(&avv)).expect("x");
        rc.bytes_up += to_a.len();
        rc.bytes_down += to_e.len();
        self.docs[e].import(&to_e).expect("i");
        self.docs[a].import(&to_a).expect("i");
        self.sent_vv[e] = self.docs[e].oplog_vv();
        rc.round_trips = 2;
        rc.applied = rc.queued;
        rc.details.push("all changes merged silently; no errors".into());
        // Relay to scout.
        let s = Who::Scout.idx();
        let svv = self.docs[s].oplog_vv();
        let relay = self.docs[a].export(ExportMode::updates(&svv)).expect("x");
        self.docs[s].import(&relay).expect("i");
        self.wire.down.push(relay.len());
        *self.extra.entry("relay_sync_bytes".into()).or_default() += relay.len();
        rc
    }

    fn snaps(&self) -> Vec<Snap> {
        self.docs.iter().map(snap_of).collect()
    }

    fn history_bytes(&mut self) -> Vec<u8> {
        self.docs[0].export(ExportMode::Snapshot).expect("snap")
    }

    fn state_bytes(&mut self) -> usize {
        self.docs[0]
            .export(ExportMode::StateOnly(None))
            .expect("state")
            .len()
    }

    fn wire(&self) -> &Wire {
        &self.wire
    }

    fn undo_stats(&self) -> &UndoStats {
        &self.undo
    }

    fn extra(&self) -> BTreeMap<String, usize> {
        self.extra.clone()
    }
}
