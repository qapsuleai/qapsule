//! (R5/M21) The shell core's motion state: the reader's motion design (`?motion=`, a
//! presentation setting like the feed order), the reduced-motion flag the page forwards
//! (`prefers-reduced-motion` or the settings page), and the face changes the feed brought
//! (`qp_grammar::motion`).
//!
//! When the feed is replaced the core compares each capsule with its previous row
//! ([`qp_grammar::motion::classify`]). Every change is recorded as the capsule's latest
//! change (`recent`, both modes: the same facts). With motion, the change's geometry
//! (glyph boxes before and after, the texts that differ) is kept for the page's animation
//! ([`ShellCore::take_face_motion`], [`ShellCore::face_change_frame`]); under reduced
//! motion nothing moves and the face carries the change's lasting label instead
//! ([`qp_grammar::motion::stamp`], applied in [`ShellCore::face`]). A label stays until the
//! reader opens that capsule or marks "I looked now", or a newer change replaces it.

use std::collections::BTreeMap;

use qp_grammar::motion::{self, ChangeKind, FaceChange};
use qp_grammar::transitions::Source;
use qp_schema::protocol::CapsuleItem;
use qp_schema::scene::SceneDesc;
use serde_json::{json, Value};

use super::ShellCore;

/// The motion part of a shell core.
#[derive(Debug, Default)]
pub struct MotionState {
    /// The motion design chosen (`None`: the default, `r4`).
    pub design: Option<String>,
    /// Reduced motion (the page forwards the reader's setting).
    pub reduced: bool,
    /// Each capsule's latest change (both modes; cleared on open and "I looked now").
    pub recent: BTreeMap<String, ChangeKind>,
    /// The changes' geometry, the face scene before the change and the animation's start
    /// (shell ms), by capsule.
    pub moves: BTreeMap<String, (FaceChange, Option<SceneDesc>, Option<f64>)>,
    /// The face scene last handed out for painting, by capsule (kept only while a design
    /// marks changes and motion is on).
    pub last: BTreeMap<String, SceneDesc>,
    /// [`Self::last`] as it was when the latest data frame arrived: the face as the reader
    /// saw it before that bar (a face repaints with the bar before the feed tells what
    /// changed, so the scene before the change is the one from before the data).
    pub stable: BTreeMap<String, SceneDesc>,
    /// Capsules whose change the page has not started animating yet.
    pub queue: Vec<String>,
}

impl ShellCore {
    /// The connection's motion design.
    #[must_use]
    pub fn motion_design(&self) -> &'static motion::MotionDesign {
        motion::by_name(self.mo.design.as_deref())
    }

    /// Whether the faces' own changes are marked (with motion, or with labels).
    fn marks(&self) -> bool {
        self.motion_design().marks
    }

    /// (R5/M21) Chooses motion design `name` (a known name; others are ignored). Returns
    /// the transition design it opens with, for the `style` gesture.
    pub fn set_motion(&mut self, name: &str) -> Option<&'static str> {
        if !motion::is_motion(name) {
            return None;
        }
        self.mo.design = (name != motion::DEFAULT_MOTION).then(|| name.to_owned());
        if !self.marks() {
            self.mo.recent.clear();
            self.mo.moves.clear();
            self.mo.queue.clear();
            self.mo.last.clear();
            self.mo.stable.clear();
        }
        self.pending.insert("feed".to_owned(), json!(true));
        Some(motion::by_name(Some(name)).transition)
    }

    /// (R5/M21) The reader's reduced-motion setting (the page forwards
    /// `prefers-reduced-motion` or its own setting). Faces repaint (labels come or go).
    pub fn set_reduced_motion(&mut self, on: bool) {
        if self.mo.reduced == on {
            return;
        }
        self.mo.reduced = on;
        self.mo.queue.clear();
        self.mo.moves.clear();
        self.mo.last.clear();
        self.mo.stable.clear();
        self.pending.insert("feed".to_owned(), json!(true));
    }

    /// Whether the faces handed out are remembered (a marking design, motion on).
    fn keeps_faces(&self) -> bool {
        self.marks() && !self.mo.reduced
    }

    /// A face scene was handed out for painting (`face_changed`).
    pub(crate) fn face_shown_now(&mut self, id: &str, scene: &SceneDesc) {
        if self.keeps_faces() {
            self.mo.last.insert(id.to_owned(), scene.clone());
        }
    }

    /// A data frame arrives: the faces as shown now are the faces "before" its changes.
    pub(crate) fn data_arrives(&mut self) {
        if self.keeps_faces() {
            self.mo.stable.clone_from(&self.mo.last);
        }
    }

    /// Whether reduced motion is on.
    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.mo.reduced
    }

    /// The face scene of `id` with its change label under reduced motion.
    pub(crate) fn stamped(&self, id: &str, mut scene: SceneDesc) -> SceneDesc {
        if self.mo.reduced && self.marks() {
            if let Some(k) = self.mo.recent.get(id) {
                motion::stamp(
                    &mut scene,
                    *k,
                    self.lang,
                    qp_session::viewer::look_of(&self.look),
                );
            }
        }
        scene
    }

    /// Before the feed is replaced by `new`: the changes against the current feed, with
    /// the face scenes before them (as shown before the latest data frame).
    pub(crate) fn changes_before(
        &mut self,
        new: &[CapsuleItem],
    ) -> Vec<(String, ChangeKind, Option<SceneDesc>)> {
        if !self.marks() || !self.feed_known {
            return Vec::new();
        }
        let newest = self.feed.iter().map(|i| i.t).max();
        let mut out = Vec::new();
        for it in new {
            let old = self.feed.iter().find(|o| o.id == it.id);
            if let Some(k) = motion::classify(old, it, newest) {
                let before = if self.mo.reduced || k == ChangeKind::Born {
                    None
                } else {
                    self.mo
                        .stable
                        .get(&it.id)
                        .or_else(|| self.mo.last.get(&it.id))
                        .cloned()
                };
                out.push((it.id.clone(), k, before));
            }
        }
        out
    }

    /// After the feed was replaced: records `changes` (from [`Self::changes_before`]).
    pub(crate) fn changes_after(&mut self, changes: Vec<(String, ChangeKind, Option<SceneDesc>)>) {
        let feed = &self.feed;
        self.mo
            .recent
            .retain(|id, _| feed.iter().any(|i| &i.id == id));
        self.mo
            .moves
            .retain(|id, _| feed.iter().any(|i| &i.id == id));
        self.mo
            .last
            .retain(|id, _| feed.iter().any(|i| &i.id == id));
        self.mo
            .stable
            .retain(|id, _| feed.iter().any(|i| &i.id == id));
        for (id, k, before) in changes {
            self.mo.recent.insert(id.clone(), k);
            if self.mo.reduced {
                continue;
            }
            let Some(after) = self.face_plain(&id) else {
                continue;
            };
            let ch = motion::face_change(k, before.as_ref(), &after);
            self.mo.moves.insert(id.clone(), (ch, before, None));
            if !self.mo.queue.contains(&id) {
                self.mo.queue.push(id);
            }
        }
    }

    /// The reader opened capsule `id` or looked at everything (`None`): its labels go.
    pub(crate) fn changes_seen(&mut self, id: Option<&str>) {
        match id {
            Some(id) => {
                self.mo.recent.remove(id);
            }
            None => self.mo.recent.clear(),
        }
        self.pending.insert("feed".to_owned(), json!(true));
    }

    /// (R5/M21) The faces whose change the page should animate now (JSON array of ids;
    /// emptied). Empty under reduced motion and for a design that marks nothing.
    pub fn take_face_motion(&mut self) -> Value {
        json!(std::mem::take(&mut self.mo.queue))
    }

    /// (R5/M21) The latest changes as facts `[{id, kind, label}]` (both modes alike: what a
    /// change animation and its label carry; the information-equality check reads it).
    #[must_use]
    pub fn face_changes_json(&self) -> Value {
        Value::Array(
            self.mo
                .recent
                .iter()
                .map(|(id, k)| json!({"id": id, "kind": k.name(), "label": motion::label(*k, self.lang)}))
                .collect(),
        )
    }

    /// (R5/M21) Progress of capsule `id`'s change animation at shell time `t` (ms); the first
    /// call starts its clock. 1 when there is none or under reduced motion.
    pub fn face_change_progress(&mut self, id: &str, t: f64) -> f64 {
        let look = qp_session::viewer::look_of(&self.look);
        if self.mo.reduced {
            return 1.0;
        }
        match self.mo.moves.get_mut(id) {
            Some((_, _, t0)) => {
                let s = *t0.get_or_insert(t);
                motion::change_progress(look, s, t)
            }
            None => 1.0,
        }
    }

    /// (R5/M21) Capsule `id`'s change frame at progress `p`, its face at page (`left`,
    /// `top`): `{done, scene_opacity, elements:[{id, source, src, style}]}` as
    /// [`ShellCore::transition_frame`] (`source` `before`: the page's copy of the face
    /// before the change; `scene_opacity`: the face's own opacity). `null` without a change.
    #[must_use]
    pub fn face_change_frame(&self, id: &str, p: f64, left: f64, top: f64) -> Value {
        let Some((ch, _, _)) = self.mo.moves.get(id) else {
            return Value::Null;
        };
        let p = if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let f = motion::change_frame(ch, left, top, p);
        let k = i32::try_from(self.scale).unwrap_or(1);
        let els: Vec<Value> = f
            .elements
            .iter()
            .map(|e| {
                let r = e.src;
                json!({"id": e.id,
                    "source": if e.source == Source::Before { "before" } else { "after" },
                    "src": [r.left * k, r.top * k, (r.right - r.left).max(1) * k,
                            (r.bottom - r.top).max(1) * k],
                    "style": {"left": format!("{}px", e.rect[0]), "top": format!("{}px", e.rect[1]),
                              "width": format!("{}px", e.rect[2]), "height": format!("{}px", e.rect[3]),
                              "opacity": format!("{}", e.opacity)}})
            })
            .collect();
        json!({"done": p >= 1.0, "kind": ch.kind.name(),
               "scene_opacity": format!("{}", f.scene_opacity.clamp(0.0, 1.0)), "elements": els})
    }

    /// (R5/M21) Pixels (RGBA, at the device scale) of capsule `id`'s face before its
    /// change (the scene the core kept; the page's canvas already shows the face after
    /// the bar), or empty when there is none.
    #[must_use]
    pub fn face_before_rgba(&self, id: &str) -> Vec<u8> {
        match self.mo.moves.get(id) {
            Some((_, Some(sc), _)) => self.pixels(sc),
            _ => Vec::new(),
        }
    }
}
